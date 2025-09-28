use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use gitpatch::Patch;
use markdown::Options;

use crate::{
    data::{Card, DiffOutput, UpdateOutput},
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

pub struct CardGenerator<'a>(String, &'a Path);

impl<'a> CardGenerator<'a> {
    pub const fn new(input: String, path: &'a Path) -> Self {
        Self(input, path)
    }

    fn to_html(&self, input: &str) -> anyhow::Result<String> {
        let option = &Options {
            parse: markdown::ParseOptions {
                constructs: markdown::Constructs {
                    character_escape: false,
                    character_reference: false,
                    math_flow: true,
                    math_text: true,
                    ..Default::default()
                },
                ..markdown::ParseOptions::gfm()
            },
            compile: markdown::CompileOptions {
                allow_dangerous_html: true,
                allow_dangerous_protocol: true,
                allow_any_img_src: true,
                base64_path: Some(self.1.to_path_buf().parent().unwrap().to_path_buf()),
                ..markdown::CompileOptions::gfm()
            },
        };
        let html =
            markdown::to_html_with_options(input, option).map_err(|f| anyhow::anyhow!("{f}"))?;
        Ok(html_escape::decode_html_entities(&html).into_owned())
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

pub struct Generator;

impl Generator {
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
    pub fn generate_card_from_input(input: &str, path: &Path) -> Vec<Card> {
        Self::skip_until_first_card(input)
            .split("##")
            .filter(|f| !f.is_empty())
            .map(|f| format!("##{}", f.trim_end()))
            .flat_map(|f| CardGenerator::new(f, &path).generate())
            .collect::<Vec<_>>()
    }
    pub fn generate_card_from_folder(path: &Path) -> Vec<Card> {
        get_md_of_folder(path)
            .iter()
            .flat_map(|f| {
                let content = std::fs::read_to_string(f).unwrap();
                Self::generate_card_from_input(&content, f.as_path())
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

    pub fn generate(&self) -> anyhow::Result<UpdateOutput> {
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
            let hashes: HashSet<String> = Generator::generate_card_from_folder(Path::new(
                &format!("./{}/{i}", self.git.repo),
            ))
            .iter()
            .map(|f| f.hash.clone())
            .collect();

            old_cards.insert(i.clone(), hashes);
        }

        self.git.checkout(&to_commit)?;

        let mut decks_cards: HashMap<String, Vec<_>> = HashMap::new();
        for i in &decks {
            let cards = Generator::generate_card_from_folder(Path::new(&format!(
                "./{}/{i}",
                self.git.repo
            )));

            decks_cards.insert(i.clone(), cards);
        }

        let mut output = UpdateOutput::default();

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

            output.insert(deck.clone(), DiffOutput { added, deleted });
        }

        Ok(output)
    }
}
