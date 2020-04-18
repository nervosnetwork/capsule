use crate::config::Contract;
use crate::project_context::Context;
use anyhow::Result;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{exit, Command};

const DOCKER_IMAGE: &str = "jjy0/ckb-riscv-rust-toolchain:2020-2-6";
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
        let old_dir = env::current_dir()?;
        let contract_path = self.context.contract_path(&self.contract.name);
        // set current dir to contract path
        env::set_current_dir(&contract_path)?;
        // docker cargo build
        let mut contract_bin_path = PathBuf::new();
        contract_bin_path.push(format!(
            "target/{}/release/{}",
            RUST_TARGET, &self.contract.name
        ));
        let build_cmd = format!(
            "docker run \
         -eOWNER=`id -u`:`id -g` \
         --rm -v `pwd`:/code {docker_image} \
         bash -c \
         \"cd /code && \
         RUSTFLAGS='{rust_flags}' cargo build --target {rust_target} --release && \
         chown -R \\$OWNER target && \
         ckb-binary-patcher -i {contract_bin} -o {contract_bin}\"",
            docker_image = DOCKER_IMAGE,
            rust_flags = RUST_FLAGS,
            rust_target = RUST_TARGET,
            contract_bin = contract_bin_path.to_str().expect("path")
        );
        println!("build cmd : {}", build_cmd);
        let exit_code = Command::new("bash")
            .arg("-c")
            .arg(build_cmd)
            .spawn()?
            .wait()?;
        if !exit_code.success() {
            exit(exit_code.code().unwrap_or(-1));
        }
        // copy to build dir
        let mut target_path = self.context.contracts_build_path();
        target_path.push(&self.contract.name);
        fs::copy(contract_bin_path, target_path)?;
        // set current dir back
        env::set_current_dir(old_dir)?;
        Ok(())
    }
}
