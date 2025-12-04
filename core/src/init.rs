use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Stdio,
};

use crate::{
    data::{DeckOutput, Output},
    generator::Generator,
};

pub struct Init<'a> {
    url: &'a str,
    output_path: &'a str,
    target_path: &'a Path,
}

impl<'a> Init<'a> {
    pub fn new(url: &'a str, output_path: &'a str, target_path: &'a Path) -> Self {
        Self {
            url,
            output_path,
            target_path,
        }
    }

    pub fn get_subdecks_path(&self) -> anyhow::Result<Vec<PathBuf>> {
        let Self { target_path, .. } = self;
        let canonic = target_path.canonicalize()?;
        let mut check_path = HashSet::new();
        let mut s = Vec::new();
        check_path.insert(canonic.clone());
        s.push(canonic.clone());

        while !s.is_empty() {
            let path = s.pop().unwrap();
            for path in std::fs::read_dir(path).unwrap().flatten() {
                if !path.file_type()?.is_dir() {
                    continue;
                }

                let Ok(name) = path.file_name().into_string() else {
                    continue;
                };

                if name.starts_with('.') {
                    continue;
                }
                let c = path.path().canonicalize()?;
                if !check_path.contains(&c) {
                    s.push(c.clone());
                }
                check_path.insert(c);
            }
        }

        check_path.remove(&canonic);

        Ok(check_path
            .into_iter()
            .filter_map(|f| f.strip_prefix(&canonic).map(|f| f.to_path_buf()).ok())
            .collect())
    }

    pub fn git_clone(&self) -> anyhow::Result<()> {
        let Self {
            url, output_path, ..
        } = self;

        let git = std::process::Command::new("git")
            .args(["clone", "--depth", "1", url, output_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        eprintln!("{}", String::from_utf8(git.stdout)?);
        eprintln!("{}", String::from_utf8(git.stderr)?);

        Ok(())
    }

    pub fn generate(&self) -> anyhow::Result<Output> {
        let mut decks: Output = HashMap::new();
        for path in self.get_subdecks_path()? {
            let name = path.to_str().unwrap().replace("/", "::");
            decks.insert(
                name.to_string(),
                DeckOutput {
                    added: Generator {
                        subproject_path: self.target_path.join(path.as_path()).as_path(),
                    }
                    .generate_card_from_folder(),
                    ..Default::default()
                },
            );
        }

        Ok(decks)
    }
}
