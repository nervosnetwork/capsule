pub mod transaction;

use crate::project_context::Context;
use crate::recipe::rust::DOCKER_IMAGE;
use crate::signal::Signal;
use crate::util::DockerCommand;
use anyhow::Result;
use ckb_testtool::context::Context as TestContext;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{HeaderView, TransactionBuilder},
    packed::*,
    prelude::*,
};
use std::fs;
use std::path::Path;
use transaction::*;

pub fn start_server<P: AsRef<Path>>(
    context: &Context,
    template_path: P,
    script_group_type: String,
    script_hash: String,
    listen_port: usize,
    signal: &Signal,
) -> Result<()> {
    const CONTAINER_NAME: &str = "capsule-debugger-server";

    let project_path = context
        .project_path
        .to_str()
        .expect("project path")
        .to_string();
    let template_path = template_path.as_ref().to_str().expect("template path");
    let cmd = format!(
        "ckb-debugger --script-group-type {} --script-hash {} --tx-file {} --listen 127.0.0.1:{}",
        script_group_type, script_hash, template_path, listen_port
    );
    println!("GDB server is started!\nhint: use rust-gdb connect to the remote server");
    DockerCommand::with_context(context, DOCKER_IMAGE.to_string(), project_path)
        .host_network(true)
        .name(CONTAINER_NAME.to_string())
        .run(cmd, signal)
}

pub fn build_template<P: AsRef<Path>>(contract_path: P) -> Result<(Script, MockTransaction)> {
    // deploy contract
    let mut context = TestContext::default();
    let contract_bin: Bytes = fs::read(contract_path)?.into();
    let contract_out_point = context.deploy_contract(contract_bin);

    // prepare scripts
    let lock_script = context
        .build_script(&contract_out_point, Default::default())
        .expect("script");
    let lock_script_dep = CellDep::new_builder().out_point(contract_out_point).build();

    // prepare cells
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .build(),
    ];

    let outputs_data = vec![Bytes::new(); 2];

    // build transaction
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(lock_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // mock transaction
    let inputs: Vec<MockInput> = tx
        .inputs()
        .into_iter()
        .map(|input| {
            let (output, data) = context
                .get_cell(&input.previous_output())
                .expect("input cell");
            MockInput {
                input,
                output,
                data,
                header: None,
            }
        })
        .collect();
    let cell_deps: Vec<MockCellDep> = tx
        .cell_deps()
        .into_iter()
        .map(|cell_dep| {
            let (output, data) = context.get_cell(&cell_dep.out_point()).expect("dep cell");
            MockCellDep {
                cell_dep,
                output,
                data,
                header: None,
            }
        })
        .collect();
    let header_deps: Vec<HeaderView> = tx
        .header_deps()
        .into_iter()
        .map(|header_hash| {
            context
                .headers
                .get(&header_hash)
                .expect("header")
                .to_owned()
        })
        .collect();
    let mock_info = MockInfo {
        inputs,
        cell_deps,
        header_deps,
    };
    let mock_tx = MockTransaction {
        tx: tx.data(),
        mock_info,
    };
    Ok((lock_script, mock_tx))
}
