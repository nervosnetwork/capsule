mod config;
mod deployment;
mod generator;
mod project_context;
mod recipe;
mod setup;
mod wallet;

use std::env;
use std::path::PathBuf;
use std::process::{exit, Command};

use anyhow::Result;
use ckb_tool::faster_hex::hex_decode;
use ckb_tool::rpc_client::RpcClient;
use deployment::DeploymentProcess;
use generator::new_project;
use project_context::load_project_context;
use recipe::get_recipe;
use setup::setup;
use wallet::{Wallet, DEFAULT_CKB_CLI_BIN_NAME, DEFAULT_CKB_RPC_URL};

fn run_cli() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().expect("command");
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
            let context = load_project_context()?;
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
            let lock_arg = {
                let lock_arg_hex = args.next().expect("lock_arg");
                let mut buf = [0u8; 20];
                hex_decode(lock_arg_hex.as_bytes(), &mut buf).expect("dehex");
                buf
            };
            let context = load_project_context()?;
            let rpc_client = RpcClient::new(DEFAULT_CKB_RPC_URL);
            let wallet = Wallet::load(DEFAULT_CKB_CLI_BIN_NAME.to_string(), rpc_client, lock_arg);
            DeploymentProcess::new(context.load_deployment()?, wallet).deploy()?;
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
