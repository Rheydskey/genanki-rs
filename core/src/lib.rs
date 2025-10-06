use std::{collections::HashMap, path::Path, process::Stdio};

use pyo3::prelude::*;

use crate::{
    config::Config,
    data::{DeckOutput, Output},
    generator::{Generator, Updater},
};

mod config;
mod data;
mod generator;
mod git;

#[cfg(test)]
mod test;

pub fn init(url: &str, output_path: &str, target_path: &Path) -> PyResult<Output> {
    let git = std::process::Command::new("git")
        .args(["clone", "--depth", "1", url, output_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    eprintln!("{}", String::from_utf8(git.stdout)?);
    eprintln!("{}", String::from_utf8(git.stderr)?);
    let mut decks: Output = HashMap::new();

    for path in std::fs::read_dir(target_path)? {
        let Ok(path) = path else {
            continue;
        };

        if !path.file_type()?.is_dir() {
            continue;
        }

        let Ok(name) = path.file_name().into_string() else {
            continue;
        };

        if name.starts_with('.') {
            continue;
        }
        decks.insert(
            name,
            DeckOutput {
                added: Generator::generate_card_from_folder(path.path().as_path()),
                deleted: Vec::new(),
            },
        );
    }

    Ok(decks)
}

#[pyfunction]
pub fn update(path: String) -> PyResult<Output> {
    Ok(Updater::new(path).generate()?)
}

#[pyfunction]
pub fn from_config(path: String) -> PyResult<Output> {
    let config = Config::from_file(path)?;
    let mut output = Output::new();
    for (name, repo) in &config.repo {
        let slug = repo.get_slug();
        let url = repo.get_url();
        let root_deck_name = repo.get_custom_deck_name().unwrap_or(name.clone());
        let subfolder = repo.get_subfolder();
        let repo_folder = std::path::Path::new(&slug);

        if repo_folder.exists() {
            let values = update(repo_folder.to_str().unwrap().to_string())?;
            for (decks, cards) in values {
                output.insert(format!("{root_deck_name}::{decks}"), cards);
            }
        } else {
            let values = init(&url, &slug, &repo_folder.join(subfolder))?;
            for (decks, cards) in values {
                output.insert(format!("{root_deck_name}::{decks}"), cards);
            }
        }
    }

    Ok(output)
}

#[pymodule]
#[pyo3(name = "gencore")]
fn gencore(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(from_config, module)?)?;
    Ok(())
}
