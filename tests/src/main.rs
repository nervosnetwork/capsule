use anyhow::Error;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONTRACT_NAME: &str = "demo-contract";

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
    let exit_code = Command::new("capsule")
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
        .arg("make build-via-docker")
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        panic!("command crash, exit_code {:?}", exit_code.code());
    }
    println!("Run contract test ...");
    let exit_code = Command::new("bash")
        .arg("-c")
        .arg("capsule test")
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        panic!("command crash, exit_code {:?}", exit_code.code());
    }
    println!("Success!");
    Ok(())
}
