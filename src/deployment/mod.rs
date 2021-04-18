pub mod deployment_process;
pub mod manage;
mod plan;
mod recipe;
mod tx_check;

use ckb_tool::ckb_types::core::TransactionView;

#[derive(Debug, Clone)]
pub struct BakedTransaction {
    pub cells: Option<TransactionView>,
    pub dep_groups: Option<TransactionView>,
}

impl BakedTransaction {
    fn is_empty(&self) -> bool {
        self.cells.is_none() && self.dep_groups.is_none()
    }

    fn len(&self) -> usize {
        self.cells.is_some() as usize + self.dep_groups.is_some() as usize
    }
}
