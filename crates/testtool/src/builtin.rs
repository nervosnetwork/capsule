//! This module contains some builtin contracts

use ckb_types::bytes::Bytes;
use lazy_static::lazy_static;

lazy_static! {
    /// Always return success
    pub static ref ALWAYS_SUCCESS: Bytes =
        ckb_always_success_script::ALWAYS_SUCCESS.to_vec().into();
}
