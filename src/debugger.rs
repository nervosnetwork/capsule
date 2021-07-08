use crate::generator::TEMPLATES;
use crate::project_context::{BuildEnv, Context};
use crate::recipe::rust::DOCKER_IMAGE;
use crate::signal::Signal;
use crate::util::docker::DockerCommand;
use anyhow::{anyhow, Result};
use ckb_tool::ckb_hash::new_blake2b;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use tera::{self, Context as TeraContext};

pub fn start_debugger<P: AsRef<Path>>(
    context: &Context,
    template_path: P,
    contract_name: &str,
    env: BuildEnv,
    script_group_type: &str,
    cell_index: usize,
    cell_type: &str,
    max_cycles: u64,
    listen_port: usize,
    tty: bool,
    signal: &Signal,
) -> Result<()> {
    const DEBUG_SERVER_NAME: &str = "capsule-debugger-server";

    let project_path = context
        .project_path
        .to_str()
        .expect("project path")
        .to_string();
    let template_file_path = template_path
        .as_ref()
        .file_name()
        .expect("not a file")
        .to_str()
        .expect("file name");
    let patched_template_dir = format!("{}/.tmp", project_path);
    let patched_template_path = format!("{}/{}", patched_template_dir, template_file_path);
    fs::create_dir_all(patched_template_dir)?;
    let template_path = template_path
        .as_ref()
        .to_str()
        .expect("template path")
        .to_string();
    patch_template(context, env, &template_path, &patched_template_path)?;

    // start GDB server container
    let container_template_path = "/tmp/tx.json".to_string();
    let cmd = format!(
        "ckb-debugger --script-group-type {} --cell-index {} --cell-type {} --tx-file {} --max-cycle {} --listen 127.0.0.1:{}",
        script_group_type, cell_index, cell_type, container_template_path, max_cycles, listen_port
    );
    println!("GDB server is started!");
    DockerCommand::with_context(
        context,
        DOCKER_IMAGE.to_string(),
        project_path.clone(),
        &HashMap::new(),
    )
    .host_network(true)
    .name(DEBUG_SERVER_NAME.to_string())
    .daemon(tty)
    .map_volume(patched_template_path, container_template_path)
    .run(cmd, signal)?;
    if tty {
        let contract_path = match env {
            BuildEnv::Debug => format!("build/debug/{}", contract_name),
            BuildEnv::Release => format!("build/release/{}", contract_name),
        };
        // start gdb client
        let cmd = format!(
            "RUST_GDB=riscv64-unknown-elf-gdb rust-gdb -ex 'target remote :{port}' -ex 'file {contract_path}' -ex 'cd contracts/{contract}'",
            port=listen_port,
            contract=contract_name,
            contract_path=contract_path
        );
        let docker_cmd = DockerCommand::with_context(
            context,
            DOCKER_IMAGE.to_string(),
            project_path,
            &HashMap::new(),
        )
        .host_network(true)
        .tty(true);

        // Prepare a specific docker environment for GDB client then enable this
        //
        // if let Some(mut home_dir) = dirs::home_dir() {
        //     home_dir.push(".gdbinit");
        //     if home_dir.exists() {
        //         let host = home_dir.to_str().expect("path").to_string();
        //         docker_cmd = docker_cmd.map_volume(host, "/root/.gdbinit".to_string());
        //     }
        // }

        docker_cmd.run(cmd, signal)?;
    }
    DockerCommand::stop(DEBUG_SERVER_NAME)
}

#[derive(Serialize)]
struct TemplateContext {
    name: String,
}

pub fn build_template(contract_name: String) -> Result<String> {
    let context = TeraContext::from_serialize(&TemplateContext {
        name: contract_name,
    })?;
    let content = TEMPLATES.render("debugger/template.json", &context)?;
    Ok(content)
}

/// patch template
pub fn patch_template<P: AsRef<Path>>(
    context: &Context,
    env: BuildEnv,
    src: P,
    dst: P,
) -> Result<()> {
    let mut template = fs::read_to_string(src)?;

    // 1. search patch content from src
    let left: Vec<_> = template.match_indices("{{").collect();
    let right: Vec<_> = template.match_indices("}}").collect();
    if left.len() != right.len() {
        return Err(anyhow!(
            "Has {} '{{{{', but {} '}}}}'",
            left.len(),
            right.len()
        ));
    }

    let mut patch_content = HashSet::new();
    for ((start, _), (end, _)) in left.into_iter().zip(right) {
        if start > end {
            return Err(anyhow!("'}}}}' at {} has no begin mark", end));
        }
        let patch_target = template[(start + 2)..end].to_string();
        let mut parts: Vec<_> = patch_target.split(".").map(|s| s.to_string()).collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "template mark syntax error: '{}' expect 'contract.attribute'",
                patch_target
            ));
        }
        let contract = parts.remove(0);
        let attribute = parts.remove(0);
        patch_content.insert((contract, attribute));
    }

    // 2. replace template in memory
    let build_dir = context.contracts_build_path(env);
    for (contract, attribute) in patch_content {
        let mut contract_binary_path = build_dir.clone();
        contract_binary_path.push(&contract);
        if !contract_binary_path.exists() {
            return Err(anyhow!("contract not exists: {:?}", contract_binary_path));
        }
        let patch = match attribute.as_str() {
            "data" => {
                let bin = fs::read(contract_binary_path)?;
                faster_hex::hex_string(&bin)?
            }
            "code_hash" => {
                let bin = fs::read(contract_binary_path)?;
                let mut hasher = new_blake2b();
                hasher.update(&bin);
                let mut code_hash = [0u8; 32];
                hasher.finalize(&mut code_hash);
                faster_hex::hex_string(&code_hash)?
            }
            _ => panic!(
                "unknown template mark attribute: '{}.{}'",
                contract, attribute
            ),
        };
        // put 0x prefix
        let patch = format!("0x{}", patch);
        let source = format!("{{{{{}.{}}}}}", contract, attribute);
        template = template.replace(source.as_str(), patch.as_str());
    }

    // 3. dump template to dst
    fs::write(dst, template)?;
    Ok(())
}
