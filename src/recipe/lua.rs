use crate::config::{Contract, TemplateType};
use crate::generator::{CreateContract, TEMPLATES};
use crate::project_context::{BuildConfig, BuildEnv, Context, CONTRACTS_DIR};
use crate::recipe::Recipe;
use crate::signal::Signal;
use crate::util::cli;
use crate::util::git;
use anyhow::{anyhow, Result};
use std::fs;
use std::io::Write;
use std::marker::PhantomData;
use std::path::PathBuf;
use tera;

// Files

const MAKEFILE: &str = "Makefile";

// Dirs

const LUA_DIR_PREFIX: &str = "lua";
const LUA_TEMPLATE_DIR_PREFIX: &str = "lua";
const DEPS_DIR_PREFIX: &str = "deps";
const SRC_DIR_PREFIX: &str = "src";
const DEBUG_DIR: &str = "build/debug";
const RELEASE_DIR: &str = "build/release";

// Deps

const CKB_LUA_URL: &str = "https://github.com/nervosnetwork/ckb-lua.git";
const CKB_LUA_COMMIT: &str = "ffc147e6a091a7a90b7dbe28d0a140def336bc7f";
const CKB_LUA_NAME: &str = "ckb-lua";

pub trait LuaRecipe {
    fn bin_name(name: &str) -> String;
    fn src_template() -> &'static str;
    fn build_template() -> &'static str;
}
pub struct LuaStandalone;

impl LuaRecipe for LuaStandalone {
    fn bin_name(name: &str) -> String {
        name.to_string()
    }

    fn src_template() -> &'static str {
        "standalone/contract/example.lua"
    }

    fn build_template() -> &'static str {
        "standalone/contract/BUILD"
    }
}
pub struct LuaEmbedded;

impl LuaRecipe for LuaEmbedded {
    fn bin_name(name: &str) -> String {
        name.to_string()
    }

    fn src_template() -> &'static str {
        "embedded/contract/example.c"
    }

    fn build_template() -> &'static str {
        "embedded/contract/BUILD"
    }
}

pub struct Lua<R> {
    context: Context,
    phantom_data: PhantomData<R>,
}

impl<R: LuaRecipe> Lua<R> {
    pub fn new(context: Context) -> Self {
        Self {
            context,
            phantom_data: PhantomData,
        }
    }

    fn lua_dir(&self) -> PathBuf {
        let mut c_dir = self.context.contracts_path();
        c_dir.push(LUA_DIR_PREFIX);
        c_dir
    }

    fn src_dir(&self) -> PathBuf {
        let mut src_path = self.lua_dir();
        src_path.push(SRC_DIR_PREFIX);
        src_path
    }

    fn makefile_path(&self) -> PathBuf {
        let mut p = self.lua_dir();
        p.push(MAKEFILE);
        p
    }

    fn setup_lua_environment(&self) -> Result<()> {
        println!("Setup Lua environment");
        let lua_dir = self.lua_dir();
        if lua_dir.exists() {
            return Ok(());
        }

        // Setup Dirs
        fs::create_dir(&lua_dir)?;

        for prefix in &[DEPS_DIR_PREFIX, SRC_DIR_PREFIX] {
            let mut dir = lua_dir.clone();
            dir.push(prefix);
            fs::create_dir(&dir)?;
        }

        // Pull deps
        let rel_path = format!(
            "{contracts}/{dir}/{deps}/{name}",
            contracts = CONTRACTS_DIR,
            dir = LUA_DIR_PREFIX,
            deps = DEPS_DIR_PREFIX,
            name = CKB_LUA_NAME
        );
        git::add_submodule(
            &self.context,
            CKB_LUA_URL,
            rel_path.as_str(),
            CKB_LUA_COMMIT,
        )?;

        // Generate files
        for f in &["Makefile"] {
            let template_path = format!("{}/{}", LUA_TEMPLATE_DIR_PREFIX, f);
            let content = TEMPLATES.render(&template_path, &tera::Context::default())?;
            let mut file_path = lua_dir.clone();
            file_path.push(f);
            fs::write(file_path, content)?;
        }

        Ok(())
    }

    fn source_name(&self, name: &str, contract_type: TemplateType) -> String {
        match contract_type {
            TemplateType::Lua => format!("{}.lua", name),
            TemplateType::LuaEmbedded => format!("{}.c", name),
            _ => unreachable!("Must be a Lua contract"),
        }
    }

    fn bin_path(&self, build_env: BuildEnv, name: &str) -> String {
        match build_env {
            BuildEnv::Debug => format!("{}/{}", DEBUG_DIR, R::bin_name(name)),
            BuildEnv::Release => format!("{}/{}", RELEASE_DIR, R::bin_name(name)),
        }
    }
}

impl<R: LuaRecipe> Recipe for Lua<R> {
    // The parameter name passed here is not enough to determine the contract
    // source file, below is a best-effort approach to check contract existence.
    fn exists(&self, name: &str) -> bool {
        let mut c_src = self.src_dir();
        c_src.push(self.source_name(name, TemplateType::Lua));
        let mut lua_src = self.src_dir();
        lua_src.push(self.source_name(name, TemplateType::LuaEmbedded));
        c_src.exists() || lua_src.exists()
    }

    fn create_contract(
        &self,
        contract: &Contract,
        rewrite_config: bool,
        _signal: &Signal,
        _docker_env_file: String,
    ) -> Result<()> {
        // setup c environment if needed
        self.setup_lua_environment()?;

        // new contract
        let name = &contract.name;
        println!("New contract {:?}", &name);
        let context = tera::Context::from_serialize(&CreateContract { name: name.clone() })?;

        // initialize contract code
        let f = R::src_template();
        let template_path = format!("{}/{}", LUA_TEMPLATE_DIR_PREFIX, f);
        let content = TEMPLATES.render(&template_path, &context)?;
        let mut src_path = self.src_dir();
        src_path.push(self.source_name(name, contract.template_type));
        fs::write(src_path, content)?;

        // TODO: support tests for TemplateType::Lua
        if contract.template_type == TemplateType::LuaEmbedded {
            for (f, template_name) in &[
                ("Cargo.toml", None),
                ("build.rs", None),
                ("src/lib.rs", None),
                ("src/tests.rs", None),
            ] {
                let template_path = format!(
                    "{}/embedded/contract/{}",
                    LUA_TEMPLATE_DIR_PREFIX,
                    template_name.unwrap_or(f)
                );
                let content = TEMPLATES.render(&template_path, &context)?;
                let mut file_path = self.context.project_path.clone();
                file_path.push(format!("tests/{}", f));
                fs::write(file_path, content)?;
            }
        }

        if rewrite_config {
            println!("Rewrite Makefile");
            let f = R::build_template();
            let template_path = format!("{}/{}", LUA_TEMPLATE_DIR_PREFIX, f);
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
    fn run(&self, _contract: &Contract, build_cmd: String, signal: &Signal) -> Result<()> {
        cli::run(build_cmd, self.lua_dir(), signal)
    }

    /// build contract
    /// Delegate to Makefile
    fn run_build(
        &self,
        c: &Contract,
        config: BuildConfig,
        signal: &Signal,
        _build_args_opt: Option<Vec<String>>,
    ) -> Result<()> {
        let path = self.bin_path(config.build_env, &c.name);
        let mut bin_path = self.lua_dir();
        bin_path.push(&path);
        // make sure the bin dir is exist
        fs::create_dir_all(&bin_path.parent().ok_or(anyhow!("expect build dir"))?)?;
        self.run(c, "make build".to_string(), signal)?;

        // copy to build dir
        if !bin_path.exists() {
            return Err(anyhow!(
                "can't find contract binary from path {:?}, please check Makefile",
                bin_path,
            ));
        }
        let mut target_path = self.context.project_path.clone();
        target_path.push(&path);
        // make sure the target dir is exist
        fs::create_dir_all(&target_path.parent().ok_or(anyhow!("expect build dir"))?)?;
        fs::copy(bin_path, target_path)?;
        Ok(())
    }

    /// clean contract
    /// Delegate to Makefile
    fn clean(&self, _contracts: &[Contract], signal: &Signal) -> Result<()> {
        cli::run("make clean".to_string(), self.lua_dir(), signal)
    }
}
