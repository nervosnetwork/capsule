use crate::config::Contract;
use crate::generator::{CreateContract, TEMPLATES};
use crate::project_context::{BuildConfig, BuildEnv, Context, CONTRACTS_DIR};
use crate::recipe::Recipe;
use crate::signal::Signal;
use crate::util::cli;
use crate::util::git;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::marker::PhantomData;
use std::path::PathBuf;
use tera;

// Files

const MAKEFILE: &str = "Makefile";

// Dirs

const C_DIR_PREFIX: &str = "c";
const DEPS_DIR_PREFIX: &str = "deps";
const SRC_DIR_PREFIX: &str = "src";
const DEBUG_DIR: &str = "build/debug";
const RELEASE_DIR: &str = "build/release";

// Deps

const CKB_C_STDLIB_URL: &str = "https://github.com/nervosnetwork/ckb-c-stdlib.git";
const CKB_C_STDLIB_COMMIT: &str = "82bc1ab07572ceacd1e016488f0a1ac7725ad3c6";
const CKB_C_STDLIB_NAME: &str = "ckb-c-stdlib";

pub trait CRecipe {
    fn bin_name(name: &str) -> String;
    fn src_template() -> &'static str;
    fn build_template() -> &'static str;
}
pub struct CBin;

impl CRecipe for CBin {
    fn bin_name(name: &str) -> String {
        name.to_string()
    }

    fn src_template() -> &'static str {
        "bin/contract/example.c"
    }

    fn build_template() -> &'static str {
        "bin/contract/BUILD"
    }
}
pub struct CSharedLib;

impl CRecipe for CSharedLib {
    fn bin_name(name: &str) -> String {
        format!("{}.so", name)
    }

    fn src_template() -> &'static str {
        "sharedlib/contract/example.c"
    }

    fn build_template() -> &'static str {
        "sharedlib/contract/BUILD"
    }
}

pub struct C<R> {
    context: Context,
    phantom_data: PhantomData<R>,
}

impl<R: CRecipe> C<R> {
    pub fn new(context: Context) -> Self {
        Self {
            context,
            phantom_data: PhantomData,
        }
    }

    fn c_dir(&self) -> PathBuf {
        let mut c_dir = self.context.contracts_path();
        c_dir.push(C_DIR_PREFIX);
        c_dir
    }

    fn src_dir(&self) -> PathBuf {
        let mut src_path = self.c_dir();
        src_path.push(SRC_DIR_PREFIX);
        src_path
    }

    fn makefile_path(&self) -> PathBuf {
        let mut p = self.c_dir();
        p.push(MAKEFILE);
        p
    }

    fn setup_c_environment(&self) -> Result<()> {
        println!("Setup C environment");
        let c_dir = self.c_dir();
        if c_dir.exists() {
            return Ok(());
        }

        // Setup Dirs
        fs::create_dir(&c_dir)?;

        for prefix in &[DEPS_DIR_PREFIX, SRC_DIR_PREFIX] {
            let mut dir = c_dir.clone();
            dir.push(prefix);
            fs::create_dir(&dir)?;
        }

        // Pull deps
        let rel_path = format!(
            "{contracts}/{c}/{deps}/{name}",
            contracts = CONTRACTS_DIR,
            c = C_DIR_PREFIX,
            deps = DEPS_DIR_PREFIX,
            name = CKB_C_STDLIB_NAME
        );
        git::add_submodule(
            &self.context,
            CKB_C_STDLIB_URL,
            rel_path.as_str(),
            CKB_C_STDLIB_COMMIT,
        )?;

        // Generate files
        for f in &["Makefile"] {
            let template_path = format!("c/{}", f);
            let content = TEMPLATES.render(&template_path, &tera::Context::default())?;
            let mut file_path = c_dir.clone();
            file_path.push(f);
            fs::write(file_path, content)?;
        }

        Ok(())
    }

    fn source_name(&self, name: &str) -> String {
        format!("{}.c", name)
    }

    fn build_target(&self, build_env: BuildEnv, name: &str) -> String {
        match build_env {
            BuildEnv::Debug => format!("{}/{}", DEBUG_DIR, R::bin_name(name)),
            BuildEnv::Release => format!("{}/{}", RELEASE_DIR, R::bin_name(name)),
        }
    }
}

impl<R: CRecipe> Recipe for C<R> {
    fn exists(&self, name: &str) -> bool {
        let mut src = self.src_dir();
        src.push(self.source_name(name));
        src.exists()
    }

    fn create_contract(
        &self,
        contract: &Contract,
        rewrite_config: bool,
        _signal: &Signal,
    ) -> Result<()> {
        // setup c environment if needed
        self.setup_c_environment()?;

        // new contract
        let name = &contract.name;
        println!("New contract {:?}", &name);
        let context = tera::Context::from_serialize(&CreateContract { name: name.clone() })?;

        // initialize contract code
        let f = R::src_template();
        let template_path = format!("c/{}", f);
        let content = TEMPLATES.render(&template_path, &context)?;
        let mut src_path = self.src_dir();
        src_path.push(self.source_name(name));
        fs::write(src_path, content)?;

        if rewrite_config {
            println!("Rewrite Makefile");
            let f = R::build_template();
            let template_path = format!("c/{}", f);
            let content = TEMPLATES.render(&template_path, &context)?;
            let makefile_path = self.makefile_path();
            fs::OpenOptions::new()
                .append(true)
                .open(makefile_path)?
                .write_all(content.as_bytes())?;
        }
        Ok(())
    }

    /// run command
    /// Delegate to cli command
    fn run(
        &self,
        _contract: &Contract,
        build_cmd: String,
        signal: &Signal,
        _custom_env: &HashMap<String, String>,
    ) -> Result<()> {
        cli::run(build_cmd, self.c_dir(), signal)
    }

    /// build contract
    /// Delegate to Makefile
    fn run_build(
        &self,
        c: &Contract,
        config: BuildConfig,
        signal: &Signal,
        _custom_build_env: &HashMap<String, String>,
    ) -> Result<()> {
        let build_target = self.build_target(config.build_env, &c.name);
        let mut bin_path = self.c_dir();
        bin_path.push(&build_target);
        // make sure the bin dir is exist
        fs::create_dir_all(&bin_path.parent().ok_or(anyhow!("expect build dir"))?)?;
        self.run(
            c,
            format!("make via-docker ARGS=\"{}\"", &build_target),
            signal,
            _custom_build_env,
        )?;

        // copy to build dir
        if !bin_path.exists() {
            return Err(anyhow!(
                "can't find contract binary from path {:?}, please check Makefile"
            ));
        }
        let mut target_path = self.context.project_path.clone();
        target_path.push(&build_target);
        // make sure the target dir is exist
        fs::create_dir_all(&target_path.parent().ok_or(anyhow!("expect build dir"))?)?;
        fs::copy(bin_path, target_path)?;
        Ok(())
    }

    /// clean contract
    /// Delegate to Makefile
    fn clean(&self, _contracts: &[Contract], signal: &Signal) -> Result<()> {
        cli::run("make clean".to_string(), self.c_dir(), signal)
    }
}
