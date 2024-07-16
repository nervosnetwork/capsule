ckb_std::entry_simulator!(program_entry);
// ckb_std::default_alloc!();

/// program entry
fn program_entry() -> i8 {
    // Call main function and return error code
    match test_contract::entry::main() {
        Ok(_) => 0,
        Err(err) => err as i8,
    }
}
