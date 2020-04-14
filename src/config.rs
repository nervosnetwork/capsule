use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum TemplateType {
    Rust,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Contract {
    pub name: String,
    pub template_type: TemplateType,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub contracts: Vec<Contract>,
}
