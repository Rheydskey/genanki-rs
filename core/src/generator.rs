use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    io::Read,
    path::{Path, PathBuf},
};

use base64::{Engine, prelude::BASE64_STANDARD};
use comrak::{
    Arena, Options, create_formatter,
    html::{ChildRendering, dangerous_url},
    nodes::NodeValue,
    parse_document,
};
use gitpatch::Patch;
use percent_encoding::percent_decode_str;

use crate::{
    data::{Card, DeckOutput, Output},
    git::{Git, GitUpdate},
};

pub fn get_md_of_folder(path: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|dir_entry| dir_entry.file_type().map(|f| f.is_file()).unwrap_or(false))
        .map(|f| f.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect()
}

struct CurrentPath<'a> {
    project_path: &'a Path,
    file_path: &'a Path,
}

pub fn render_to_base64<'a>(paths: &'a CurrentPath<'a>, url: &str) -> Option<String> {
    let percent_decode = PathBuf::from(percent_decode_str(url).decode_utf8().ok()?.into_owned());
    let joined_path = if percent_decode.is_absolute() {
        paths
            .project_path
            .join(percent_decode.strip_prefix("/").unwrap())
    } else {
        paths.file_path.join(percent_decode)
    };

    let mut p = std::fs::File::open(&joined_path)
        .inspect_err(|f| eprintln!("Warn on {joined_path:?}: {f}"))
        .ok()?;
    let mut vec = Vec::new();
    p.read_to_end(&mut vec).unwrap();
    let mimetype = infer::get(&vec).unwrap();
    if !matches!(mimetype.matcher_type(), infer::MatcherType::Image) {
        return None;
    }
    let a = BASE64_STANDARD.encode(&vec);
    Some(format!("{};base64,{}", mimetype.mime_type(), a))
}

create_formatter!(CustomMath<&'a CurrentPath<'a>>, {
    NodeValue::Math(ref node) => |context, entering| {
        let fence = if node.display_math {
            "$$"
        } else {
            "$"
        };

        if entering {
            context.write_str(fence)?;
            context.write_str(&node.literal)?;
        } else {
            context.write_str(fence)?;
        }
    },
    NodeValue::Image(ref nl) => |context, node, entering| {
        if entering {
            if context.options.render.figure_with_caption {
                context.write_str("<figure>")?;
            }
            context.write_str("<img")?;
            if context.options.render.sourcepos {
                let ast = node.data();
                if ast.sourcepos.start.line > 0 {
                    write!(context, " data-sourcepos=\"{}\"", ast.sourcepos)?;
                }
            }
            context.write_str(" src=\"")?;
            let url = &nl.url;
            if context.options.render.r#unsafe || !dangerous_url(url) {
                if let Some(base64) = render_to_base64(context.user, url) {
                    context.write_str(&base64)?;
                } else if let Some(rewriter) = &context.options.extension.image_url_rewriter {
                    context.escape_href(&rewriter.to_html(&nl.url))?;
                } else {
                    context.escape_href(url)?;
                }
            }
            context.write_str("\" alt=\"")?;
            return Ok(ChildRendering::Plain);
        } else {
            if !nl.title.is_empty() {
                context.write_str("\" title=\"")?;
                context.escape(&nl.title)?;
            }
            context.write_str("\" />")?;
            if context.options.render.figure_with_caption {
                if !nl.title.is_empty() {
                    context.write_str("<figcaption>")?;
                    context.escape(&nl.title)?;
                    context.write_str("</figcaption>")?;
                }
                context.write_str("</figure>")?;
            };
        }

        return Ok(ChildRendering::HTML);
    },
});

pub struct CardGenerator<'a>(String, &'a CurrentPath<'a>);

impl<'a> CardGenerator<'a> {
    pub const fn new(input: String, path: &'a CurrentPath<'a>) -> Self {
        Self(input, path)
    }

    fn to_html(&self, input: &str) -> anyhow::Result<String> {
        let options = Options {
            extension: comrak::options::Extension {
                math_dollars: true,
                math_code: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let arena = Arena::new();
        let document = parse_document(&arena, input, &options);
        let mut output = String::new();

        CustomMath::format_document(document, &options, &mut output, self.1)?;

        Ok(output.trim().to_string())
    }

    fn transform_to_html(&self, card: Card) -> anyhow::Result<Card> {
        let front = self.to_html(&card.front)?;
        let back = self.to_html(&card.back)?;
        Ok(Card {
            front,
            back,
            ..card
        })
    }

    fn generate_hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.0.trim().as_bytes());
        hasher.finalize().to_hex().as_str().to_string()
    }

    fn generate_extend(&self) -> anyhow::Result<Card> {
        let Some((front, back)) = self.0.split_once('%') else {
            return Err(anyhow::anyhow!("This card isn't extended"));
        };

        self.transform_to_html(Card {
            front: front.to_string(),
            back: back.to_string(),
            hash: self.generate_hash(),
        })
    }

    fn generate_basic(&self) -> anyhow::Result<Card> {
        let lines = self.0.lines().collect::<Vec<_>>();
        let front = lines[0].to_string();
        let back = lines[1..].join("\n");

        self.transform_to_html(Card {
            front,
            back,
            hash: self.generate_hash(),
        })
    }

    fn is_extends(&self) -> bool {
        self.0.lines().any(|f| f.trim_end() == "%")
    }

    pub fn generate(&self) -> anyhow::Result<Card> {
        if self.is_extends() {
            return self.generate_extend();
        }

        self.generate_basic()
    }
}

pub struct Generator<'a> {
    pub project_path: &'a Path,
}

impl<'a> Generator<'a> {
    fn skip_until_first_card(input: &str) -> &str {
        let mut offset = 0;
        for i in input.lines() {
            if i.starts_with("##") {
                break;
            }

            offset += i.len();
        }

        &input[offset..]
    }
    pub fn generate_card_from_input(&self, input: &str, path: &Path) -> Vec<Card> {
        Self::skip_until_first_card(input)
            .split("##")
            .inspect(|f| println!("{:?}", f))
            .filter(|f| !f.trim_end().is_empty())
            .map(|f| format!("##{}", f.trim_end()))
            .flat_map(|f| {
                CardGenerator::new(
                    f,
                    &CurrentPath {
                        project_path: &self.project_path,
                        file_path: path,
                    },
                )
                .generate()
            })
            .collect::<Vec<_>>()
    }
    pub fn generate_card_from_folder(&self) -> Vec<Card> {
        get_md_of_folder(self.project_path)
            .iter()
            .flat_map(|f| {
                let content = std::fs::read_to_string(f).unwrap();
                self.generate_card_from_input(&content, f.as_path())
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct Updater {
    git: Git,
}

impl Updater {
    pub fn new(repo: String) -> Self {
        let git = Git::new(repo);
        Self { git }
    }

    fn root_folder_of_patch(path: &str) -> String {
        path.split('/').nth(1).unwrap().to_string()
    }

    fn get_folder_of_patch(patch: &Patch) -> Vec<String> {
        let mut paths = Vec::new();
        if patch.old.path != "/dev/null" {
            paths.push(Self::root_folder_of_patch(&patch.old.path));
        }

        if patch.new.path != "/dev/null" {
            paths.push(Self::root_folder_of_patch(&patch.new.path));
        }

        paths
    }

    pub fn generate(&self) -> anyhow::Result<Output> {
        let GitUpdate {
            from_commit,
            to_commit,
        } = self.git.update()?;

        let Some(diff) = self.git.diff(&from_commit, &to_commit) else {
            return Err(anyhow::anyhow!(
                "Cannot get a diff between {from_commit} and {to_commit}"
            ));
        };
        let Ok(patchs) = gitpatch::Patch::from_multiple(&diff) else {
            return Err(anyhow::anyhow!("Output diff is not correct"));
        };

        let decks = patchs
            .iter()
            .flat_map(Self::get_folder_of_patch)
            .collect::<HashSet<String>>();

        self.git.checkout(&from_commit)?;

        let mut old_cards: HashMap<String, HashSet<String>> = HashMap::new();
        for i in &decks {
            let hashes: HashSet<String> = Generator {
                project_path: Path::new(&format!("./{}/{i}", self.git.repo)),
            }
            .generate_card_from_folder()
            .iter()
            .map(|f| f.hash.clone())
            .collect();

            old_cards.insert(i.clone(), hashes);
        }

        self.git.checkout(&to_commit)?;

        let mut decks_cards: HashMap<String, Vec<_>> = HashMap::new();
        for i in &decks {
            let cards = Generator {
                project_path: Path::new(&format!("./{}/{i}", self.git.repo)),
            }
            .generate_card_from_folder();

            decks_cards.insert(i.clone(), cards);
        }

        let mut output = Output::default();

        for (deck, cards) in &decks_cards {
            let new_cards_hash: HashSet<String> = cards.iter().map(|f| f.hash.clone()).collect();
            let Some(old_deck) = old_cards.get(deck) else {
                continue;
            };

            let deleted = old_deck
                .difference(&new_cards_hash)
                .cloned()
                .collect::<Vec<_>>();

            let added_cards = new_cards_hash.difference(old_deck).collect::<Vec<_>>();

            let added: Vec<_> = cards
                .iter()
                .filter(|f| added_cards.contains(&&f.hash))
                .cloned()
                .collect();

            output.insert(deck.clone(), DeckOutput { added, deleted });
        }

        Ok(output)
    }
}
