/// Project Context
use crate::config::{Config, Deployment};
use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::io::ErrorKind as IOErrorKind;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const CONTRACTS_DIR: &str = "contracts";
const CONTRACTS_BUILD_DIR: &str = "build";
const MIGRATIONS_DIR: &str = "migrations";
const CONFIG_NAME: &str = "capsule.toml";

#[derive(Debug, Copy, Clone)]
pub enum BuildEnv {
    Debug,
    Release,
}

impl FromStr for BuildEnv {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(BuildEnv::Debug),
            "release" => Ok(BuildEnv::Release),
            _ => Err("no match"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DeployEnv {
    Dev,
    Production,
}

impl FromStr for DeployEnv {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dev" => Ok(DeployEnv::Dev),
            "production" => Ok(DeployEnv::Production),
            _ => Err("no match"),
        }
    }
}

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

    pub fn contract_relative_path<P: AsRef<Path>>(&self, contract_name: P) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(CONTRACTS_DIR);
        path.push(contract_name);
        path
    }

    pub fn contracts_build_path(&self, env: BuildEnv) -> PathBuf {
        let mut path = self.project_path.clone();
        path.push(CONTRACTS_BUILD_DIR);
        let prefix = match env {
            BuildEnv::Debug => "debug",
            BuildEnv::Release => "release",
        };
        path.push(prefix);
        path
    }

    pub fn migrations_path(&self, env: DeployEnv) -> PathBuf {
        let mut path = self.project_path.clone();
        path.push(MIGRATIONS_DIR);
        let prefix = match env {
            DeployEnv::Production => "production",
            DeployEnv::Dev => "dev",
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

pub fn read_config_file() -> Result<String> {
    let mut project_path = PathBuf::new();
    project_path.push(env::current_dir()?);
    let mut path = project_path.clone();
    path.push(CONFIG_NAME);
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(err) if err.kind() == IOErrorKind::NotFound => Err(anyhow!(
            "Can't found {}, current directory is not a project",
            CONFIG_NAME
        )),
        Err(err) => Err(err.into()),
    }
}

pub fn write_config_file(content: String) -> Result<()> {
    let mut project_path = PathBuf::new();
    project_path.push(env::current_dir()?);
    let mut path = project_path.clone();
    path.push(CONFIG_NAME);
    fs::write(path, content)?;
    Ok(())
}

pub fn load_project_context() -> Result<Context> {
    let content = read_config_file()?;
    let config: Config = toml::from_slice(content.as_bytes())?;
    let mut project_path = PathBuf::new();
    project_path.push(env::current_dir()?);
    Ok(Context {
        config,
        project_path,
    })
}
