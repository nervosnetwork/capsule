//! ckb-testtool
//!
//! This module provides testing context for CKB contracts.
//!
//! To setup a contract verification context, you may need to import ckb modules
//! to build the transaction structure or calculate the hash result.
//! `ckb-testtool` crate provides re-exports of ckb modules.
//!
//! # Example
//!
//! ``` rust
//! use ckb_testtool::context::Context;
//! use ckb_testtool::ckb_types::{
//!     bytes::Bytes,
//!     core::TransactionBuilder,
//!     packed::*,
//!     prelude::*,
//! };
//! use std::fs;
//!
//! // max cycles of verification
//! const MAX_CYCLES: u64 = 10_000_000;
//!
//! #[test]
//! fn test_basic() {
//!     // Init testing context
//!     let mut context = Context::default();
//!     let contract_bin: Bytes = fs::read("my_contract").unwrap().into();
//!
//!     // deploy contract
//!     let out_point = context.deploy_cell(contract_bin);
//!
//!     // prepare scripts and cell dep
//!     let lock_script = context
//!         .build_script(&out_point, Default::default())
//!         .expect("script");
//!     let lock_script_dep = CellDep::new_builder()
//!         .out_point(out_point)
//!         .build();
//!
//!     // prepare input cell
//!     let input_out_point = context.create_cell(
//!         CellOutput::new_builder()
//!             .capacity(1000u64.pack())
//!             .lock(lock_script.clone())
//!             .build(),
//!         Bytes::new(),
//!     );
//!     let input = CellInput::new_builder()
//!         .previous_output(input_out_point)
//!         .build();
//!
//!     // outputs
//!     let outputs = vec![
//!         CellOutput::new_builder()
//!             .capacity(500u64.pack())
//!             .lock(lock_script.clone())
//!             .build(),
//!         CellOutput::new_builder()
//!             .capacity(500u64.pack())
//!             .lock(lock_script)
//!             .build(),
//!     ];
//!
//!     let outputs_data = vec![Bytes::new(); 2];
//!
//!     // build transaction
//!     let tx = TransactionBuilder::default()
//!         .input(input)
//!         .outputs(outputs)
//!         .outputs_data(outputs_data.pack())
//!         .cell_dep(lock_script_dep)
//!         .build();
//!
//!     let tx = context.complete_tx(tx);
//!
//!     // run
//!     let cycles = context
//!         .verify_tx(&tx, MAX_CYCLES)
//!         .expect("pass verification");
//!     println!("consume cycles: {}", cycles);
//! }
//! ```

pub mod builtin;
pub mod context;
mod tx_verifier;

// re-exports
pub use ckb_chain_spec;
pub use ckb_crypto;
pub use ckb_error;
pub use ckb_hash;
pub use ckb_jsonrpc_types;
pub use ckb_script;
pub use ckb_traits;
pub use ckb_types;
pub use ckb_types::bytes;
pub use ckb_verification;

#[cfg(test)]
mod tests;
