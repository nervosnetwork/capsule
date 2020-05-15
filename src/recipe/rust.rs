use crate::config::Contract;
use crate::project_context::Context;
use crate::util::DockerCommand;
use anyhow::Result;

use std::fs;
use std::path::PathBuf;

pub const DOCKER_IMAGE: &str = "jjy0/ckb-capsule-recipe-rust:2020-5-9";
const RUST_TARGET: &str = "riscv64imac-unknown-none-elf";
const RUST_FLAGS: &str = "-C link-arg=-s";

pub struct Rust<'a> {
    context: &'a Context,
    contract: &'a Contract,
}

impl<'a> Rust<'a> {
    pub fn new(context: &'a Context, contract: &'a Contract) -> Self {
        Self { context, contract }
    }

    // build contract
    pub fn run_build(&self) -> Result<()> {
        let contract_source_path = self.context.contract_path(&self.contract.name);
        // docker cargo build
        let mut bin_path = PathBuf::new();
        bin_path.push(format!(
            "target/{}/release/{}",
            RUST_TARGET, &self.contract.name
        ));
        let build_cmd = format!(
            "cd /code && \
         RUSTFLAGS='{rust_flags}' cargo build --target {rust_target} --release && \
         ckb-binary-patcher -i {contract_bin} -o {contract_bin}",
            rust_flags = RUST_FLAGS,
            rust_target = RUST_TARGET,
            contract_bin = bin_path.to_str().expect("bin")
        );
        let contract_source_path = contract_source_path.to_str().expect("path");
        let cmd = DockerCommand::with_context(
            self.context,
            DOCKER_IMAGE.to_string(),
            contract_source_path.to_string(),
        )
        .fix_dir_permission("target".to_string());
        cmd.run(build_cmd)?;
        // copy to build dir
        let mut target_path = self.context.contracts_build_path();
        target_path.push(&self.contract.name);
        let mut contract_bin_path = PathBuf::new();
        contract_bin_path.push(contract_source_path);
        contract_bin_path.push(bin_path);
        fs::copy(contract_bin_path, target_path)?;
        Ok(())
    }
}
