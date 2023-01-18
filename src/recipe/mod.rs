mod c;
mod lua;
pub mod rust;

use crate::config::{Contract, TemplateType};
use crate::project_context::{BuildConfig, Context};
use crate::signal::Signal;
use anyhow::Result;

pub fn get_recipe(context: Context, template_type: TemplateType) -> Result<Box<dyn Recipe>> {
    match template_type {
        TemplateType::Rust => Ok(Box::new(rust::Rust::new(context))),
        TemplateType::C => Ok(Box::new(c::C::<c::CBin>::new(context))),
        TemplateType::CSharedLib => Ok(Box::new(c::C::<c::CSharedLib>::new(context))),
        TemplateType::Lua => Ok(Box::new(lua::Lua::<lua::LuaStandalone>::new(context))),
        TemplateType::LuaEmbedded => Ok(Box::new(lua::Lua::<lua::LuaEmbeddedLib>::new(context))),
    }
}

pub trait Recipe {
    fn exists(&self, name: &str) -> bool;
    fn create_contract(
        &self,
        contract: &Contract,
        rewrite_config: bool,
        signal: &Signal,
        docker_env_file: String,
    ) -> Result<()>;
    fn run(&self, contract: &Contract, build_cmd: String, signal: &Signal) -> Result<()>;
    fn run_build(
        &self,
        contract: &Contract,
        config: BuildConfig,
        signal: &Signal,
        build_args_opt: Option<Vec<String>>,
    ) -> Result<()>;
    fn clean(&self, contracts: &[Contract], signal: &Signal) -> Result<()>;
}
