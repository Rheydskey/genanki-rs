use std::{collections::HashMap, process::Stdio};

use argh::FromArgs;

use crate::{
    data::{Card, InitOutput},
    generator::{Generator, Updater},
};

mod data;
mod generator;
mod git;

#[cfg(test)]
mod test;

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "init")]
/// Init a repo containing genanki-rs folder
/// and returning init json
pub struct Init {
    #[argh(positional)]
    /// repo: Url path of git repo
    repo: String,
    /// output_folder: Name of output folder
    #[argh(positional)]
    output_folder: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "update")]
/// Update a repo containing genanki-rs folder
/// and returning update json
pub struct Update {
    #[argh(positional)]
    /// path repo
    repo: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum SubcommandEnum {
    Init(Init),
    Update(Update),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
struct TopLevel {
    #[argh(subcommand)]
    nested: SubcommandEnum,
}

#[must_use]
pub fn init(init: &Init) -> anyhow::Result<String> {
    let git = std::process::Command::new("git")
        .args(["clone", "--depth", "1", &init.repo, &init.output_folder])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    eprintln!("{}", String::from_utf8(git.stdout)?);
    eprintln!("{}", String::from_utf8(git.stderr)?);
    let mut decks: HashMap<String, Vec<Card>> = HashMap::new();
    for path in std::fs::read_dir(&init.output_folder)? {
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

    Ok(serde_json::to_string(&InitOutput { decks })?)
}

#[must_use]
pub fn update(update: &Update) -> anyhow::Result<String> {
    Updater::new(update.repo.clone()).generate()
}

fn main() {
    let a: TopLevel = argh::from_env();
    let output = match a.nested {
        SubcommandEnum::Init(i) => init(&i),
        SubcommandEnum::Update(u) => update(&u),
    };

    match output {
        Ok(output_str) => print!("{output_str}"),
        Err(err) => eprintln!("{err}"),
    }
}
