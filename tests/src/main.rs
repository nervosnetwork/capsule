use anyhow::Error;
use ckb_tool::{
    ckb_types::{bytes::Bytes, core::ScriptHashType, packed::*, prelude::*},
    testtool::{context::Context, tx_builder::TxBuilder},
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONTRACT_NAME: &str = "demo-contract";
const EXPECTED_CYCLES: u64 = 6288;

fn main() {
    let mut dir = env::current_dir().expect("current dir");
    dir.push("tmp");
    fs::create_dir(&dir).expect("create dir");
    test_build(&dir).unwrap();
    fs::remove_dir_all(&dir).expect("remove dir");
}

fn test_build<P: AsRef<Path>>(dir: P) -> Result<(), Error> {
    env::set_current_dir(&dir)?;
    let mut contract_path = PathBuf::new();
    contract_path.push(&dir);
    contract_path.push(CONTRACT_NAME);
    println!("Creating {:?} ...", contract_path);
    Command::new("capsule").arg(CONTRACT_NAME).spawn()?.wait()?;
    println!("Building ...");
    env::set_current_dir(&contract_path)?;
    Command::new("bash")
        .arg("-c")
        .arg("make build-via-docker")
        .spawn()?
        .wait()?;
    println!("Run contract ...");
    let mut bin_path = contract_path.clone();
    bin_path.push("build");
    bin_path.push(CONTRACT_NAME);
    let contract_bin: Bytes = fs::read(bin_path)?.into();
    let contract_code_hash = CellOutput::calc_data_hash(&contract_bin);
    let mut context = Context::default();
    context.deploy_contract(contract_bin.clone());
    let tx = TxBuilder::default()
        .lock_script(
            Script::new_builder()
                .code_hash(contract_code_hash)
                .hash_type(ScriptHashType::Data.into())
                .build()
                .as_slice()
                .to_owned()
                .into(),
        )
        .inject_and_build(&mut context)
        .expect("build tx");
    let verify_result = context.verify_tx(&tx, EXPECTED_CYCLES);
    let cycles = verify_result.expect("pass verification");
    assert_eq!(cycles, EXPECTED_CYCLES);
    println!("Success! cycles: {}", cycles);
    Ok(())
}
