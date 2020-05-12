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

pub struct Checker {
    pub docker: bool,
    pub ckb_cli: bool,
}

impl Checker {
    pub fn build() -> Result<Self> {
        let docker = check_cmd("docker", "version").unwrap_or(false);
        let ckb_cli = check_cmd("ckb-cli", "--version").unwrap_or(false);
        Ok(Checker { docker, ckb_cli })
    }

    pub fn print_report(&self) {
        println!("------------------------------");
        if self.docker {
            println!("docker\tinstalled");
        } else {
            println!("docker\tnot found - Please install docker");
        }
        if self.ckb_cli {
            println!("ckb-cli\tinstalled");
        } else {
            println!("ckb-cli\tnot found - The deployment feature is disabled");
        }
        println!("------------------------------");
    }
}
