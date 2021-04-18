//! Deployment Recipes

use ckb_tool::ckb_types::H256;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CellRecipe {
    pub name: String,
    pub tx_hash: H256,
    pub index: u32,
    pub occupied_capacity: u64,
    pub data_hash: H256,
    pub type_id: Option<H256>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepGroupRecipe {
    pub name: String,
    pub tx_hash: H256,
    pub index: u32,
    pub occupied_capacity: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DeploymentRecipe {
    pub cell_recipes: Vec<CellRecipe>,
    pub dep_group_recipes: Vec<DepGroupRecipe>,
}
