use crate::config::Contract;
use crate::project_context::{BuildEnv, Context};
use crate::signal::Signal;
use crate::util::DockerCommand;
use anyhow::Result;

use std::fs;
use std::path::PathBuf;

pub const DOCKER_IMAGE: &str = "jjy0/ckb-capsule-recipe-rust:2020-5-9";
const RUST_TARGET: &str = "riscv64imac-unknown-none-elf";
const RUSTFLAGS: &str = "-C link-arg=-s";
const CARGO_CONFIG_PATH: &str = ".cargo/config";

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
    fn injection_rustflags(&self, build_env: BuildEnv) -> &str {
        let has_cargo_config = self.has_cargo_config();
        match build_env {
            BuildEnv::Debug => "",
            BuildEnv::Release if has_cargo_config => "",
            BuildEnv::Release => RUSTFLAGS,
        }
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
        let build_cmd = format!(
            "cd /code && \
         RUSTFLAGS='{rustflags}' cargo build --target {rust_target} {build_env} && \
         ckb-binary-patcher -i {contract_bin} -o {contract_bin}",
            rustflags = self.injection_rustflags(build_env),
            rust_target = RUST_TARGET,
            contract_bin = bin_path.to_str().expect("bin"),
            build_env = build_cmd_opt
        );
        let contract_source_path = contract_source_path.to_str().expect("path");
        let cmd = DockerCommand::with_context(
            self.context,
            DOCKER_IMAGE.to_string(),
            contract_source_path.to_string(),
        )
        .fix_dir_permission("target".to_string())
        .fix_dir_permission("Cargo.lock".to_string());
        cmd.run(build_cmd, &signal)?;
        // copy to build dir
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
