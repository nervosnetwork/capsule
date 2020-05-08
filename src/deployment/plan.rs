use super::recipe::DeploymentRecipe;
use crate::deployment::recipe::*;
use crate::wallet::cli_types::HumanCapacity;
use ckb_tool::ckb_types::H256;
use serde::Serialize;

#[derive(Serialize)]
pub struct Plan {
    migrated_capacity: String,
    new_capacity: String,
    total_used_capacity: String,
    tx_fee_capacity: String,
    recipe: RecipePlan,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecipePlan {
    pub cell_txs: Vec<CellTxPlan>,
    pub dep_group_txs: Vec<DepGroupTxPlan>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CellPlan {
    pub name: String,
    pub index: u32,
    pub occupied_capacity: String,
    pub data_hash: H256,
}

#[derive(Clone, Debug, Serialize)]
pub struct CellTxPlan {
    pub tx_hash: H256,
    pub cells: Vec<CellPlan>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DepGroupTxPlan {
    pub tx_hash: H256,
    pub dep_groups: Vec<DepGroupPlan>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DepGroupPlan {
    pub name: String,
    pub index: u32,
    pub occupied_capacity: String,
}

impl From<DeploymentRecipe> for RecipePlan {
    fn from(recipe: DeploymentRecipe) -> Self {
        RecipePlan {
            cell_txs: recipe.cell_txs.into_iter().map(Into::into).collect(),
            dep_group_txs: recipe.dep_group_txs.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CellTxRecipe> for CellTxPlan {
    fn from(recipe: CellTxRecipe) -> Self {
        CellTxPlan {
            tx_hash: recipe.tx_hash,
            cells: recipe.cells.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CellRecipe> for CellPlan {
    fn from(recipe: CellRecipe) -> Self {
        CellPlan {
            name: recipe.name,
            index: recipe.index,
            data_hash: recipe.data_hash,
            occupied_capacity: format!("{:#}", HumanCapacity::from(recipe.occupied_capacity)),
        }
    }
}

impl From<DepGroupTxRecipe> for DepGroupTxPlan {
    fn from(recipe: DepGroupTxRecipe) -> Self {
        DepGroupTxPlan {
            tx_hash: recipe.tx_hash,
            dep_groups: recipe.dep_groups.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DepGroupRecipe> for DepGroupPlan {
    fn from(recipe: DepGroupRecipe) -> Self {
        DepGroupPlan {
            name: recipe.name,
            index: recipe.index,
            occupied_capacity: format!("{:#}", HumanCapacity::from(recipe.occupied_capacity)),
        }
    }
}

impl Plan {
    pub fn new(
        migrated_capacity: u64,
        new_capacity: u64,
        total_used_capacity: u64,
        tx_fee_capacity: u64,
        recipe: DeploymentRecipe,
    ) -> Self {
        Plan {
            migrated_capacity: format!("{:#}", HumanCapacity::from(migrated_capacity)),
            new_capacity: format!("{:#}", HumanCapacity::from(new_capacity)),
            total_used_capacity: format!("{:#}", HumanCapacity::from(total_used_capacity)),
            tx_fee_capacity: format!("{:#}", HumanCapacity::from(tx_fee_capacity)),
            recipe: recipe.into(),
        }
    }
}
