//! Deployment Recipes

use ckb_tool::ckb_types::H256;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CellRecipe {
    pub name: String,
    pub index: u32,
    pub occupied_capacity: u64,
    pub data_hash: H256,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CellTxRecipe {
    pub tx_hash: H256,
    pub cells: Vec<CellRecipe>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepGroupRecipe {
    pub name: String,
    pub index: u32,
    pub occupied_capacity: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepGroupTxRecipe {
    pub tx_hash: H256,
    pub dep_groups: Vec<DepGroupRecipe>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeploymentRecipe {
    pub cell_txs: Vec<CellTxRecipe>,
    pub dep_group_txs: Vec<DepGroupTxRecipe>,
}
