use crate::config::Contract;
use crate::config_manipulate::{append_cargo_workspace_member, Document};
use crate::generator::{CreateContract, TEMPLATES};
use crate::project_context::{
    read_config_file, write_config_file, BuildConfig, BuildEnv, Context, CARGO_CONFIG_FILE,
    CONTRACTS_DIR,
};
use crate::recipe::Recipe;
use crate::signal::Signal;
use anyhow::{bail, Result};
use path_macro::path;
use tera;
use xshell::{cmd, Shell};

use std::fs;
use std::path::PathBuf;

pub const DOCKER_IMAGE: &str = "thewawar/ckb-capsule:2022-08-01";
const RUST_TARGET: &str = "riscv64imac-unknown-none-elf";

pub struct Rust {
    context: Context,
}

impl Rust {
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    fn contract_path(&self, name: &str) -> PathBuf {
        let mut path = self.context.contracts_path();
        path.push(name);
        path
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
}

impl Recipe for Rust {
    fn exists(&self, name: &str) -> bool {
        self.contract_path(name).exists()
    }

    fn create_contract(
        &self,
        contract: &Contract,
        rewrite_config: bool,
        _signal: &Signal,
        _docker_env_file: String,
    ) -> Result<()> {
        let name = &contract.name;
        println!("New contract {:?}", &name);
        let path = self.context.contracts_path();
        let context = tera::Context::from_serialize(&CreateContract { name: name.clone() })?;
        // generate contract
        let mut cmd = std::process::Command::new("cargo");
        let output = cmd
            .args(["new", name, "--vcs", "none"])
            .current_dir(&path)
            .output()?;
        if !output.status.success() {
            bail!("failed to generate tests, status: {}", output.status);
        }
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
    fn run(&self, _contract: &Contract, _build_cmd: String, _signal: &Signal) -> Result<()> {
        bail!("run command is no longer supported for rust contracts, just use cargo or cross directly")
    }

    /// build contract
    fn run_build(
        &self,
        contract: &Contract,
        config: BuildConfig,
        _signal: &Signal,
        build_args_opt: Option<Vec<String>>,
    ) -> Result<()> {
        // docker cargo build
        let (debug_or_release, build_cmd_opt) = match config.build_env {
            BuildEnv::Debug => ("debug", None),
            BuildEnv::Release => ("release", Some("--release")),
        };
        let bin_path = path!("target" / RUST_TARGET / debug_or_release / &contract.name);

        let sh = Shell::new()?;
        sh.change_dir(self.context.workspace_dir()?);

        // TODO: support host network.
        if self.context.use_docker_host {
            eprintln!("warn: host network is not supported in cross yet; as an alternative, run `cargo fetch` first");
        }
        let _debug_env_guard = if config.always_debug {
            println!(r#"RUSTFLAGS="--cfg debug_assertions""#);
            Some(sh.push_env("RUSTFLAGS", "--cfg debug_assertions"))
        } else {
            None
        };

        let pkg = &contract.name;
        let args = build_args_opt.into_iter().flatten();
        cmd!(sh, "cross build -p {pkg} {build_cmd_opt...} {args...}").run()?;

        // copy to build dir
        let mut target_path = self.context.contracts_build_path(config.build_env);
        // make sure the dir is exist
        sh.create_dir(&target_path)?;
        target_path.push(&contract.name);
        sh.copy_file(bin_path, target_path)?;
        Ok(())
    }

    /// clean contract
    fn clean(&self, contracts: &[Contract], _signal: &Signal) -> Result<()> {
        let sh = Shell::new()?;
        // Do we want `cargo clean -p contract1 -p contract2 ...`?
        cmd!(sh, "cargo clean").run()?;
        let build_dir = self.context.contracts_build_dir();
        for c in contracts {
            sh.remove_path(path!(build_dir / "debug" / &c.name))?;
            sh.remove_path(path!(build_dir / "release" / &c.name))?;
        }

        Ok(())
    }
}
