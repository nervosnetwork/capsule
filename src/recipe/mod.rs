mod rust;

use crate::config::{Contract, TemplateType};
use crate::project_context::Context;
use anyhow::Result;

pub fn get_recipe<'a>(context: &'a Context, contract: &'a Contract) -> Result<rust::Rust<'a>> {
    match contract.template_type {
        TemplateType::Rust => Ok(rust::Rust::new(context, contract)),
    }
}
