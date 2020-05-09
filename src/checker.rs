use anyhow::Result;
use std::process::{Command, Stdio};

fn check_cmd(program: &str, arg: &str) -> Result<bool> {
    let success = Command::new(program)
        .arg(arg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?
        .wait()?
        .success();
    Ok(success)
}

pub struct Checker;

impl Checker {
    pub fn run() -> Result<()> {
        let docker_exist = check_cmd("docker", "version").unwrap_or(false);
        let ckb_cli_exist = check_cmd("ckb-cli", "--version").unwrap_or(false);

        println!("------------------------------");
        if docker_exist {
            println!("docker\tinstalled");
        } else {
            println!("docker\tnot found - Please install docker");
        }
        if ckb_cli_exist {
            println!("ckb-cli\tinstalled");
        } else {
            println!("ckb-cli\tnot found - The deployment feature is disabled");
        }
        println!("------------------------------");
        Ok(())
    }
}
