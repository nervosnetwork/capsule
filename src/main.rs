mod checker;
mod config;
mod deployment;
mod generator;
mod project_context;
mod recipe;
mod tester;
mod util;
mod wallet;

use std::env;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use checker::Checker;
use ckb_tool::ckb_types::core::Capacity;
use deployment::manage::{DeployOption, Manage as DeployManage};
use generator::new_project;
use project_context::{load_project_context, Env};
use recipe::get_recipe;
use tester::Tester;
use wallet::cli_types::HumanCapacity;
use wallet::{Address, Wallet, DEFAULT_CKB_CLI_BIN_NAME, DEFAULT_CKB_RPC_URL};

use clap::{App, Arg, SubCommand};

fn run_cli() -> Result<()> {
    let matches = App::new("Capsule")
        .version("0.0.0-pre.1")
        .author("Nervos Developer Tools Team")
        .about("Capsule CKB contract scaffold")
        .subcommand(SubCommand::with_name("check").about("Check environment and dependencies"))
        .subcommand(SubCommand::with_name("new").about("Create a new project").arg(Arg::with_name("name").help("project name").index(1).required(true).takes_value(true)))
        .subcommand(SubCommand::with_name("build").about("Build contracts"))
        .subcommand(SubCommand::with_name("test").about("Run tests"))
        .subcommand(
            SubCommand::with_name("deploy")
                .about("Deploy contracts")
                .help("Edit deployment.toml to custodian deployment recipe.")
                .args(&[
                    Arg::with_name("address").long("address").help(
                        "Denote which address provides cells",
                    ).required(true).takes_value(true),
                    Arg::with_name("fee").long("fee").help(
                        "Per transaction's fee, deployment may involve more than one transaction.",
                    ).default_value("0.0001").takes_value(true),
                    Arg::with_name("no-migrate")
                        .long("no-migrate")
                        .help("Do not use deployed cells as inputs."),
                ]),
        )
        .get_matches();
    let env = Env::Dev;
    match matches.subcommand() {
        ("check", _args) => {
            Checker::build()?.print_report();
        }
        ("new", Some(args)) => {
            let mut name = args.value_of("name").expect("name").to_string();
            let mut path = PathBuf::new();
            if let Some(index) = name.rfind("/") {
                path.push(&name[..index]);
                name = name[index + 1..].to_string();
            } else {
                path.push(env::current_dir()?);
            }
            new_project(name.to_string(), path)?;
        }
        ("build", _args) => {
            let context = load_project_context(env)?;
            for c in &context.config.contracts {
                println!("Building contract {}", c.name);
                get_recipe(&context, c)?.run_build()?;
            }
            println!("Done");
        }
        ("test", _args) => {
            let context = load_project_context(env)?;
            let exit_code = Tester::run(&context.project_path)?;
            exit(exit_code.code().unwrap_or(1));
        }
        ("deploy", Some(args)) => {
            if !Checker::build()?.ckb_cli {
                eprintln!("Can't find ckb-cli, install it to enable deployment");
                exit(1);
            }
            let address = {
                let address_hex = args.value_of("address").expect("address");
                Address::from_str(&address_hex).expect("parse address")
            };
            let context = load_project_context(env)?;
            let wallet = Wallet::load(
                DEFAULT_CKB_RPC_URL.to_string(),
                DEFAULT_CKB_CLI_BIN_NAME.to_string(),
                address,
            );
            let migration_dir = context.migrations_path();
            let migrate = !args.is_present("no-migrate");
            let tx_fee = match HumanCapacity::from_str(args.value_of("fee").expect("tx fee")) {
                Ok(tx_fee) => Capacity::shannons(tx_fee.0),
                Err(err) => return Err(anyhow!(err)),
            };
            let opt = DeployOption { migrate, tx_fee };
            DeployManage::new(migration_dir, context.load_deployment()?).deploy(wallet, opt)?;
        }
        (command, _) => {
            eprintln!("unrecognize command '{}'", command);
            exit(1);
        }
    }
    Ok(())
}

fn main() {
    run_cli().expect("error");
}
