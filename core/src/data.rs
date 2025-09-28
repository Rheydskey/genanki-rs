use std::collections::HashMap;

#[derive(serde::Serialize, Clone, Debug, pyo3::IntoPyObject)]
pub struct Card {
    pub front: String,
    pub back: String,
    pub hash: String,
}

#[derive(serde::Serialize, Clone)]
pub struct InitOutput {
    pub decks: HashMap<String, Vec<Card>>,
}

#[derive(serde::Serialize, Clone, Debug, Default)]
pub struct UpdateOutput {
    pub decks: HashMap<String, DiffOutput>,
}

#[derive(serde::Serialize, Clone, Debug, Default, pyo3::IntoPyObject)]
pub struct DiffOutput {
    pub added: Vec<Card>,
    /// Vec of hash
    pub deleted: Vec<String>,
}
