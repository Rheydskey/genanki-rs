use pyo3::prelude::*;
use std::path::Path;

use crate::{config::Config, data::Output, init::Init, updater::Updater};

mod config;
mod data;
mod generator;
mod git;
mod init;
mod markdown;
mod updater;

#[cfg(test)]
mod tests;

pub fn init(url: &str, output_path: &str, target_path: &Path) -> PyResult<Output> {
    let init = Init::new(url, output_path, target_path);
    init.git_clone()?;
    Ok(init.generate()?)
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
        let root_deck_name = repo.get_custom_deck_name().unwrap_or_else(|| name.clone());
        let subfolder = repo.get_subfolder();
        let repo_folder = std::path::Path::new(&slug);

        if repo_folder.exists() {
            let values = update(repo_folder.to_str().unwrap().to_string())?;
            for (decks, cards) in values {
                output.insert(format!("{root_deck_name}::{decks}"), cards);
            }
        } else {
            let values = init(url, &slug, &repo_folder.join(subfolder))?;
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
