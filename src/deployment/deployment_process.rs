use super::recipe::*;
use crate::config::{Cell, CellLocation, DepGroup, Deployment};
use crate::wallet::{cli_types::LiveCell, *};

use anyhow::Result;
use ckb_tool::ckb_chain_spec::consensus::TYPE_ID_CODE_HASH;
use ckb_tool::ckb_hash::new_blake2b;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{Capacity, ScriptHashType, TransactionBuilder, TransactionView},
    packed,
    prelude::*,
    H256,
};
use std::fs;
use std::io::Read;

pub struct DeploymentProcess {
    wallet: Wallet,
    tx_fee: Capacity,
    config: Deployment,
}

impl DeploymentProcess {
    pub fn new(config: Deployment, wallet: Wallet, tx_fee: Capacity) -> Self {
        DeploymentProcess {
            wallet,
            tx_fee,
            config,
        }
    }

    /// generate recipe and deploy
    pub fn prepare_recipe(
        &mut self,
        pre_inputs_cells: Vec<LiveCell>,
    ) -> Result<(DeploymentRecipe, Vec<TransactionView>)> {
        let cells: Vec<(Cell, Bytes)> = load_deployable_cells_data(&self.config.cells);
        let dep_groups = self.config.dep_groups.clone();
        Ok(self.build_recipe(cells, dep_groups, pre_inputs_cells))
    }

    fn build_cells_tx(
        &mut self,
        deployable_cells: &[(Cell, Bytes)],
        mut pre_inputs_cells: Vec<LiveCell>,
    ) -> TransactionView {
        let lock: packed::Script = self.config.lock.to_owned().into();
        // type_id requires at least one input
        if pre_inputs_cells.is_empty() {
            pre_inputs_cells.extend(
                self.wallet
                    .collect_live_cells(Capacity::shannons(1))
                    .into_iter()
                    .map(|i| i.into()),
            );
        }
        self.wallet.lock_cells(pre_inputs_cells.clone().into_iter());
        let inputs: Vec<_> = pre_inputs_cells
            .iter()
            .map(|live_cell| {
                packed::CellInput::new_builder()
                    .previous_output(live_cell.out_point())
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
        let inputs_capacity = pre_inputs_cells
            .iter()
            .map(|cell| cell.capacity)
            .sum::<u64>();
        let tx =
            self.wallet
                .complete_tx_inputs(tx, Capacity::shannons(inputs_capacity), self.tx_fee);
        self.wallet.lock_tx_inputs(&tx);
        tx
    }

    fn build_dep_groups_tx(
        &mut self,
        cell_txs: &[CellTxRecipe],
        deployable_dep_groups: &[DepGroup],
        pre_inputs_cells: Vec<LiveCell>,
    ) -> TransactionView {
        fn find_cell(name: &str, cell_txs: &[CellTxRecipe]) -> Option<(H256, CellRecipe)> {
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
        let inputs: Vec<_> = pre_inputs_cells
            .iter()
            .map(|live_cell| {
                packed::CellInput::new_builder()
                    .previous_output(live_cell.out_point())
                    .build()
            })
            .collect();
        let inputs_capacity = pre_inputs_cells
            .iter()
            .map(|cell| cell.capacity)
            .sum::<u64>();
        let tx = TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs.pack())
            .outputs_data(cells_data.pack())
            .build();
        let tx = self.wallet.complete_tx_lock_deps(tx);
        let tx =
            self.wallet
                .complete_tx_inputs(tx, Capacity::shannons(inputs_capacity), self.tx_fee);
        self.wallet.lock_tx_inputs(&tx);
        tx
    }

    fn build_recipe(
        &mut self,
        cells: Vec<(Cell, Bytes)>,
        dep_groups: Vec<DepGroup>,
        pre_inputs_cells: Vec<LiveCell>,
    ) -> (DeploymentRecipe, Vec<TransactionView>) {
        let mut txs = Vec::new();
        let mut cell_txs = Vec::new();
        let mut dep_group_txs = Vec::new();
        // build cells tx
        if !cells.is_empty() {
            let tx = self.build_cells_tx(&cells, pre_inputs_cells);
            let cells_tx_recipe = build_cell_tx_recipe(&tx, &cells);
            txs.push(tx);
            cell_txs.push(cells_tx_recipe);
        }
        // build dep_groups tx
        if !dep_groups.is_empty() {
            let tx = self.build_dep_groups_tx(&cell_txs, &dep_groups, Vec::new());
            let dep_group_tx_recipe = build_dep_group_tx_recipe(&tx, &dep_groups);
            txs.push(tx);
            dep_group_txs.push(dep_group_tx_recipe)
        }
        // construct deployment recipe
        let recipe = DeploymentRecipe {
            cell_txs,
            dep_group_txs,
        };
        (recipe, txs)
    }

    pub fn sign_txs(&self, txs: Vec<TransactionView>) -> Result<Vec<TransactionView>> {
        let password = self.wallet.read_password().expect("read password");
        txs.into_iter()
            .map(|tx| self.wallet.sign_tx(tx, password.clone()))
            .collect()
    }

    pub fn execute_recipe(
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
                    let tx_hash = tx.hash().unpack();
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
                    let tx_hash = tx.hash().unpack();
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
            .map(|(i, (c, data))| {
                let occupied_capacity = tx
                    .outputs()
                    .get(i)
                    .expect("get cell")
                    .occupied_capacity(
                        Capacity::bytes(tx.outputs_data().get(i).expect("get data").len())
                            .expect("capacity"),
                    )
                    .expect("capacity")
                    .as_u64();
                CellRecipe {
                    index: i as u32,
                    name: c.name.to_owned(),
                    data_hash: packed::CellOutput::calc_data_hash(&data).unpack(),
                    occupied_capacity,
                }
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
            .map(|(i, dep_group)| {
                let occupied_capacity = tx
                    .outputs()
                    .get(i)
                    .expect("get cell")
                    .occupied_capacity(
                        Capacity::bytes(tx.outputs_data().get(i).expect("get data").len())
                            .expect("capacity"),
                    )
                    .expect("capacity")
                    .as_u64();
                DepGroupRecipe {
                    index: i as u32,
                    name: dep_group.name.to_owned(),
                    occupied_capacity,
                }
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
