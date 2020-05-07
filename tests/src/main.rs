use anyhow::Error;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONTRACT_NAME: &str = "demo-contract";
const BIN_PATH: &str = "target/debug/capsule";

fn main() {
    let cur_dir = env::current_dir().expect("current dir");
    let bin_path = {
        let mut path = PathBuf::new();
        path.push(&cur_dir);
        path.push(BIN_PATH);
        path
    };
    let tmp_dir = {
        let mut path = PathBuf::new();
        path.push(&cur_dir);
        path.push("tmp");
        path
    };
    fs::create_dir(&tmp_dir).expect("create dir");
    test_build(&tmp_dir, bin_path.to_str().expect("capsule bin path")).unwrap();
    fs::remove_dir_all(&tmp_dir).expect("remove dir");
}

fn test_build<P: AsRef<Path>>(dir: P, bin_path: &str) -> Result<(), Error> {
    env::set_current_dir(&dir)?;
    let mut contract_path = PathBuf::new();
    contract_path.push(&dir);
    contract_path.push(CONTRACT_NAME);
    println!("Creating {:?} ...", contract_path);
    let exit_code = Command::new(bin_path)
        .arg("new")
        .arg(CONTRACT_NAME)
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        panic!("command crash, exit_code {:?}", exit_code.code());
    }
    println!("Building ...");
    env::set_current_dir(&contract_path)?;
    let exit_code = Command::new("bash")
        .arg("-c")
        .arg(format!("{} build", bin_path))
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        panic!("command crash, exit_code {:?}", exit_code.code());
    }
    println!("Run contract test ...");
    let exit_code = Command::new("bash")
        .arg("-c")
        .arg(format!("{} test", bin_path))
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        panic!("command crash, exit_code {:?}", exit_code.code());
    }
    println!("Success!");
    Ok(())
}
