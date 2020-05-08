mod config;
mod deployment;
mod generator;
mod project_context;
mod recipe;
mod setup;
mod util;
mod wallet;

use std::env;
use std::path::PathBuf;
use std::process::{exit, Command};
use std::str::FromStr;

use anyhow::Result;
use ckb_tool::ckb_types::core::Capacity;
use deployment::manage::{DeployOption, Manage as DeployManage};
use generator::new_project;
use project_context::{load_project_context, Env};
use recipe::get_recipe;
use setup::setup;
use wallet::{Address, Wallet, DEFAULT_CKB_CLI_BIN_NAME, DEFAULT_CKB_RPC_URL};

fn run_cli() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().expect("command");
    let env = Env::Dev;
    match &command[..] {
        "setup" => {
            setup()?;
            println!("Done");
        }
        "new" => {
            let mut name = args.next().expect("name");
            let mut path = PathBuf::new();
            if let Some(index) = name.rfind("/") {
                path.push(&name[..index]);
                name = name[index + 1..].to_string();
            } else {
                path.push(env::current_dir()?);
            }
            new_project(name.to_string(), path)?;
        }
        "build" => {
            let context = load_project_context(env)?;
            for c in &context.config.contracts {
                println!("Building contract {}", c.name);
                get_recipe(&context, c)?.run_build()?;
            }
            println!("Done");
        }
        "test" => {
            let exit_code = Command::new("cargo").arg("test").spawn()?.wait()?;
            exit(exit_code.code().unwrap_or(1));
        }
        "deploy" => {
            let address = {
                let address_hex = args.next().expect("address");
                Address::from_str(&address_hex).expect("parse address")
            };
            let context = load_project_context(env)?;
            let wallet = Wallet::load(
                DEFAULT_CKB_RPC_URL.to_string(),
                DEFAULT_CKB_CLI_BIN_NAME.to_string(),
                address,
            );
            let migration_dir = context.migrations_path();
            let opt = DeployOption {
                migrate: true,
                tx_fee: Capacity::bytes(1).unwrap(),
            };
            DeployManage::new(migration_dir, context.load_deployment()?).deploy(wallet, opt)?;
        }
        _ => {
            println!("unrecognize command '{}'", command);
            exit(1);
        }
    }
    Ok(())
}

fn main() {
    run_cli().expect("error");
}
