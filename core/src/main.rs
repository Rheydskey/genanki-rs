use std::{collections::HashMap, process::Stdio};

use argh::FromArgs;

use crate::{
    data::{Card, InitOutput},
    generator::{Generator, Updater},
};

mod data;
mod generator;

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
pub fn init(init: &Init) -> String {
    let git = std::process::Command::new("git")
        .args(["clone", "--depth", "1", &init.repo, &init.output_folder])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    eprintln!("{}", String::from_utf8(git.stdout).unwrap());
    eprintln!("{}", String::from_utf8(git.stderr).unwrap());
    let mut decks: HashMap<String, Vec<Card>> = HashMap::new();
    for path in std::fs::read_dir(&init.output_folder).unwrap() {
        let Ok(path) = path else {
            continue;
        };

        if path.file_type().unwrap().is_dir() {
            let name = path.file_name().into_string().unwrap();
            if name.starts_with('.') {
                continue;
            }
            decks.insert(
                name,
                Generator::generate_card_from_folder(path.path().as_path()),
            );
        }
    }

    serde_json::to_string(&InitOutput { decks }).unwrap()
}

#[must_use]
pub fn update(update: &Update) -> String {
    Updater::new(update.repo.clone()).generate()
}

fn main() {
    let a: TopLevel = argh::from_env();
    let output = match a.nested {
        SubcommandEnum::Init(i) => init(&i),
        SubcommandEnum::Update(u) => update(&u),
    };

    print!("{output}");
}
