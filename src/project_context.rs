use crate::config::Config;
use anyhow::Result;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const CONTRACTS_DIR: &str = "contracts";
const CONTRACTS_BUILD_DIR: &str = "build";

pub struct Context {
    pub project_path: PathBuf,
    pub config: Config,
}

impl Context {
    pub fn contracts_path(&self) -> PathBuf {
        let mut path = self.project_path.clone();
        path.push(CONTRACTS_DIR);
        path
    }

    pub fn contract_path<P: AsRef<Path>>(&self, contract_name: P) -> PathBuf {
        let mut path = self.contracts_path();
        path.push(contract_name);
        path
    }

    pub fn contracts_build_path(&self) -> PathBuf {
        let mut path = self.project_path.clone();
        path.push(CONTRACTS_BUILD_DIR);
        path
    }
}

pub fn load_project_context() -> Result<Context> {
    const CONFIG_NAME: &str = "capsule.toml";

    let mut project_path = PathBuf::new();
    project_path.push(env::current_dir()?);
    let mut path = project_path.clone();
    path.push(CONFIG_NAME);
    let config: Config = toml::from_slice(&fs::read(path)?)?;
    Ok(Context {
        config,
        project_path,
    })
}
