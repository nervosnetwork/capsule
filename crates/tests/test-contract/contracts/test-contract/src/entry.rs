// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import CKB syscalls and structures
use ckb_std::{
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_header, load_script},
    ckb_constants::Source,
};

use crate::error::Error;

pub fn main() -> Result<(), Error> {
    // remove below examples and write your code here

    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:?}", args);

    if args.is_empty() {
        return Err(Error::MyError);
    }

    let header = load_header(0, Source::Input)?;
    debug!("header1 {:?}", header);
    let header = load_header(0, Source::CellDep)?;
    debug!("header2 {:?}", header);
    let header = load_header(0, Source::HeaderDep)?;
    debug!("header3 {:?}", header);

    Ok(())
}
