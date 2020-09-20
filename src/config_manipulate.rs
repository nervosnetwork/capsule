//! functions manipulate config file

use crate::config::TemplateType;
use anyhow::{anyhow, Result};
pub use toml_edit::{value, Document, Table};

pub fn append_contract(
    doc: &mut Document,
    name: String,
    template_type: TemplateType,
) -> Result<()> {
    let contracts = doc["contracts"]
        .as_array_of_tables_mut()
        .ok_or(anyhow!("no 'contracts' section"))?;
    let contract = contracts.append(Table::new());
    contract["name"] = value(name);
    contract["template_type"] = value(toml::to_string(&template_type)?);
    Ok(())
}

pub fn append_workspace_member(doc: &mut Document, name: String) -> Result<()> {
    let workspace = doc["workspace"]
        .as_table_mut()
        .ok_or(anyhow!("no 'workspace' section"))?;
    let members = workspace["members"]
        .as_array_mut()
        .ok_or(anyhow!("no 'members' section"))?;
    members.push(name);
    Ok(())
}
