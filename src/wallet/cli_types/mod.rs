mod address;
mod human_capacity;
mod live_cell_info;
mod network_type;
mod signature;

pub use address::Address;
pub use live_cell_info::{LiveCellInfo, LiveCellInfoVec};
pub use signature::SignatureOutput;

use ckb_tool::ckb_types::{h256, H256};

pub const ONE_CKB: u64 = 1_00000000;
pub const PREFIX_MAINNET: &str = "ckb";
pub const PREFIX_TESTNET: &str = "ckt";

pub const NETWORK_MAINNET: &str = "ckb";
pub const NETWORK_TESTNET: &str = "ckb_testnet";
pub const NETWORK_STAGING: &str = "ckb_staging";
pub const NETWORK_DEV: &str = "ckb_dev";
pub const SIGHASH_TYPE_HASH: H256 =
    h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");
pub const MULTISIG_TYPE_HASH: H256 =
    h256!("0x5c5069eb0857efc65e1bca0c07df34c31663b3622fd3876c876320fc9634e2a8");
