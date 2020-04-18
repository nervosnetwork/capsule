use ckb_tool::{ckb_jsonrpc_types::Script, ckb_types::H256};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// contracts config
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
    pub deployment: PathBuf,
}

// Deployment
// TODO
// 1. support refer to builtin cells by name
// 2. deploy via ckb cli
// 3. output simulation before deployment
// 4. output type_id.toml as deployment results
// 5. two phase deploy, 1 use a unlockable script, 2 use a true lock script
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Deployment {
    pub lock: Script,
    pub cells: Vec<Cell>,
    #[serde(default)]
    pub dep_groups: Vec<DepGroup>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellLocation {
    OutPoint { tx_hash: H256, index: u32 },
    File { file: String },
}

impl CellLocation {
    pub fn is_on_chain(&self) -> bool {
        match self {
            CellLocation::File { .. } => false,
            _ => true,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub name: String,
    pub location: CellLocation,
    pub enable_type_id: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DepGroup {
    pub name: String,
    pub cells: Vec<String>,
}
