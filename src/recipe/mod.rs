pub mod rust;

use crate::config::{Contract, TemplateType};
use crate::project_context::{BuildConfig, Context};
use crate::signal::Signal;
use anyhow::Result;

pub fn get_recipe<'a>(context: &'a Context, contract: &'a Contract) -> Result<impl Recipe<'a>> {
    match contract.template_type {
        TemplateType::Rust => Ok(rust::Rust::new(context, contract)),
    }
}

pub trait Recipe<'a> {
    fn new(context: &'a Context, contract: &'a Contract) -> Self;
    fn create_contract(&self, rewrite_config: bool, signal: &Signal) -> Result<()>;
    fn run(&self, build_cmd: String, signal: &Signal) -> Result<()>;
    fn run_build(&self, config: BuildConfig, signal: &Signal) -> Result<()>;
    fn clean(&self, signal: &Signal) -> Result<()>;
}
