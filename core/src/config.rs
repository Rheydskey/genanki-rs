use std::{collections::HashMap, io::Read};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub repo: HashMap<String, Repo>,
}

impl Config {
    pub fn from_file(path: String) -> anyhow::Result<Config> {
        let mut file = std::fs::File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let toml = toml::from_str(&content);

        Ok(toml?)
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Repo {
    SimpleUrl(String),
    Object {
        url: String,
        target: Option<String>,
        deck_name: Option<String>,
    },
}

impl Repo {
    pub fn get_url(&self) -> &String {
        match self {
            Repo::SimpleUrl(url) | Repo::Object { url, .. } => url,
        }
    }

    pub fn get_slug(&self) -> String {
        let digest = match self {
            Repo::SimpleUrl(url) | Repo::Object { url, .. } => sha256::digest(url),
        };

        digest[0..6].to_string()
    }

    pub fn get_custom_deck_name(&self) -> Option<String> {
        match self {
            Repo::Object { deck_name, .. } => deck_name.clone(),
            Repo::SimpleUrl(_) => None,
        }
    }

    pub fn get_subfolder(&self) -> String {
        match self {
            Repo::SimpleUrl(_) => String::new(),
            Repo::Object { target, .. } => target.clone().unwrap_or_default(),
        }
    }
}
