//! Deployment Recipes

#[derive(Clone, Debug)]
pub struct CellRecipe {
    pub name: String,
    pub index: u32,
    pub data_hash: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct CellTxRecipe {
    pub tx_hash: [u8; 32],
    pub cells: Vec<CellRecipe>,
}

#[derive(Clone, Debug)]
pub struct DepGroupRecipe {
    pub name: String,
    pub index: u32,
}

#[derive(Clone, Debug)]
pub struct DepGroupTxRecipe {
    pub tx_hash: [u8; 32],
    pub dep_groups: Vec<DepGroupRecipe>,
}

#[derive(Clone, Debug)]
pub struct DeploymentRecipe {
    pub cell_txs: Vec<CellTxRecipe>,
    pub dep_group_txs: Vec<DepGroupTxRecipe>,
}
