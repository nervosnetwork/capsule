use super::recipe::DeploymentRecipe;
use crate::deployment::recipe::*;
use crate::wallet::cli_types::HumanCapacity;
use ckb_testtool::ckb_types::H256;
use serde::Serialize;

#[derive(Serialize)]
pub struct Plan {
    migrated_capacity: String,
    new_occupied_capacity: String,
    txs_fee_capacity: String,
    total_occupied_capacity: String,
    recipe: RecipePlan,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecipePlan {
    pub cells: Vec<CellPlan>,
    pub dep_groups: Vec<DepGroupPlan>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CellPlan {
    pub name: String,
    pub index: u32,
    pub tx_hash: H256,
    pub occupied_capacity: String,
    pub data_hash: H256,
    pub type_id: Option<H256>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DepGroupPlan {
    pub name: String,
    pub tx_hash: H256,
    pub index: u32,
    pub occupied_capacity: String,
}

impl From<DeploymentRecipe> for RecipePlan {
    fn from(recipe: DeploymentRecipe) -> Self {
        RecipePlan {
            cells: recipe.cell_recipes.into_iter().map(Into::into).collect(),
            dep_groups: recipe
                .dep_group_recipes
                .into_iter()
                .map(Into::into)
                .collect(),
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
            tx_hash: recipe.tx_hash,
            type_id: recipe.type_id,
        }
    }
}

impl From<DepGroupRecipe> for DepGroupPlan {
    fn from(recipe: DepGroupRecipe) -> Self {
        DepGroupPlan {
            name: recipe.name,
            index: recipe.index,
            occupied_capacity: format!("{:#}", HumanCapacity::from(recipe.occupied_capacity)),
            tx_hash: recipe.tx_hash,
        }
    }
}

impl Plan {
    pub fn new(
        migrated_capacity: u64,
        new_occupied_capacity: u64,
        total_occupied_capacity: u64,
        txs_fee_capacity: u64,
        recipe: DeploymentRecipe,
    ) -> Self {
        Plan {
            migrated_capacity: format!("{:#}", HumanCapacity::from(migrated_capacity)),
            new_occupied_capacity: format!("{:#}", HumanCapacity::from(new_occupied_capacity)),
            txs_fee_capacity: format!("{:#}", HumanCapacity::from(txs_fee_capacity)),
            total_occupied_capacity: format!("{:#}", HumanCapacity::from(total_occupied_capacity)),
            recipe: recipe.into(),
        }
    }
}
