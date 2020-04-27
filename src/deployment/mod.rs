mod recipe;

use crate::wallet::*;
use recipe::*;

use crate::config::{Cell, CellLocation, DepGroup, Deployment};
use anyhow::Result;
use ckb_tool::ckb_chain_spec::consensus::TYPE_ID_CODE_HASH;
use ckb_tool::ckb_hash::new_blake2b;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{Capacity, ScriptHashType, TransactionBuilder, TransactionView},
    packed,
    prelude::*,
};
use std::fs;
use std::io::Read;

pub struct DeploymentProcess {
    wallet: Wallet,
    tx_fee: Capacity,
    config: Deployment,
}

impl DeploymentProcess {
    pub fn new(config: Deployment, wallet: Wallet) -> Self {
        // TODO optimize tx fee
        let tx_fee = Capacity::bytes(100).expect("fee");
        DeploymentProcess {
            wallet,
            tx_fee,
            config,
        }
    }

    /// generate recipe and deploy
    pub fn deploy(&mut self) -> Result<()> {
        let cells: Vec<(Cell, Bytes)> = load_deployable_cells_data(&self.config.cells);
        let dep_groups = self.config.dep_groups.clone();
        let (recipe, txs) = self.build_recipe(cells, dep_groups);
        self.execute_recipe(recipe, txs)?;
        Ok(())
    }

    /// rerun an exist recipe
    /// any txs on-chain will be ignored
    pub fn rerun_recipe(&mut self, recipe: DeploymentRecipe) -> Result<()> {
        let cells: Vec<(Cell, Bytes)> = load_deployable_cells_data(&self.config.cells);
        let dep_groups = self.config.dep_groups.clone();
        let (recipe, txs) = self.build_recipe(cells, dep_groups);
        self.execute_recipe(recipe, txs)?;
        Ok(())
    }

    fn build_cells_tx(&mut self, deployable_cells: &[(Cell, Bytes)]) -> TransactionView {
        let lock: packed::Script = self.config.lock.to_owned().into();
        // type_id requires at least one input
        let type_id_live_cells = self.wallet.collect_live_cells(Capacity::shannons(1));
        self.wallet.lock_cells(type_id_live_cells.clone().into_iter());
        let inputs: Vec<_> = type_id_live_cells
            .iter()
            .map(|live_cell| {
                let index: u32 = live_cell.index.output_index;
                packed::CellInput::new_builder()
                    .previous_output(
                        packed::OutPoint::new_builder()
                            .tx_hash(live_cell.tx_hash.pack())
                            .index((index as u32).pack())
                            .build(),
                    )
                    .build()
            })
            .collect();
        let outputs: Vec<_> = deployable_cells
            .iter()
            .enumerate()
            .map(|(i, (cell, data))| {
                let mut output = packed::CellOutput::new_builder().lock(lock.clone());
                if cell.enable_type_id {
                    let type_script = build_type_id_script(&inputs[0], i as u64);
                    output = output.type_(Some(type_script).pack());
                }
                output
                    .build_exact_capacity(Capacity::bytes(data.len()).expect("bytes"))
                    .expect("build")
            })
            .collect();
        let cells_data: Vec<_> = deployable_cells
            .iter()
            .map(|(_, data)| data.to_owned())
            .collect();
        let tx = TransactionBuilder::default()
            .inputs(inputs.pack())
            .outputs(outputs.pack())
            .outputs_data(cells_data.pack())
            .build();
        let tx = self.wallet.complete_tx_lock_deps(tx);
        let tx = self
            .wallet
            .complete_tx_inputs(tx, type_id_live_cells.iter(), self.tx_fee);
        self.wallet.lock_tx_inputs(&tx);
        tx
    }

    fn build_dep_groups_tx(
        &mut self,
        cell_txs: &[&CellTxRecipe],
        deployable_dep_groups: &[DepGroup],
    ) -> TransactionView {
        fn find_cell(name: &str, cell_txs: &[&CellTxRecipe]) -> Option<([u8; 32], CellRecipe)> {
            for tx in cell_txs {
                if let Some(c) = tx.cells.iter().find(|c| c.name == name) {
                    return Some((tx.tx_hash.to_owned(), c.clone()));
                }
            }
            return None;
        }

        let lock: packed::Script = self.config.lock.to_owned().into();
        let mut cells_data: Vec<Bytes> = Vec::new();
        let mut outputs: Vec<packed::CellOutput> = Vec::new();
        for dep_group in deployable_dep_groups.iter() {
            let out_points: packed::OutPointVec = dep_group
                .cells
                .iter()
                .map(|name| {
                    let cell = self
                        .config
                        .cells
                        .iter()
                        .find(|c| &c.name == name)
                        .expect("find cell");
                    let (tx_hash, index) = match cell.location.clone() {
                        CellLocation::File { .. } => {
                            let (tx_hash, cell) = find_cell(name, cell_txs).expect("must exists");
                            (tx_hash, cell.index)
                        }
                        CellLocation::OutPoint { tx_hash, index } => (tx_hash.into(), index),
                    };
                    packed::OutPoint::new_builder()
                        .tx_hash(tx_hash.pack())
                        .index(index.pack())
                        .build()
                })
                .pack();
            let data = out_points.as_bytes();
            let data_len = data.len();
            cells_data.push(data);
            let output = packed::CellOutput::new_builder()
                .lock(lock.clone())
                .build_exact_capacity(Capacity::bytes(data_len).expect("bytes"))
                .expect("build");
            outputs.push(output);
        }
        let tx = TransactionBuilder::default()
            .outputs(outputs.pack())
            .outputs_data(cells_data.pack())
            .build();
        let tx = self.wallet.complete_tx_lock_deps(tx);
        let tx = self.wallet.complete_tx_inputs(tx, Vec::new().into_iter(), self.tx_fee);
        self.wallet.lock_tx_inputs(&tx);
        tx
    }

    fn build_recipe(
        &mut self,
        cells: Vec<(Cell, Bytes)>,
        dep_groups: Vec<DepGroup>,
    ) -> (DeploymentRecipe, Vec<TransactionView>) {
        // build cells tx
        let mut cells_tx = self.build_cells_tx(&cells);
        let cells_tx_recipe = build_cell_tx_recipe(&cells_tx, &cells);
        // build dep_groups tx
        let mut dep_groups_tx = self.build_dep_groups_tx(&[&cells_tx_recipe], &dep_groups);
        let dep_group_tx_recipe = build_dep_group_tx_recipe(&dep_groups_tx, &dep_groups);
        // sign txs
        {
            let password = self.wallet.read_password().expect("read password");
            cells_tx = self.wallet.sign_tx(cells_tx, password.clone()).expect("sign cells_tx");
            dep_groups_tx = self.wallet.sign_tx(dep_groups_tx, password).expect("sign dep_groups_tx");
        }
        // construct deployment recipe
        let recipe = DeploymentRecipe {
            cell_txs: vec![cells_tx_recipe],
            dep_group_txs: vec![dep_group_tx_recipe],
        };
        (recipe, vec![cells_tx, dep_groups_tx])
    }

    fn execute_recipe(
        &mut self,
        recipe: DeploymentRecipe,
        txs: Vec<TransactionView>,
    ) -> Result<()> {
        for cell_tx in recipe.cell_txs {
            if self.wallet.query_transaction(&cell_tx.tx_hash)?.is_some() {
                continue;
            }
            let tx = txs
                .iter()
                .find(|tx| {
                    let tx_hash: [u8; 32] = tx.hash().unpack();
                    cell_tx.tx_hash == tx_hash
                })
                .expect("missing recipe tx");
            let tx_hash = self.wallet.send_transaction(tx.to_owned())?;
            println!("send cell_tx {}", tx_hash);
        }
        for dep_group_tx in recipe.dep_group_txs {
            if self
                .wallet
                .query_transaction(&dep_group_tx.tx_hash)?
                .is_some()
            {
                continue;
            }
            let tx = txs
                .iter()
                .find(|tx| {
                    let tx_hash: [u8; 32] = tx.hash().unpack();
                    dep_group_tx.tx_hash == tx_hash
                })
                .expect("missing recipe tx");
            let tx_hash = self.wallet.send_transaction(tx.to_owned())?;
            println!("send dep_group_tx {}", tx_hash);
        }
        Ok(())
    }
}

fn build_cell_tx_recipe(tx: &TransactionView, cells: &[(Cell, Bytes)]) -> CellTxRecipe {
    CellTxRecipe {
        tx_hash: tx.hash().unpack(),
        cells: cells
            .iter()
            .enumerate()
            .map(|(i, (c, data))| CellRecipe {
                index: i as u32,
                name: c.name.to_owned(),
                data_hash: packed::CellOutput::calc_data_hash(&data).unpack(),
            })
            .collect(),
    }
}

fn build_dep_group_tx_recipe(tx: &TransactionView, dep_groups: &[DepGroup]) -> DepGroupTxRecipe {
    DepGroupTxRecipe {
        tx_hash: tx.hash().unpack(),
        dep_groups: dep_groups
            .iter()
            .enumerate()
            .map(|(i, dep_group)| DepGroupRecipe {
                index: i as u32,
                name: dep_group.name.to_owned(),
            })
            .collect(),
    }
}

fn build_type_id_script(input: &packed::CellInput, output_index: u64) -> packed::Script {
    let mut blake2b = new_blake2b();
    blake2b.update(&input.as_slice());
    blake2b.update(&output_index.to_le_bytes());
    let mut ret = [0; 32];
    blake2b.finalize(&mut ret);
    let script_arg = Bytes::from(ret.to_vec());
    packed::Script::new_builder()
        .code_hash(TYPE_ID_CODE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(script_arg.pack())
        .build()
}

fn load_deployable_cells_data(cells: &[Cell]) -> Vec<(Cell, Bytes)> {
    let mut cells_data: Vec<(Cell, Bytes)> = Vec::new();
    for cell in cells {
        match cell.location.to_owned() {
            CellLocation::OutPoint { .. } => {}
            CellLocation::File { file } => {
                let mut data = Vec::new();
                fs::File::open(file)
                    .expect("open")
                    .read_to_end(&mut data)
                    .expect("read");
                cells_data.push((cell.to_owned(), data.into()));
            }
        }
    }
    cells_data
}
