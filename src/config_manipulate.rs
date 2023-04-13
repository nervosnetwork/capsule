//! functions manipulate config file

use crate::config::{Contract, TemplateType};
use anyhow::{anyhow, Result};
use serde::Serialize;
pub use toml_edit::Document;
use toml_edit::{array, ser::ValueSerializer, value};

pub fn append_contract(
    doc: &mut Document,
    name: String,
    template_type: TemplateType,
) -> Result<()> {
    let contracts = doc["contracts"]
        .or_insert(array())
        .as_array_of_tables_mut()
        .ok_or(anyhow!("no 'contracts' section"))?;
    // Why doesn't toml_edit provide a to_value function?
    let t = Contract {
        name,
        template_type,
    }
    .serialize(ValueSerializer::new())?;
    contracts.push(value(t).into_table().unwrap());
    Ok(())
}

pub fn append_cargo_workspace_member(doc: &mut Document, name: String) -> Result<()> {
    let workspace = doc["workspace"]
        .as_table_mut()
        .ok_or(anyhow!("no 'workspace' section"))?;
    let members = workspace["members"]
        .as_array_mut()
        .ok_or(anyhow!("no 'members' section"))?;
    members.push(name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::append_contract;

    #[test]
    fn test_append_contract() {
        let mut doc = r#"
version = "0.9.1"
"#
        .parse()
        .unwrap();
        append_contract(&mut doc, "a".into(), crate::config::TemplateType::C).unwrap();
        append_contract(
            &mut doc,
            "'strange name\"".into(),
            crate::config::TemplateType::Rust,
        )
        .unwrap();
        assert_eq!(
            doc.to_string(),
            r#"
version = "0.9.1"

[[contracts]]
name = "a"
template_type = "C"

[[contracts]]
name = "'strange name\""
template_type = "Rust"
"#
        )
    }
}
