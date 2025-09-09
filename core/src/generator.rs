use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use gitpatch::Patch;
use markdown::Options;

use crate::data::{Card, DiffOutput, UpdateOutput};

pub fn get_md_of_folder(path: &Path) -> Vec<PathBuf> {
    let mut vec = Vec::new();
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };

    for i in entries {
        let Ok(entry) = i else {
            continue;
        };

        if !entry.file_type().unwrap().is_file() {
            continue;
        }
        let path = entry.path();
        let extension = path.extension().map(|f| f.to_str().unwrap());

        if !matches!(extension, Some("md")) {
            continue;
        }

        vec.push(path);
    }

    vec
}

pub struct CardGenerator(String);

impl CardGenerator {
    pub const fn new(input: String) -> Self {
        Self(input)
    }

    fn to_html(input: &str) -> anyhow::Result<String> {
        let option = &Options {
            parse: markdown::ParseOptions {
                constructs: markdown::Constructs {
                    character_escape: false,
                    character_reference: false,
                    ..Default::default()
                },
                ..markdown::ParseOptions::gfm()
            },
            compile: markdown::CompileOptions {
                allow_dangerous_html: true,
                allow_dangerous_protocol: true,
                ..markdown::CompileOptions::gfm()
            },
        };

        let html =
            markdown::to_html_with_options(input, option).map_err(|f| anyhow::anyhow!("{f}"))?;
        Ok(html_escape::decode_html_entities(&html).into_owned())
    }

    fn transform_to_html(card: Card) -> anyhow::Result<Card> {
        let front = Self::to_html(&card.front)?;
        let back = Self::to_html(&card.back)?;
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
        Self::transform_to_html(Card {
            front: front.to_string(),
            back: back.to_string(),
            hash: self.generate_hash(),
        })
    }

    fn generate_basic(&self) -> anyhow::Result<Card> {
        let lines = self.0.lines().collect::<Vec<_>>();
        let front = lines[0].to_string();
        let back = lines[1..].join("\n");

        Self::transform_to_html(Card {
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

        return &input[offset..];
    }
    pub fn generate_card_from_input(input: &str) -> Vec<Card> {
        Self::skip_until_first_card(input)
            .split("##")
            .filter(|f| !f.is_empty())
            .map(|f| format!("##{}", f.trim_end()))
            .map(|f| CardGenerator::new(f).generate())
            .flatten()
            .collect::<Vec<_>>()
    }
    pub fn generate_card_from_folder(path: &Path) -> Vec<Card> {
        get_md_of_folder(path)
            .iter()
            .flat_map(|f| {
                let content = std::fs::read_to_string(f).unwrap();
                Self::generate_card_from_input(&content)
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct Updater {
    repo: String,
}

impl Updater {
    fn update(&self) -> anyhow::Result<(String, String)> {
        let mut git = std::process::Command::new("git");
        git.args(["pull"]);
        git.current_dir(&self.repo);
        let a = git.output()?;
        // eprintln!("{a:?}");
        let output = String::from_utf8(a.stdout)?;
        let Some(first_line) = output.lines().nth(0) else {
            return Err(anyhow::anyhow!("No output lines"));
        };
        let Some(words) = first_line.split(' ').next_back() else {
            return Err(anyhow::anyhow!("Empty line"));
        };

        let mut commit_range = words.split("..");
        let Some(old) = commit_range.next() else {
            return Err(anyhow::anyhow!("No old commit"));
        };
        let Some(new) = commit_range.next() else {
            return Err(anyhow::anyhow!("No new commit"));
        };

        Ok((old.to_string(), new.to_string()))
    }
    fn get_diff(&self, commit1: &str, commit2: &str) -> Option<String> {
        let mut git = std::process::Command::new("git");
        git.args([
            "--no-pager",
            "diff",
            "-U1",
            "--no-color",
            &format!("{commit1}..{commit2}"),
        ]);
        git.current_dir(&self.repo);
        let a = git.output().ok()?;
        String::from_utf8(a.stdout).ok()
    }

    fn checkout(&self, commit: &str) {
        let mut git = std::process::Command::new("git");
        git.args(["--no-pager", "checkout", commit]);
        git.current_dir(&self.repo);
        git.status().unwrap();
    }

    pub const fn new(repo: String) -> Self {
        Self { repo }
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

    pub fn generate(&self) -> anyhow::Result<String> {
        let (old, new) = self.update()?;

        let Some(diff) = self.get_diff(&old, &new) else {
            return Err(anyhow::anyhow!("Cannot get a diff between {old} and {new}"));
        };
        let Ok(patchs) = gitpatch::Patch::from_multiple(&diff) else {
            return Err(anyhow::anyhow!("Output diff is not correct"));
        };

        let decks = patchs
            .iter()
            .flat_map(Self::get_folder_of_patch)
            .collect::<HashSet<String>>();

        self.checkout(&old);

        let mut old_cards: HashMap<String, HashSet<String>> = HashMap::new();
        for i in &decks {
            let hashes: HashSet<String> =
                Generator::generate_card_from_folder(Path::new(&format!("./{}/{i}", self.repo)))
                    .iter()
                    .map(|f| f.hash.clone())
                    .collect();

            old_cards.insert(i.clone(), hashes);
        }

        self.checkout(&new);

        let mut decks_cards: HashMap<String, Vec<_>> = HashMap::new();
        for i in &decks {
            let cards =
                Generator::generate_card_from_folder(Path::new(&format!("./{}/{i}", self.repo)));

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

            output
                .decks
                .insert(deck.clone(), DiffOutput { added, deleted });
        }

        Ok(serde_json::to_string(&output)?)
    }
}
