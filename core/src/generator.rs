use crate::{data::Card, markdown::CustomMath};
use comrak::{Arena, Options, parse_document};
use std::path::{Path, PathBuf};

pub struct CurrentPath<'a> {
    pub project_path: &'a Path,
    pub file_path: &'a Path,
}

pub struct CardGenerator<'a> {
    content: String,
    paths: &'a CurrentPath<'a>,
}

impl<'a> CardGenerator<'a> {
    pub const fn new(content: String, paths: &'a CurrentPath<'a>) -> Self {
        Self { content, paths }
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

        CustomMath::format_document(document, &options, &mut output, self.paths)?;

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
        hasher.update(self.content.trim().as_bytes());
        hasher.finalize().to_hex().as_str().to_string()
    }

    fn split_extended(&self) -> anyhow::Result<(String, String)> {
        let Some((front, back)) = self.content.split_once('%') else {
            return Err(anyhow::anyhow!("This card isn't extended"));
        };

        Ok((front.to_string(), back.to_string()))
    }

    fn split_basic(&self) -> anyhow::Result<(String, String)> {
        let lines = self.content.lines().collect::<Vec<_>>();
        let front = lines[0].to_string();
        let back = lines[1..].join("\n");

        Ok((front, back))
    }

    fn is_extends(&self) -> bool {
        self.content.lines().any(|f| f.trim_end() == "%")
    }

    pub fn generate(&self) -> anyhow::Result<Card> {
        let (front, back) = if self.is_extends() {
            self.split_extended()?
        } else {
            self.split_basic()?
        };

        self.transform_to_html(Card {
            front,
            back,
            hash: self.generate_hash(),
        })
    }
}

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

pub struct Generator<'a> {
    pub subproject_path: &'a Path,
}

impl Generator<'_> {
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
            .filter(|f| !f.trim_end().is_empty())
            .map(|f| format!("##{}", f.trim_end()))
            .flat_map(|f| {
                CardGenerator::new(
                    f,
                    &CurrentPath {
                        project_path: self.subproject_path,
                        file_path: path,
                    },
                )
                .generate()
            })
            .collect::<Vec<_>>()
    }
    pub fn generate_card_from_folder(&self) -> Vec<Card> {
        get_md_of_folder(self.subproject_path)
            .iter()
            .flat_map(|f| {
                let content = std::fs::read_to_string(f).unwrap();
                self.generate_card_from_input(&content, f.as_path())
            })
            .collect()
    }
}
