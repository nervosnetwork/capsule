#![cfg_attr(not(feature = "simulator"), no_std)]

#[cfg(feature = "simulator")]
pub mod entry;
#[cfg(feature = "simulator")]
pub mod error;
