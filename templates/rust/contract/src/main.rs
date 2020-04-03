#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

use ckb_std::{entry, default_alloc};

entry!(main);
default_alloc!();

#[no_mangle]
fn main() -> i8 {
    // this contract always return true
    0
}
