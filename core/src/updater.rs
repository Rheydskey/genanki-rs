use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
};

use gitpatch::Patch;

use crate::{
    data::{Card, DeckOutput, Output},
    generator::Generator,
    git::{Git, GitUpdate},
};

#[derive(Debug)]
pub struct Updater {
    git: Git,
    repo_path: PathBuf,
}

impl Updater {
    pub fn new(repo: String) -> Self {
        let repo_path = PathBuf::from_str(repo.as_str()).unwrap();
        let git = Git::new(repo);
        Self { git, repo_path }
    }

    fn root_folder_of_patch(path: &str) -> String {
        let mut path = PathBuf::from_str(path).unwrap();
        if path.starts_with("a/") {
            path = path.strip_prefix("a/").unwrap().to_path_buf();
        }

        if path.starts_with("b/") {
            path = path.strip_prefix("b/").unwrap().to_path_buf();
        }

        path.parent().unwrap().to_str().unwrap().to_string()
    }

    pub fn get_folder_of_patch(patch: &Patch) -> Vec<String> {
        let mut paths = Vec::new();
        if patch.old.path != "/dev/null" {
            paths.push(Self::root_folder_of_patch(&patch.old.path));
        }

        if patch.new.path != "/dev/null" {
            paths.push(Self::root_folder_of_patch(&patch.new.path));
        }

        paths
    }

    pub fn get_folder_with_diff(diff: &str) -> anyhow::Result<HashSet<String>> {
        let Ok(patchs) = gitpatch::Patch::from_multiple(diff) else {
            return Err(anyhow::anyhow!("Output diff is not correct"));
        };

        let decks = patchs
            .iter()
            .flat_map(Self::get_folder_of_patch)
            .collect::<HashSet<String>>();

        Ok(decks)
    }

    pub fn get_card_of_from_commit(
        &self,
        updated_folder: &HashSet<String>,
        from_commit: &str,
    ) -> anyhow::Result<HashMap<String, HashSet<String>>> {
        self.git.checkout(from_commit)?;

        let mut old_cards: HashMap<String, HashSet<String>> = HashMap::new();
        for i in updated_folder {
            let hashes: HashSet<String> = Generator {
                subproject_path: self.repo_path.join(i).as_path(),
            }
            .generate_card_from_folder()
            .iter()
            .map(|f| f.hash.clone())
            .collect();

            old_cards.insert(i.clone(), hashes);
        }

        Ok(old_cards)
    }

    pub fn get_cards_of_to_commit(
        &self,
        updated_folder: &HashSet<String>,
        to_commit: &str,
    ) -> anyhow::Result<HashMap<String, Vec<Card>>> {
        self.git.checkout(to_commit)?;

        let mut decks_cards = HashMap::new();
        for i in updated_folder {
            let cards = Generator {
                subproject_path: self.repo_path.join(i).as_path(),
            }
            .generate_card_from_folder();

            decks_cards.insert(i.clone(), cards);
        }

        Ok(decks_cards)
    }

    pub fn generate_decks_from_diff(
        &self,
        diff: &str,
        from_commit: &str,
        to_commit: &str,
    ) -> anyhow::Result<Output> {
        let updated_folder = Self::get_folder_with_diff(diff)?;
        let cards_from_commit = self.get_card_of_from_commit(&updated_folder, from_commit)?;
        let cards_to_commit = self.get_cards_of_to_commit(&updated_folder, to_commit)?;

        let mut output = Output::default();

        for (deck, cards) in &cards_to_commit {
            let new_cards_hash: HashSet<String> = cards.iter().map(|f| f.hash.clone()).collect();
            let Some(old_deck) = cards_from_commit.get(deck) else {
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

        self.generate_decks_from_diff(&diff, &from_commit, &to_commit)
    }
}
