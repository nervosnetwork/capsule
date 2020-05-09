use crate::config::Contract;
use crate::project_context::Context;
use crate::util::build_docker_cmd;
use anyhow::Result;

use std::fs;
use std::path::PathBuf;
use std::process::exit;

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
         ckb-binary-patcher -i {contract_bin} -o {contract_bin}; \
         EXITCODE=$?;chown -R $UID:$GID target; exit $EXITCODE",
            rust_flags = RUST_FLAGS,
            rust_target = RUST_TARGET,
            contract_bin = bin_path.to_str().expect("bin")
        );
        let exit_code = build_docker_cmd(
            &build_cmd,
            contract_source_path.to_str().expect("pwd"),
            DOCKER_IMAGE,
        )?
        .spawn()?
        .wait()?;
        if !exit_code.success() {
            exit(exit_code.code().unwrap_or(-1));
        }
        // copy to build dir
        let mut target_path = self.context.contracts_build_path();
        target_path.push(&self.contract.name);
        let mut contract_bin_path = contract_source_path.clone();
        contract_bin_path.push(bin_path);
        fs::copy(contract_bin_path, target_path)?;
        Ok(())
    }
}
