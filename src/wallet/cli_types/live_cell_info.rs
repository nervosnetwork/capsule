use super::human_capacity::HumanCapacity;
use super::live_cell::LiveCell;
use ckb_testtool::ckb_types::H256;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct LiveCellInfoVec {
    pub live_cells: Vec<LiveCellInfo>,
    pub current_capacity: String,
    pub current_count: usize,
    pub total_capacity: String,
    pub total_count: usize,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct LiveCellInfo {
    pub tx_hash: H256,
    pub output_index: u32,
    pub data_bytes: u64,
    pub lock_hash: H256,
    // Type script's code_hash and script_hash
    pub type_hashes: Option<(H256, H256)>,
    // Capacity
    pub capacity: String,
    // Block number
    pub number: u64,
    // Location in the block
    pub index: CellIndex,
    pub mature: bool,
}

impl Into<LiveCell> for LiveCellInfo {
    fn into(self) -> LiveCell {
        let capacity = self.capacity();
        let index = self.index.output_index;
        let LiveCellInfo {
            tx_hash, mature, ..
        } = self;
        LiveCell {
            tx_hash,
            index,
            capacity,
            mature,
        }
    }
}

impl LiveCellInfo {
    pub fn capacity(&self) -> u64 {
        HumanCapacity::from_str(&self.capacity)
            .expect("parse capacity")
            .0
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct CellIndex {
    // The transaction index in the block
    pub tx_index: u32,
    // The output index in the transaction
    pub output_index: u32,
}

impl CellIndex {
    // pub(crate) fn to_bytes(self) -> Vec<u8> {
    //     let mut bytes = self.tx_index.to_be_bytes().to_vec();
    //     bytes.extend(self.output_index.to_be_bytes().to_vec());
    //     bytes
    // }

    // pub(crate) fn from_bytes(bytes: [u8; 8]) -> CellIndex {
    //     let mut tx_index_bytes = [0u8; 4];
    //     let mut output_index_bytes = [0u8; 4];
    //     tx_index_bytes.copy_from_slice(&bytes[..4]);
    //     output_index_bytes.copy_from_slice(&bytes[4..]);
    //     CellIndex {
    //         tx_index: u32::from_be_bytes(tx_index_bytes),
    //         output_index: u32::from_be_bytes(output_index_bytes),
    //     }
    // }

    // pub(crate) fn new(tx_index: u32, output_index: u32) -> CellIndex {
    //     CellIndex {
    //         tx_index,
    //         output_index,
    //     }
    // }
}
