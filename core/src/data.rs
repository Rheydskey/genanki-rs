use std::collections::HashMap;

#[derive(Clone, Debug, pyo3::IntoPyObject)]
pub struct Card {
    pub front: String,
    pub back: String,
    pub hash: String,
}

#[derive(Clone, Debug, Default, pyo3::IntoPyObject)]
pub struct DiffOutput {
    pub added: Vec<Card>,
    /// Vec of hash
    pub deleted: Vec<String>,
}

pub type InitOutput = HashMap<String, Vec<Card>>;
pub type UpdateOutput = HashMap<String, DiffOutput>;
