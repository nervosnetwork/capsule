pub mod transaction;

use anyhow::Result;
use ckb_testtool::context::Context;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{HeaderView, TransactionBuilder},
    packed::*,
    prelude::*,
};
use std::fs;
use std::path::Path;
use transaction::*;

pub fn build_template<P: AsRef<Path>>(contract_path: P) -> Result<(Script, MockTransaction)> {
    // deploy contract
    let mut context = Context::default();
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
