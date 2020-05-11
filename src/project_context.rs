/// Project Context
use crate::config::{Config, Deployment};
use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::io::ErrorKind as IOErrorKind;
use std::path::{Path, PathBuf};

const CONTRACTS_DIR: &str = "contracts";
const CONTRACTS_BUILD_DIR: &str = "build";
const MIGRATIONS_DIR: &str = "migrations";
const RELEASE_PREFIX: &str = "release";
const DEV_PREFIX: &str = "dev";

#[derive(Debug)]
pub enum Env {
    Dev,
    Release,
}

pub struct Context {
    pub project_path: PathBuf,
    pub config: Config,
    pub env: Env,
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

    pub fn migrations_path(&self) -> PathBuf {
        let mut path = self.project_path.clone();
        path.push(MIGRATIONS_DIR);
        let prefix = match self.env {
            Env::Release => RELEASE_PREFIX,
            Env::Dev => DEV_PREFIX,
        };
        path.push(prefix);
        path
    }

    pub fn load_deployment(&self) -> Result<Deployment> {
        let mut path = self.project_path.clone();
        path.push(&self.config.deployment);
        let deployment: Deployment = toml::from_slice(&fs::read(path)?)?;
        Ok(deployment)
    }
}

pub fn load_project_context(env: Env) -> Result<Context> {
    const CONFIG_NAME: &str = "capsule.toml";

    let mut project_path = PathBuf::new();
    project_path.push(env::current_dir()?);
    let mut path = project_path.clone();
    path.push(CONFIG_NAME);
    match fs::read(path) {
        Ok(content) => {
            let config: Config = toml::from_slice(&content)?;
            Ok(Context {
                config,
                project_path,
                env,
            })
        }
        Err(err) if err.kind() == IOErrorKind::NotFound => Err(anyhow!(
            "Can't found {}, not in the project directory",
            CONFIG_NAME
        )),
        Err(err) => Err(err.into()),
    }
}
