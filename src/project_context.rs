/// Project Context
use crate::config::{Config, Deployment};
use anyhow::{anyhow, Result};
use log::error;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub const CONTRACTS_DIR: &str = "contracts";
const CONTRACTS_BUILD_DIR: &str = "build";
const MIGRATIONS_DIR: &str = "migrations";
pub const CONFIG_FILE: &str = "capsule.toml";
pub const CARGO_CONFIG_FILE: &str = "Cargo.toml";

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
pub struct BuildConfig {
    pub build_env: BuildEnv,
    pub always_debug: bool,
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

#[derive(Clone)]
pub struct Context {
    pub project_path: PathBuf,
    pub config: Config,
}

impl Context {
    pub fn load() -> Result<Context> {
        Self::load_from_path(env::current_dir()?)
    }

    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Context> {
        let mut project_path = PathBuf::new();
        project_path.push(&path);
        let content = {
            let mut config_path = project_path.clone();
            config_path.push(CONFIG_FILE);
            read_config_file(config_path)?
        };
        let config: Config = toml::from_slice(content.as_bytes()).expect("parse config");
        Ok(Context {
            config,
            project_path,
        })
    }

    pub fn contracts_path(&self) -> PathBuf {
        let mut path = self.project_path.clone();
        path.push(CONTRACTS_DIR);
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
        match toml::from_slice(&fs::read(&path)?) {
            Ok(deployment) => Ok(deployment),
            Err(err) => {
                error!("failed to parse {:?}", path);
                Err(err.into())
            }
        }
    }
}

pub fn read_config_file<P: AsRef<Path> + std::fmt::Debug>(path: P) -> Result<String> {
    match fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(err) => Err(anyhow!(
            "Can't found {:?}, current directory is not a project. error: {:?}",
            path,
            err
        )),
    }
}

pub fn write_config_file<P: AsRef<Path>>(path: P, content: String) -> Result<()> {
    fs::write(path, content)?;
    Ok(())
}
