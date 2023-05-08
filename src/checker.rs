use anyhow::{anyhow, bail, Result};
use log::warn;
use std::fmt;
use std::process::Command;

struct BinDep {
    program: String,
    installed: bool,
    version: Option<Version>,
    required_version: Option<Version>,
}

impl BinDep {
    fn build(
        program: &str,
        arg: &'static str,
        version_prefix: Option<&'static str>,
        required_version: Option<Version>,
    ) -> Result<Self> {
        let output = Command::new(program).arg(arg).output()?;
        let installed = output.status.success();
        let version = version_prefix
            .filter(|_| installed)
            .and_then(|prefix| Version::parse_with_prefix(prefix, output.stdout).ok());
        Ok(BinDep {
            program: program.to_string(),
            installed,
            version,
            required_version,
        })
    }

    /// Check if the required version is met, return true if no required version
    fn meet_required_version(&self) -> bool {
        self.required_version
            .as_ref()
            .map(|required_version| {
                self.version
                    .as_ref()
                    .map(|version| version >= required_version)
                    .unwrap_or(true)
            })
            .unwrap_or(true)
    }
}

pub struct Checker {
    cargo: BinDep,
    docker: BinDep,
    cross: BinDep,
    ckb_cli: BinDep,
}

impl Checker {
    pub fn build(ckb_cli_bin: &str) -> Result<Self> {
        let [cargo, docker, cross, ckb_cli] = [
            ("cargo", "version", None, None),
            ("docker", "version", None, None),
            ("cross-util", "--version", None, None),
            (
                ckb_cli_bin,
                "--version",
                Some("ckb-cli"),
                Some(REQUIRED_CKB_CLI_VERSION),
            ),
        ]
        .map(|(program, arg, version_prefix, required_version)| {
            BinDep::build(program, arg, version_prefix, required_version)
        });
        Ok(Checker {
            cargo: cargo?,
            docker: docker?,
            cross: cross?,
            ckb_cli: ckb_cli?,
        })
    }

    pub fn check_ckb_cli(&self) -> Result<()> {
        let ckb_cli_dep = &self.ckb_cli;
        if !ckb_cli_dep.installed {
            bail!("Can't find ckb-cli");
        }
        if !ckb_cli_dep.meet_required_version() {
            match ckb_cli_dep.version {
                Some(ref version) => {
                    bail!(
                        "Find ckb-cli {} (required {})",
                        version,
                        REQUIRED_CKB_CLI_VERSION
                    );
                }
                None => {
                    bail!(
                        "Find ckb-cli (unknown version) (required {})",
                        REQUIRED_CKB_CLI_VERSION
                    );
                }
            }
        }
        Ok(())
    }

    pub fn print_report(&self) {
        println!("------------------------------");
        for (bin_dep, help_message) in [&self.cargo, &self.docker, &self.cross].into_iter().zip(
            [
                "Please install rust (https://www.rust-lang.org/tools/install)",
                "Please install docker",
                "Please install cross (https://github.com/cross-rs/cross)",
            ]
            .into_iter(),
        ) {
            if bin_dep.installed {
                println!("{:10} installed", bin_dep.program);
            } else {
                println!("{:10} not found - {}", bin_dep.program, help_message);
            }
        }

        let ckb_cli_dep = &self.ckb_cli;
        if ckb_cli_dep.installed {
            match &ckb_cli_dep.version {
                Some(v) => {
                    println!(
                        "{:10} installed {} (required {})",
                        ckb_cli_dep.program, v, REQUIRED_CKB_CLI_VERSION
                    );
                }
                None => {
                    warn!(
                        "{:10} installed (unknown version) - The deployment feature is disabled",
                        ckb_cli_dep.program
                    );
                }
            }
        } else {
            warn!(
                "{:10} not found - The deployment feature is disabled",
                ckb_cli_dep.program
            );
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
