use ckb_tool::ckb_types::{
    packed::{CellInput, OutPoint},
    prelude::*,
    H256,
};
use serde::{Deserialize, Serialize};

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct LiveCell {
    pub tx_hash: H256,
    pub index: u32,
    pub capacity: u64,
    pub mature: bool,
}

impl LiveCell {
    pub fn out_point(&self) -> OutPoint {
        OutPoint::new(self.tx_hash.clone().pack(), self.index)
    }
    pub fn input(&self) -> CellInput {
        CellInput::new(self.out_point(), 0)
    }
}
