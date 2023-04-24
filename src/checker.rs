use anyhow::{anyhow, Result};
use log::warn;
use std::fmt;
use std::process::{Command, Output};

fn check_cmd(program: &str, arg: &str) -> Result<Output> {
    Command::new(program).arg(arg).output().map_err(Into::into)
}

pub struct Checker {
    cargo: bool,
    docker: bool,
    ckb_cli: Option<Vec<u8>>,
}

impl Checker {
    pub fn build(ckb_cli_bin: &str) -> Result<Self> {
        let cargo = check_cmd("cargo", "version")
            .map(|output| output.status.success())
            .unwrap_or(false);
        let docker = check_cmd("docker", "version")
            .map(|output| output.status.success())
            .unwrap_or(false);
        let ckb_cli = check_cmd(ckb_cli_bin, "--version")
            .map(|output| output.stdout)
            .ok();
        Ok(Checker {
            cargo,
            docker,
            ckb_cli,
        })
    }

    pub fn check_ckb_cli(&self) -> Result<()> {
        if self.ckb_cli.is_none() {
            return Err(anyhow!("Can't find ckb-cli"));
        }
        match Version::parse_with_prefix("ckb-cli", self.ckb_cli.clone().unwrap()) {
            Ok(v) if v >= REQUIRED_CKB_CLI_VERSION => {}
            Ok(v) => {
                return Err(anyhow!(
                    "Find ckb-cli {} (required {})",
                    v,
                    REQUIRED_CKB_CLI_VERSION
                ));
            }
            Err(_) => {
                warn!("Find ckb-cli (unknown version)");
            }
        }
        Ok(())
    }

    pub fn print_report(&self) {
        println!("------------------------------");
        if self.cargo {
            println!("cargo\tinstalled");
        } else {
            println!(
                "cargo\tnot found - Please install rust (https://www.rust-lang.org/tools/install)"
            );
        }
        if self.docker {
            println!("docker\tinstalled");
        } else {
            println!("docker\tnot found - Please install docker");
        }
        if self.ckb_cli.is_some() {
            match Version::parse_with_prefix("ckb-cli", self.ckb_cli.clone().unwrap()) {
                Ok(v) if v >= REQUIRED_CKB_CLI_VERSION => {
                    println!("ckb-cli\tinstalled {}", v);
                }
                Ok(v) => {
                    println!(
                        "ckb-cli\tinstalled {} (required {})",
                        v, REQUIRED_CKB_CLI_VERSION
                    );
                }
                Err(_) => {
                    println!("ckb-cli\tinstalled (unknown version)");
                }
            }
        } else {
            println!("ckb-cli\tnot found - The deployment feature is disabled");
        }
        println!("------------------------------");
    }
}

const REQUIRED_CKB_CLI_VERSION: Version = Version(1, 2, 0);

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Version(usize, usize, usize);

impl Version {
    fn parse_with_prefix(prefix: &'static str, buf: Vec<u8>) -> Result<Self> {
        let s = String::from_utf8(buf)?;
        let vers = s.trim_start_matches(prefix);
        let vers = vers
            .split_whitespace()
            .next()
            .ok_or(anyhow!("no version found"))?;
        let mut vers_numbers = vers.split('.');
        let major: usize = vers_numbers
            .next()
            .ok_or(anyhow!("miss major version"))?
            .parse()?;
        let minor: usize = vers_numbers
            .next()
            .ok_or(anyhow!("miss minor version"))?
            .parse()?;
        let patch: usize = vers_numbers
            .next()
            .ok_or(anyhow!("miss patch version"))?
            .parse()?;
        if vers_numbers.next().is_some() {
            return Err(anyhow!("parse version error"));
        }
        Ok(Version(major, minor, patch))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}.{}", self.0, self.1, self.2)
    }
}
