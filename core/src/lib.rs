use std::{collections::HashMap, process::Stdio};

use pyo3::prelude::*;

use crate::{
    data::{Card, DiffOutput},
    generator::{Generator, Updater},
};

mod data;
mod generator;
mod git;

#[pyfunction]
pub fn init(url: String, output_path: String) -> PyResult<HashMap<String, Vec<Card>>> {
    let git = std::process::Command::new("git")
        .args(["clone", "--depth", "1", &url, &output_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    eprintln!("{}", String::from_utf8(git.stdout)?);
    eprintln!("{}", String::from_utf8(git.stderr)?);
    let mut decks: HashMap<String, Vec<Card>> = HashMap::new();

    for path in std::fs::read_dir(&output_path)? {
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
            Generator::generate_card_from_folder(path.path().as_path()),
        );
    }

    Ok(decks)
}

#[pyfunction]
pub fn update(path: String) -> PyResult<HashMap<String, DiffOutput>> {
    Ok(Updater::new(path).generate()?.decks)
}

#[pymodule]
#[pyo3(name = "gencore")]
fn gencore(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(init, module)?)?;
    module.add_function(wrap_pyfunction!(update, module)?)?;
    Ok(())
}
