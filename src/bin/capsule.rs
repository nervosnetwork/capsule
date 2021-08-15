use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use ckb_capsule::checker::Checker;
use ckb_capsule::config::{Contract, TemplateType};
use ckb_capsule::config_manipulate::{append_contract, Document};
use ckb_capsule::debugger;
use ckb_capsule::deployment::manage::{DeployOption, Manage as DeployManage};
use ckb_capsule::generator::new_project;
use ckb_capsule::project_context::{
    read_config_file, write_config_file, BuildConfig, BuildEnv, Context, DeployEnv, CONFIG_FILE,
};
use ckb_capsule::recipe::get_recipe;
use ckb_capsule::signal;
use ckb_capsule::tester::Tester;
use ckb_capsule::version::Version;
use ckb_capsule::wallet::cli_types::HumanCapacity;
use ckb_capsule::wallet::{Address, Wallet, DEFAULT_CKB_CLI_BIN_NAME, DEFAULT_CKB_RPC_URL};
use ckb_tool::ckb_types::core::Capacity;
use log::debug;

use clap::{App, AppSettings, Arg, SubCommand};

const DEBUGGER_MAX_CYCLES: u64 = 70_000_000u64;
const TEMPLATES_NAMES: &[&str] = &["rust", "c", "c-sharedlib"];

fn append_contract_to_config(context: &Context, contract: &Contract) -> Result<()> {
    println!("Rewrite capsule.toml");
    let mut config_path = context.project_path.clone();
    config_path.push(CONFIG_FILE);
    let config_content = read_config_file(&config_path)?;
    let mut doc = config_content.parse::<Document>()?;
    append_contract(&mut doc, contract.name.to_string(), contract.template_type)?;
    write_config_file(&config_path, doc.to_string())?;
    Ok(())
}

fn select_contracts(context: &Context, names: &[&str]) -> Vec<Contract> {
    context
        .config
        .contracts
        .iter()
        .filter(|c| names.is_empty() || names.contains(&c.name.as_str()))
        .cloned()
        .collect()
}

fn group_contracts_by_type(contracts: Vec<Contract>) -> HashMap<TemplateType, Vec<Contract>> {
    let mut contracts_by_type = HashMap::default();
    for c in contracts {
        contracts_by_type
            .entry(c.template_type)
            .or_insert(Vec::new())
            .push(c.clone());
    }
    contracts_by_type
}

fn run_cli() -> Result<()> {
    env_logger::init();

    let version_str = Version::current().to_string();
    let default_max_cycles_str = format!("{}", DEBUGGER_MAX_CYCLES);

    let contract_args = [
        Arg::with_name("name")
            .help("project name")
            .index(1)
            .required(true)
            .takes_value(true),
        Arg::with_name("template")
            .long("template")
            .help("language template")
            .possible_values(TEMPLATES_NAMES)
            .default_value(TEMPLATES_NAMES[0])
            .takes_value(true),
    ];

    let mut app = App::new("Capsule")
        .setting(AppSettings::ArgRequiredElseHelp)
        .version(version_str.as_str())
        .author("Nervos Developer Tools Team")
        .about("Capsule CKB contract scaffold")
        .subcommand(SubCommand::with_name("check").about("Check environment and dependencies").display_order(0))
        .subcommand(SubCommand::with_name("new").about("Create a new project").args(&contract_args).display_order(1))
        .subcommand(SubCommand::with_name("new-contract").about("Create a new contract").args(&contract_args).display_order(2))
        .subcommand(SubCommand::with_name("build").about("Build contracts")
            .arg(Arg::with_name("name").short("n").long("name").multiple(true).takes_value(true).help("contract name"))
            .arg(Arg::with_name("release").long("release").help("Build contracts in release mode."))
            .arg(Arg::with_name("debug-output").long("debug-output").help("Always enable debugging output"))
            .arg(Arg::with_name("env").long("env").short("e").multiple(true).takes_value(true).help("Environment variable passed to compiler assigned by equal-signs, only ASCII key and value pairs, e.g. --env --env X=A Y=B Z"))
            .display_order(3))
        .subcommand(SubCommand::with_name("run").about("Run command in contract build image").usage("ckb_capsule run --name <name> 'echo list contract dir: && ls'")
            .args(&[Arg::with_name("name").short("n").long("name").required(true).takes_value(true).help("contract name"),
                Arg::with_name("cmd").required(true).multiple(true).help("command to run")])
            .display_order(4))
        .subcommand(SubCommand::with_name("test").about("Run tests").arg(
            Arg::with_name("release").long("release").help("Test release mode contracts.")
        ).display_order(5))
        .subcommand(
            SubCommand::with_name("deploy")
                .about("Deploy contracts, edit deployment.toml to custodian deployment recipe.")
                .args(&[
                    Arg::with_name("address").long("address").help(
                        "Denote which address provides cells",
                    ).required(true).takes_value(true),
                    Arg::with_name("fee").long("fee").help(
                        "Per transaction's fee, deployment may involve more than one transaction.",
                    ).default_value("0.0001").takes_value(true),
                    Arg::with_name("env").long("env").help("Deployment environment.")
                        .possible_values(&["dev", "production"]).default_value("dev").takes_value(true),
                    Arg::with_name("migrate")
                        .long("migrate")
                        .help("Use previously deployed cells as inputs.").possible_values(&["on", "off"]).default_value("on").takes_value(true),
                    Arg::with_name("api")
                        .long("api")
                        .help("CKB RPC url").default_value(DEFAULT_CKB_RPC_URL).takes_value(true),
                    Arg::with_name("ckb-cli")
                        .long("ckb-cli")
                        .help("CKB cli binary").default_value(DEFAULT_CKB_CLI_BIN_NAME).takes_value(true),
                ]).display_order(6),
        )
        .subcommand(SubCommand::with_name("clean").about("Remove contracts targets and binaries").arg(Arg::with_name("name").short("n").long("name").multiple(true).takes_value(true).help("contract name"))
            .display_order(7))
        .subcommand(
            SubCommand::with_name("debugger")
                .about("CKB debugger")
                .subcommand(
                    SubCommand::with_name("gen-template")
                        .about("Generate transaction debugging template")
                        .args(&[
                            Arg::with_name("name").long("name").short("n").help(
                                "contract name",
                            ).required(true).takes_value(true),
                            Arg::with_name("output-file")
                                .long("output-file")
                                .short("o")
                                .help("Output file path").required(true).takes_value(true),
                        ])
                )
                .subcommand(
                    SubCommand::with_name("start")
                        .about("Start GDB")
                        .args(&[
                            Arg::with_name("template-file")
                                .long("template-file")
                                .short("f")
                                .help("Transaction debugging template file")
                                .required(true)
                                .takes_value(true),
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .required(true)
                                .takes_value(true)
                                .help("contract name"),
                            Arg::with_name("release").long("release").help("Debugging release contract"),
                            Arg::with_name("script-group-type")
                                .long("script-group-type")
                                .possible_values(&["type", "lock"])
                                .help("Script type")
                                .required(true)
                                .takes_value(true),
                            Arg::with_name("cell-index")
                                .long("cell-index")
                                .required(true)
                                .help("index of the cell")
                                .takes_value(true),
                            Arg::with_name("cell-type")
                                .long("cell-type")
                                .required(true)
                                .possible_values(&["input", "output"])
                                .help("cell type")
                                .takes_value(true),
                            Arg::with_name("max-cycles")
                                .long("max-cycles")
                                .default_value(&default_max_cycles_str)
                                .help("Max cycles")
                                .takes_value(true),
                            Arg::with_name("listen")
                                .long("listen")
                                .short("l")
                                .help("GDB server listening port")
                                .default_value("8000").required(true).takes_value(true),
                            Arg::with_name("only-server").long("only-server").help("Only start debugger server"),
                        ])
                )
                .display_order(8),
        );

    let signal = signal::Signal::setup();

    let help_str = {
        let mut buf = Vec::new();
        app.write_long_help(&mut buf)?;
        String::from_utf8(buf)?
    };

    let matches = app.get_matches();
    match matches.subcommand() {
        ("check", _args) => {
            Checker::build()?.print_report();
        }
        ("new", Some(args)) => {
            let mut name = args
                .value_of("name")
                .expect("name")
                .trim()
                .trim_end_matches("/")
                .to_string();
            let template_type: TemplateType =
                args.value_of("template").expect("template").parse()?;
            let mut path = PathBuf::new();
            if let Some(index) = name.rfind("/") {
                path.push(&name[..index]);
                name = name[index + 1..].to_string();
            } else {
                path.push(env::current_dir()?);
            }
            let project_path = new_project(name.to_string(), path, &signal)?;
            let context = Context::load_from_path(&project_path)?;
            let c = Contract {
                name,
                template_type,
            };
            get_recipe(context.clone(), c.template_type)?.create_contract(&c, true, &signal)?;
            append_contract_to_config(&context, &c)?;
            println!("Done");
        }
        ("new-contract", Some(args)) => {
            let context = Context::load()?;
            let name = args.value_of("name").expect("name").trim().to_string();
            let template_type: TemplateType =
                args.value_of("template").expect("template").parse()?;
            let contract = Contract {
                name,
                template_type,
            };
            let recipe = get_recipe(context.clone(), contract.template_type)?;
            if recipe.exists(&contract.name) {
                return Err(anyhow!("contract '{}' is already exists", contract.name));
            }
            recipe.create_contract(&contract, true, &signal)?;
            append_contract_to_config(&context, &contract)?;
            println!("Done");
        }
        ("build", Some(args)) => {
            let context = Context::load()?;
            let build_names: Vec<&str> = args
                .values_of("name")
                .map(|values| values.collect())
                .unwrap_or_default();
            let build_env: BuildEnv = if args.is_present("release") {
                BuildEnv::Release
            } else {
                BuildEnv::Debug
            };
            let always_debug = args.is_present("debug-output");
            let build_config = BuildConfig {
                build_env,
                always_debug,
            };

            let custom_env = args
                .values_of("env")
                .map(|values| {
                    values
                        .into_iter()
                        .map(|entry| {
                            debug!("entry: ->{}<-", entry);
                            if let Some(index) = entry.find('=') {
                                if index == 0 {
                                    Err(anyhow!("Key of env {} ?", entry))
                                } else {
                                    let key = entry[0..index].to_string();
                                    //the value could be emtpy
                                    let value = entry[index + 1..].to_string();
                                    debug!("Env: {}={}", key, value);
                                    Ok((key, value))
                                }
                            } else {
                                Ok((entry.to_string(), "".to_string()))
                            }
                        })
                        .collect::<Result<HashMap<String, String>, _>>()
                })
                .unwrap_or(Ok(HashMap::<_, _, _>::default()))?;

            let contracts: Vec<_> = select_contracts(&context, &build_names);
            if contracts.is_empty() {
                println!("Nothing to do");
            } else {
                for contract in contracts {
                    println!("Building contract {}", contract.name);
                    let recipe = get_recipe(context.clone(), contract.template_type)?;
                    recipe.run_build(&contract, build_config, &signal, &custom_env)?;
                }
                println!("Done");
            }
        }
        ("clean", Some(args)) => {
            let context = Context::load()?;
            let build_names: Vec<&str> = args
                .values_of("name")
                .map(|values| values.collect())
                .unwrap_or_default();
            let contracts: Vec<_> = select_contracts(&context, &build_names);
            if contracts.is_empty() {
                println!("Nothing to do");
            } else {
                let contracts_by_types = group_contracts_by_type(contracts);
                for (template_type, contracts) in contracts_by_types {
                    get_recipe(context.clone(), template_type)?.clean(&contracts, &signal)?;
                }
                println!("Done");
            }
        }
        ("run", Some(args)) => {
            let context = Context::load()?;
            let name = args.value_of("name").expect("name");
            let cmd = args
                .values_of("cmd")
                .expect("cmd")
                .collect::<Vec<&str>>()
                .join(" ");
            let contract = match context.config.contracts.iter().find(|c| name == c.name) {
                Some(c) => c.clone(),
                None => return Err(anyhow!("can't find contract '{}'", name)),
            };
            get_recipe(context, contract.template_type)?.run(
                &contract,
                cmd,
                &signal,
                &HashMap::new(),
            )?;
        }
        ("test", Some(args)) => {
            let context = Context::load()?;
            let build_env: BuildEnv = if args.is_present("release") {
                BuildEnv::Release
            } else {
                BuildEnv::Debug
            };
            Tester::run(&context, build_env, &signal)?;
        }
        ("deploy", Some(args)) => {
            Checker::build()?.check_ckb_cli()?;
            let address = {
                let address_hex = args.value_of("address").expect("address");
                Address::from_str(&address_hex).expect("parse address")
            };
            let context = Context::load()?;
            let ckb_rpc_url = args.value_of("api").expect("api");
            let ckb_cli_bin = args.value_of("ckb-cli").expect("ckb-cli");
            let wallet = Wallet::load(ckb_rpc_url.to_string(), ckb_cli_bin.to_string(), address);
            let deploy_env: DeployEnv = args
                .value_of("env")
                .expect("deploy env")
                .parse()
                .map_err(|err: &str| anyhow!(err))?;
            let migration_dir = context.migrations_path(deploy_env);
            let migrate = args.value_of("migrate").expect("migrate").to_lowercase() == "on";
            let tx_fee = match HumanCapacity::from_str(args.value_of("fee").expect("tx fee")) {
                Ok(tx_fee) => Capacity::shannons(tx_fee.0),
                Err(err) => return Err(anyhow!(err)),
            };
            let opt = DeployOption { migrate, tx_fee };
            DeployManage::new(migration_dir, context.load_deployment()?).deploy(wallet, opt)?;
        }
        ("debugger", Some(sub_matches)) => match sub_matches.subcommand() {
            ("gen-template", Some(args)) => {
                let contract = args.value_of("name").expect("contract name");
                let template_content = debugger::build_template(contract.to_string())?;
                let template_path = args.value_of("output-file").expect("output file");
                fs::write(&template_path, template_content)?;
                println!("Write transaction debugging template to {}", template_path,);
            }
            ("start", Some(args)) => {
                let context = Context::load()?;
                let template_path = args.value_of("template-file").expect("template file");
                let build_env: BuildEnv = if args.is_present("release") {
                    BuildEnv::Release
                } else {
                    BuildEnv::Debug
                };
                let contract_name = args.value_of("name").expect("contract name");
                let script_group_type = args.value_of("script-group-type").unwrap();
                let cell_index: usize = args.value_of("cell-index").unwrap().parse()?;
                let cell_type = args.value_of("cell-type").unwrap();
                let max_cycles: u64 = args.value_of("max-cycles").unwrap().parse()?;
                let listen_port: usize = args
                    .value_of("listen")
                    .unwrap()
                    .parse()
                    .expect("listen port");
                let tty = !args.is_present("only-server");
                debugger::start_debugger(
                    &context,
                    template_path,
                    contract_name,
                    build_env,
                    script_group_type,
                    cell_index,
                    cell_type,
                    max_cycles,
                    listen_port,
                    tty,
                    &signal,
                )?;
            }
            (command, _) => {
                eprintln!("unknown debugger subcommand '{}'", command);
                eprintln!("{}", help_str);
                exit(1);
            }
        },
        (command, _) => {
            eprintln!("unrecognize command '{}'", command);
            eprintln!("{}", help_str);
            exit(1);
        }
    }
    Ok(())
}

fn main() {
    let backtrace_level = env::var("RUST_BACKTRACE").unwrap_or("".to_string());
    let enable_backtrace =
        !backtrace_level.is_empty() && backtrace_level.as_str() != "0".to_string();
    match run_cli() {
        Ok(_) => {}
        err if enable_backtrace => {
            err.unwrap();
        }
        Err(err) => {
            eprintln!("error: {}", err);
            exit(-1);
        }
    }
}
