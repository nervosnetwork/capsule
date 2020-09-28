use crate::config::Contract;
use crate::config_manipulate::{append_cargo_workspace_member, append_contract, Document};
use crate::generator::{CreateContract, TEMPLATES};
use crate::project_context::{
    read_config_file, write_config_file, BuildConfig, BuildEnv, Context, CARGO_CONFIG_FILE,
    CONFIG_FILE, CONTRACTS_DIR,
};
use crate::recipe::Recipe;
use crate::signal::Signal;
use crate::util::DockerCommand;
use crate::TemplateType;
use anyhow::{anyhow, Result};
use tera;

use std::fs;
use std::path::PathBuf;

pub const DOCKER_IMAGE: &str = "jjy0/ckb-capsule-recipe-rust:2020-9-28";
const RUST_TARGET: &str = "riscv64imac-unknown-none-elf";
const CARGO_CONFIG_PATH: &str = ".cargo/config";
const BASE_RUSTFLAGS: &str =
    "-Z pre-link-arg=-zseparate-code -Z pre-link-arg=-zseparate-loadable-segments";
const RELEASE_RUSTFLAGS: &str = "-C link-arg=-s";
const ALWAYS_DEBUG_RUSTFLAGS: &str = "--cfg=debug_assertions";

pub struct Rust<'a> {
    context: &'a Context,
    contract: &'a Contract,
}

impl<'a> Rust<'a> {
    fn has_cargo_config(&self) -> bool {
        let mut contract_path = self.context.contract_path(&self.contract.name);
        contract_path.push(CARGO_CONFIG_PATH);
        contract_path.exists()
    }

    /// inject rustflags on release build unless project has cargo config
    fn injection_rustflags(&self, config: BuildConfig) -> String {
        let has_cargo_config = self.has_cargo_config();
        match config.build_env {
            _ if has_cargo_config => "".to_string(),
            BuildEnv::Debug => format!("RUSTFLAGS=\"{}\"", BASE_RUSTFLAGS.to_string()),
            BuildEnv::Release => {
                if config.always_debug {
                    format!(
                        "RUSTFLAGS=\"{} {} {}\"",
                        BASE_RUSTFLAGS, RELEASE_RUSTFLAGS, ALWAYS_DEBUG_RUSTFLAGS
                    )
                } else {
                    format!("RUSTFLAGS=\"{} {}\"", BASE_RUSTFLAGS, RELEASE_RUSTFLAGS)
                }
            }
        }
    }

    fn rewrite_config_for_new_contract(&self) -> Result<()> {
        let name = &self.contract.name;
        // rewrite config
        {
            println!("Rewrite Cargo.toml");
            let mut cargo_path = self.context.project_path.clone();
            cargo_path.push(CARGO_CONFIG_FILE);
            let config_content = read_config_file(&cargo_path)?;
            let mut doc = config_content.parse::<Document>()?;
            append_cargo_workspace_member(&mut doc, format!("{}/{}", CONTRACTS_DIR, name))?;
            write_config_file(&cargo_path, doc.to_string())?;
        }
        {
            println!("Rewrite capsule.toml");
            let mut config_path = self.context.project_path.clone();
            config_path.push(CONFIG_FILE);
            let config_content = read_config_file(&config_path)?;
            let mut doc = config_content.parse::<Document>()?;
            append_contract(&mut doc, name.to_string(), TemplateType::Rust)?;
            write_config_file(&config_path, doc.to_string())?;
        }
        Ok(())
    }
}

impl<'a> Recipe<'a> for Rust<'a> {
    fn new(context: &'a Context, contract: &'a Contract) -> Self {
        Self { context, contract }
    }

    fn create_contract(&self, rewrite_config: bool, signal: &Signal) -> Result<()> {
        let name = &self.contract.name;
        println!("New contract {:?}", &name);
        let path = self.context.contracts_path();
        let context = tera::Context::from_serialize(&CreateContract { name: name.clone() })?;
        // generate contract
        let cmd = DockerCommand::with_config(
            DOCKER_IMAGE.to_string(),
            path.to_str().expect("str").to_string(),
        )
        .fix_dir_permission(name.clone());
        cmd.run(format!("cargo new {} --vcs none", name), signal)?;
        let mut contract_path = PathBuf::new();
        contract_path.push(path);
        contract_path.push(name);
        // initialize contract code
        for f in &["src/main.rs", "src/error.rs", "src/entry.rs", "Cargo.toml"] {
            let template_path = format!("rust/contract/{}", f);
            let content = TEMPLATES.render(&template_path, &context)?;
            let mut file_path = contract_path.clone();
            file_path.push(f);
            fs::write(file_path, content)?;
        }

        if rewrite_config {
            self.rewrite_config_for_new_contract()?;
        }
        Ok(())
    }

    /// run command in build image
    fn run(&self, build_cmd: String, signal: &Signal) -> Result<()> {
        let project_path = self.context.project_path.to_str().expect("path");
        let contract_relative_path = self.context.contract_relative_path(&self.contract.name);
        let cmd = DockerCommand::with_context(
            self.context,
            DOCKER_IMAGE.to_string(),
            project_path.to_string(),
        )
        .workdir(format!(
            "/code/{}",
            contract_relative_path.to_str().expect("path")
        ))
        .fix_dir_permission("/code/target".to_string())
        .fix_dir_permission("/code/Cargo.lock".to_string());
        cmd.run(build_cmd, &signal)?;
        Ok(())
    }

    /// build contract
    fn run_build(&self, config: BuildConfig, signal: &Signal) -> Result<()> {
        // docker cargo build
        let mut rel_bin_path = PathBuf::new();
        let (bin_dir_prefix, build_cmd_opt) = match config.build_env {
            BuildEnv::Debug => ("debug", ""),
            BuildEnv::Release => ("release", "--release"),
        };
        rel_bin_path.push(format!(
            "target/{}/{}/{}",
            RUST_TARGET, bin_dir_prefix, &self.contract.name
        ));
        let mut container_bin_path = PathBuf::new();
        container_bin_path.push("/code");
        if let Some(workspace_dir) = self.context.config.workspace_dir.as_ref() {
            container_bin_path.push(workspace_dir);
        }
        container_bin_path.push(&rel_bin_path);

        // run build command
        let build_cmd = format!(
            "{rustflags} cargo build --target {rust_target} {build_env} && \
         ckb-binary-patcher -i {contract_bin} -o {contract_bin}",
            rustflags = self.injection_rustflags(config),
            rust_target = RUST_TARGET,
            contract_bin = container_bin_path.to_str().expect("bin"),
            build_env = build_cmd_opt
        );
        self.run(build_cmd, signal)?;

        // copy to build dir
        let mut project_bin_path = self.context.project_path.clone();
        if let Some(workspace_dir) = self.context.config.workspace_dir.as_ref() {
            project_bin_path.push(workspace_dir);
        }
        project_bin_path.push(&rel_bin_path);
        if !project_bin_path.exists() {
            return Err(anyhow!("can't find contract binary from path {:?}, please set `workspace_dir` in capsule.toml", project_bin_path));
        }
        let mut target_path = self.context.contracts_build_path(config.build_env);
        // make sure the dir is exist
        fs::create_dir_all(&target_path)?;
        target_path.push(&self.contract.name);
        fs::copy(project_bin_path, target_path)?;
        Ok(())
    }

    /// clean contract
    fn clean(&self, signal: &Signal) -> Result<()> {
        // cargo clean
        let clean_cmd = format!(
            "cargo clean --target {rust_target}",
            rust_target = RUST_TARGET,
        );
        self.run(clean_cmd, signal)?;

        // remove binary
        for build_env in &[BuildEnv::Debug, BuildEnv::Release] {
            let mut target_path = self.context.contracts_build_path(*build_env);
            // make sure the dir is exist
            fs::create_dir_all(&target_path)?;
            target_path.push(&self.contract.name);
            if target_path.exists() {
                fs::remove_file(&target_path)?;
            }
        }
        Ok(())
    }
}
