use crate::config::Contract;
use crate::project_context::{BuildEnv, Context};
use crate::signal::Signal;
use crate::util::DockerCommand;
use anyhow::Result;

use std::fs;
use std::path::PathBuf;

pub const DOCKER_IMAGE: &str = "jjy0/ckb-capsule-recipe-rust:2020-6-2";
const RUST_TARGET: &str = "riscv64imac-unknown-none-elf";
const CARGO_CONFIG_PATH: &str = ".cargo/config";
const BASE_RUSTFLAGS: &str =
    "-Z pre-link-arg=-zseparate-code -Z pre-link-arg=-zseparate-loadable-segments";
const RELEASE_RUSTFLAGS: &str = "-C link-arg=-s";

pub struct Rust<'a> {
    context: &'a Context,
    contract: &'a Contract,
}

impl<'a> Rust<'a> {
    pub fn new(context: &'a Context, contract: &'a Contract) -> Self {
        Self { context, contract }
    }

    fn has_cargo_config(&self) -> bool {
        let mut contract_path = self.context.contract_path(&self.contract.name);
        contract_path.push(CARGO_CONFIG_PATH);
        contract_path.exists()
    }

    /// inject rustflags on release build unless project has cargo config
    fn injection_rustflags(&self, build_env: BuildEnv) -> String {
        let has_cargo_config = self.has_cargo_config();
        match build_env {
            _ if has_cargo_config => "".to_string(),
            BuildEnv::Debug => format!("RUSTFLAGS=\"{}\"", BASE_RUSTFLAGS.to_string()),
            BuildEnv::Release => format!("RUSTFLAGS=\"{} {}\"", BASE_RUSTFLAGS, RELEASE_RUSTFLAGS),
        }
    }

    /// run command in build image
    pub fn run(&self, build_cmd: String, signal: &Signal) -> Result<()> {
        let project_path = self.context.project_path.to_str().expect("path");
        let contract_relative_path = self.context.contract_relative_path(&self.contract.name);
        let cmd = DockerCommand::with_context(
            self.context,
            DOCKER_IMAGE.to_string(),
            project_path.to_string(),
        )
        .workdir(format!("/code/{}", contract_relative_path.to_str().expect("path")))
        .fix_dir_permission("target".to_string())
        .fix_dir_permission("Cargo.lock".to_string());
        cmd.run(build_cmd, &signal)?;
        Ok(())
    }

    /// build contract
    pub fn run_build(&self, build_env: BuildEnv, signal: &Signal) -> Result<()> {
        let contract_source_path = self.context.contract_path(&self.contract.name);

        // docker cargo build
        let mut bin_path = PathBuf::new();
        let (bin_dir_prefix, build_cmd_opt) = match build_env {
            BuildEnv::Debug => ("debug", ""),
            BuildEnv::Release => ("release", "--release"),
        };
        bin_path.push(format!(
            "target/{}/{}/{}",
            RUST_TARGET, bin_dir_prefix, &self.contract.name
        ));

        // run build command
        let build_cmd = format!(
            "{rustflags} cargo build --target {rust_target} {build_env} && \
         ckb-binary-patcher -i {contract_bin} -o {contract_bin}",
            rustflags = self.injection_rustflags(build_env),
            rust_target = RUST_TARGET,
            contract_bin = bin_path.to_str().expect("bin"),
            build_env = build_cmd_opt
        );
        self.run(build_cmd, signal)?;

        // copy to build dir
        let contract_source_path = contract_source_path.to_str().expect("path");
        let mut target_path = self.context.contracts_build_path(build_env);
        // make sure the dir is exist
        fs::create_dir_all(&target_path)?;
        target_path.push(&self.contract.name);
        let mut contract_bin_path = PathBuf::new();
        contract_bin_path.push(contract_source_path);
        contract_bin_path.push(bin_path);
        fs::copy(contract_bin_path, target_path)?;
        Ok(())
    }
}
