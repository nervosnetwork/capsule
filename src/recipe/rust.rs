use crate::config::Contract;
use crate::config_manipulate::{append_cargo_workspace_member, Document};
use crate::generator::{CreateContract, TEMPLATES};
use crate::project_context::{
    read_config_file, write_config_file, BuildConfig, BuildEnv, Context, CARGO_CONFIG_FILE,
    CONTRACTS_DIR,
};
use crate::recipe::Recipe;
use crate::signal::Signal;
use crate::util::docker::DockerCommand;
use anyhow::{anyhow, Result};
use tera;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub const DOCKER_IMAGE: &str = "jjy0/ckb-capsule-recipe-rust:2020-9-28";
const RUST_TARGET: &str = "riscv64imac-unknown-none-elf";
const CARGO_CONFIG_PATH: &str = ".cargo/config";
const BASE_RUSTFLAGS: &str =
    "-Z pre-link-arg=-zseparate-code -Z pre-link-arg=-zseparate-loadable-segments";
const RELEASE_RUSTFLAGS: &str = "-C link-arg=-s";
const ALWAYS_DEBUG_RUSTFLAGS: &str = "--cfg=debug_assertions";

pub struct Rust {
    context: Context,
}

impl Rust {
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    fn contract_path(&self, name: &str) -> PathBuf {
        let mut path = self.context.contracts_path();
        path.push(&name);
        path
    }

    fn contract_relative_path(&self, name: &str) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(CONTRACTS_DIR);
        path.push(name);
        path
    }

    fn has_cargo_config(&self, name: &str) -> bool {
        let mut contract_path = self.contract_path(name);
        contract_path.push(CARGO_CONFIG_PATH);
        contract_path.exists()
    }

    /// inject rustflags on release build unless project has cargo config
    fn injection_rustflags(&self, config: BuildConfig, name: &str) -> String {
        let has_cargo_config = self.has_cargo_config(name);
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

    fn rewrite_config_for_new_contract(&self, name: &str) -> Result<()> {
        // rewrite config
        {
            println!("Rewrite Cargo.toml");
            let mut cargo_path = self.context.workspace_dir()?;
            let workspace_member = if Some(Some(CONTRACTS_DIR))
                == self
                    .context
                    .config
                    .rust
                    .workspace_dir
                    .as_ref()
                    .map(|dir| dir.to_str())
            {
                name.to_string()
            } else {
                format!("{}/{}", CONTRACTS_DIR, name)
            };
            cargo_path.push(CARGO_CONFIG_FILE);
            let config_content = read_config_file(&cargo_path)?;
            let mut doc = config_content.parse::<Document>()?;
            append_cargo_workspace_member(&mut doc, workspace_member)?;
            write_config_file(&cargo_path, doc.to_string())?;
        }
        Ok(())
    }

    fn docker_image(&self) -> String {
        self.context
            .config
            .rust
            .docker_image
            .clone()
            .unwrap_or(DOCKER_IMAGE.to_string())
    }

    fn cargo_cmd(&self) -> String {
        let mut cargo_cmd = "cargo".to_string();
        if let Some(toolchain) = self.context.config.rust.toolchain.as_ref() {
            cargo_cmd.push_str(&format!(" +{}", toolchain));
        }
        cargo_cmd
    }
}

impl Recipe for Rust {
    fn exists(&self, name: &str) -> bool {
        self.contract_path(name).exists()
    }

    fn create_contract(
        &self,
        contract: &Contract,
        rewrite_config: bool,
        signal: &Signal,
    ) -> Result<()> {
        let name = &contract.name;
        println!("New contract {:?}", &name);
        let path = self.context.contracts_path();
        let context = tera::Context::from_serialize(&CreateContract { name: name.clone() })?;
        // generate contract
        let cmd = DockerCommand::with_config(
            self.docker_image(),
            path.to_str().expect("str").to_string(),
            &HashMap::new(),
        )
        .fix_dir_permission(name.clone());
        cmd.run(
            format!("{} new {} --vcs none", self.cargo_cmd(), name),
            signal,
        )?;
        let mut contract_path = PathBuf::new();
        contract_path.push(path);
        contract_path.push(name);
        // initialize contract code
        for (f, template_name) in &[
            ("src/main.rs", None),
            ("src/error.rs", None),
            ("src/entry.rs", None),
            ("Cargo.toml", Some("Cargo-manifest.toml")),
        ] {
            let template_path = format!("rust/contract/{}", template_name.unwrap_or(f));
            let content = TEMPLATES.render(&template_path, &context)?;
            let mut file_path = contract_path.clone();
            file_path.push(f);
            fs::write(file_path, content)?;
        }

        if rewrite_config {
            self.rewrite_config_for_new_contract(&contract.name)?;
        }
        Ok(())
    }

    /// run command in build image
    fn run(
        &self,
        contract: &Contract,
        build_cmd: String,
        signal: &Signal,
        custom_env: &HashMap<String, String>,
    ) -> Result<()> {
        let project_path = self.context.project_path.to_str().expect("path");
        let contract_relative_path = self.contract_relative_path(&contract.name);
        let cmd = DockerCommand::with_context(
            &self.context,
            self.docker_image(),
            project_path.to_string(),
            custom_env,
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
    fn run_build(
        &self,
        contract: &Contract,
        config: BuildConfig,
        signal: &Signal,
        custom_env: &HashMap<String, String>,
    ) -> Result<()> {
        // docker cargo build
        let mut rel_bin_path = PathBuf::new();
        let (bin_dir_prefix, build_cmd_opt) = match config.build_env {
            BuildEnv::Debug => ("debug", ""),
            BuildEnv::Release => ("release", "--release"),
        };
        rel_bin_path.push(format!(
            "target/{}/{}/{}",
            RUST_TARGET, bin_dir_prefix, &contract.name
        ));
        let mut container_bin_path = PathBuf::new();
        container_bin_path.push("/code");
        if let Some(workspace_dir) = self.context.config.rust.workspace_dir.as_ref() {
            container_bin_path.push(workspace_dir);
        }
        container_bin_path.push(&rel_bin_path);

        // run build command
        let build_cmd = format!(
            "{rustflags} {cargo_cmd} build --target {rust_target} {build_env} && \
         ckb-binary-patcher -i {contract_bin} -o {contract_bin}",
            cargo_cmd = self.cargo_cmd(),
            rustflags = self.injection_rustflags(config, &contract.name),
            rust_target = RUST_TARGET,
            contract_bin = container_bin_path.to_str().expect("bin"),
            build_env = build_cmd_opt
        );
        self.run(contract, build_cmd, signal, custom_env)?;

        // copy to build dir
        let mut project_bin_path = self.context.workspace_dir()?;
        project_bin_path.push(&rel_bin_path);
        if !project_bin_path.exists() {
            return Err(anyhow!("can't find contract binary from path {:?}, please set `workspace_dir` in capsule.toml", project_bin_path));
        }
        let mut target_path = self.context.contracts_build_path(config.build_env);
        // make sure the dir is exist
        fs::create_dir_all(&target_path)?;
        target_path.push(&contract.name);
        fs::copy(project_bin_path, target_path)?;
        Ok(())
    }

    /// clean contract
    fn clean(&self, contracts: &[Contract], signal: &Signal) -> Result<()> {
        // cargo clean
        let clean_cmd = format!(
            "{cargo_cmd} clean --target {rust_target}",
            cargo_cmd = self.cargo_cmd(),
            rust_target = RUST_TARGET,
        );

        for c in contracts {
            self.run(c, clean_cmd.clone(), signal, &HashMap::new())?;

            // remove binary
            for build_env in &[BuildEnv::Debug, BuildEnv::Release] {
                let mut target_path = self.context.contracts_build_path(*build_env);
                // make sure the dir is exist
                fs::create_dir_all(&target_path)?;
                target_path.push(&c.name);
                if target_path.exists() {
                    fs::remove_file(&target_path)?;
                }
            }
        }
        Ok(())
    }
}
