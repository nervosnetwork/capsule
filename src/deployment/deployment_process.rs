use super::recipe::{CellRecipe, DepGroupRecipe, DeploymentRecipe};
use super::tx_check::tx_check;
use super::BakedTransaction;
use crate::config::{Cell, CellLocation, DepGroup, Deployment};
use crate::wallet::{cli_types::LiveCell, *};

use anyhow::{anyhow, Result};
use ckb_tool::ckb_chain_spec::consensus::TYPE_ID_CODE_HASH;
use ckb_tool::ckb_hash::new_blake2b;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{Capacity, ScriptHashType, TransactionBuilder, TransactionView},
    packed,
    prelude::*,
    H256,
};
use log::{debug, log_enabled, Level::Debug};
use std::fs;
use std::io::Read;

pub struct DeploymentProcess {
    wallet: Wallet,
    tx_fee: Capacity,
    config: Deployment,
    recipe: DeploymentRecipe,
}

impl DeploymentProcess {
    pub fn new(
        config: Deployment,
        recipe: DeploymentRecipe,
        wallet: Wallet,
        tx_fee: Capacity,
    ) -> Self {
        DeploymentProcess {
            wallet,
            recipe,
            tx_fee,
            config,
        }
    }

    /// generate recipe and deploy
    pub fn prepare_recipe(
        &mut self,
        cells_pre_inputs: Vec<(String, LiveCell, Bytes)>,
        dep_groups_pre_inputs: Vec<(String, LiveCell, Bytes)>,
    ) -> Result<(DeploymentRecipe, BakedTransaction)> {
        self.check_pre_inputs_unlockable(&cells_pre_inputs)?;
        self.check_pre_inputs_unlockable(&dep_groups_pre_inputs)?;
        let cells: Vec<(Cell, Bytes)> = load_deployable_cells_data(&self.config.cells)?;
        let dep_groups = self.config.dep_groups.clone();
        let (cell_recipes, cells_tx) = self.build_cells_recipe(cells, cells_pre_inputs)?;
        let (dep_group_recipes, dep_groups_tx) =
            self.build_dep_groups_recipe(dep_groups, dep_groups_pre_inputs, &cell_recipes)?;
        if let Some(tx) = cells_tx.as_ref() {
            tx_check(&self.wallet, tx)?;
        }
        if let Some(tx) = dep_groups_tx.as_ref() {
            tx_check(&self.wallet, tx)?;
        }
        let recipe = DeploymentRecipe {
            cell_recipes,
            dep_group_recipes,
        };
        let baked_tx = BakedTransaction {
            cells: cells_tx,
            dep_groups: dep_groups_tx,
        };
        Ok((recipe, baked_tx))
    }

    fn check_pre_inputs_unlockable(
        &self,
        pre_inputs_cell: &[(String, LiveCell, Bytes)],
    ) -> Result<()> {
        let wallet_lock: packed::Script = self.wallet.lock_script();
        for (name, live_cell, _) in pre_inputs_cell {
            let cell_output: packed::CellOutput =
                self.wallet.get_cell_output(live_cell.out_point());
            if cell_output.lock() != wallet_lock {
                let address = self
                    .wallet
                    .address()
                    .display_with_network(self.wallet.address().network());
                return Err(anyhow!("Can't unlock previously deployed cells with address '{}'\ncell '{}' uses lock:\n{}\naddress's lock:\n{}\n\nhint: update the lock field in `deployment.toml` or turn off migration with option `--migrate=off`", address, name, cell_output.lock(), wallet_lock));
            }
            log::debug!(
                "pre_inputs_cell: name={}, tx_hash={:#x}, output_index={}",
                name,
                live_cell.tx_hash,
                live_cell.index
            );
        }
        Ok(())
    }

    fn build_cells_recipe(
        &mut self,
        // deployment.toml
        cells: Vec<(Cell, Bytes)>,
        // migrations/dev/xx.json
        pre_inputs_cells: Vec<(String, LiveCell, Bytes)>,
    ) -> Result<(Vec<CellRecipe>, Option<TransactionView>)> {
        // FIXME: should keep removed cells in migration file
        // [Transaction]:
        //    inputs = pre_inputs_cells + fuel_cells
        //    outputs = new_outputs + pre_inputs_cells

        let mut unchanged_cells = Vec::new();
        let mut changed_cells = Vec::new();
        let mut new_cells = Vec::new();
        for (cell, data) in &cells {
            if let Some((_, live_cell, pre_data)) = pre_inputs_cells
                .iter()
                .find(|(name, _, _)| name == &cell.name)
            {
                if pre_data == data {
                    println!("Unchanged cell: {}", cell.name);
                    unchanged_cells.push((cell, data, live_cell));
                } else {
                    println!("Changed cell: {}", cell.name);
                    changed_cells.push((cell, data, live_cell));
                }
            } else {
                println!("New cell: {}", cell.name);
                new_cells.push((cell, data));
            }
        }

        let mut cell_recipes = Vec::new();
        for (cell, _, _) in &unchanged_cells {
            let unchanged_recipe = self
                .recipe
                .cell_recipes
                .iter()
                .find(|cell_recipe| cell_recipe.name == cell.name)
                .expect("unchaged recipe");
            cell_recipes.push(unchanged_recipe.clone());
        }
        if new_cells.is_empty() && changed_cells.is_empty() {
            // No cells transaction needed
            return Ok((cell_recipes, None));
        }

        let lock: packed::Script = self.config.lock.to_owned().into();
        let mut inputs_cells = Vec::new();
        for (_cell, _data, live_cell) in &changed_cells {
            self.wallet
                .lock_out_points(vec![live_cell.out_point()].into_iter());
            inputs_cells.push((*live_cell).clone());
        }
        if inputs_cells.is_empty() {
            inputs_cells.extend(
                self.wallet
                    .collect_live_cells(Capacity::shannons(1))
                    .into_iter(),
            );
            self.wallet
                .lock_out_points(inputs_cells.iter().map(|c| c.out_point()));
        }

        let mut outputs = Vec::new();
        let mut outputs_data = Vec::new();
        for (output_index, (cell, data)) in new_cells.iter().enumerate() {
            let output = {
                let mut output = packed::CellOutput::new_builder().lock(lock.clone());
                if cell.enable_type_id {
                    let type_script =
                        build_type_id_script(&inputs_cells[0].input(), output_index as u64);
                    output = output.type_(Some(type_script).pack());
                }
                output
                    .build_exact_capacity(Capacity::bytes(data.len()).expect("bytes"))
                    .expect("build")
            };

            // NOTE: Update tx_hash when transaction built
            let recipe =
                build_cell_recipe(H256::default(), output_index as u32, &output, cell, data);

            cell_recipes.push(recipe);
            outputs.push(output);
            outputs_data.push((*data).clone());
        }
        for (idx, (cell, data, input_cell)) in changed_cells.iter().enumerate() {
            let output_index = new_cells.len() + idx;
            let output = {
                let mut output = packed::CellOutput::new_builder().lock(lock.clone());
                if cell.enable_type_id {
                    let tx: packed::Transaction = self
                        .wallet
                        .query_transaction(&input_cell.tx_hash)?
                        .expect("tx")
                        .transaction
                        .inner
                        .into();
                    let tx: TransactionView = tx.into_view();
                    let input_cell_output =
                        tx.outputs().get(input_cell.index as usize).expect("output");
                    // inherit type id from input cell or create a new one
                    let type_script = match input_cell_output.type_().to_opt() {
                        Some(script) if is_type_id_script(&script) => script,
                        _ => build_type_id_script(&input_cell.input(), output_index as u64),
                    };
                    output = output.type_(Some(type_script).pack());
                }
                output
                    .build_exact_capacity(Capacity::bytes(data.len()).expect("bytes"))
                    .expect("build")
            };

            // NOTE: Update tx_hash when transaction built
            let recipe =
                build_cell_recipe(H256::default(), output_index as u32, &output, cell, data);

            cell_recipes.push(recipe);
            outputs.push(output);
            outputs_data.push((*data).clone());
        }

        let tx = TransactionBuilder::default()
            .inputs(inputs_cells.iter().map(|cell| cell.input()))
            .outputs(outputs)
            .outputs_data(outputs_data.into_iter().map(|data| data.pack()).pack())
            .build();
        let tx = self.wallet.complete_tx_lock_deps(tx);
        let inputs_capacity = inputs_cells.iter().map(|cell| cell.capacity).sum::<u64>();
        let tx =
            self.wallet
                .complete_tx_inputs(tx, Capacity::shannons(inputs_capacity), self.tx_fee);
        let tx_hash: H256 = tx.hash().unpack();
        cell_recipes
            .iter_mut()
            .skip(unchanged_cells.len())
            .for_each(|cell_recipe| {
                cell_recipe.tx_hash = tx_hash.clone();
            });
        self.wallet.lock_tx_inputs(&tx);
        Ok((cell_recipes, Some(tx)))
    }

    fn build_dep_groups_recipe(
        &mut self,
        dep_groups: Vec<DepGroup>,
        pre_inputs_cells: Vec<(String, LiveCell, Bytes)>,
        cell_recipes: &[CellRecipe],
    ) -> Result<(Vec<DepGroupRecipe>, Option<TransactionView>)> {
        // FIXME: should keep removed dep groups in migration file
        let mut dep_groups_with_data = Vec::new();
        for dep_group in dep_groups {
            let mut out_points = Vec::new();
            for cell_name in &dep_group.cells {
                if let Some(cell_recipe) = cell_recipes
                    .iter()
                    .find(|cell_recipe| &cell_recipe.name == cell_name)
                {
                    out_points.push(packed::OutPoint::new(
                        cell_recipe.tx_hash.clone().pack(),
                        cell_recipe.index,
                    ));
                } else {
                    return Err(anyhow!(
                        "Can't find cell recipe by name '{}' for dep group: {}",
                        cell_name,
                        dep_group.name
                    ));
                }
            }
            let out_points_vec: packed::OutPointVec = out_points.pack();
            let data = out_points_vec.as_bytes();
            dep_groups_with_data.push((dep_group, data));
        }

        let mut unchanged_groups = Vec::new();
        let mut changed_groups = Vec::new();
        let mut new_groups = Vec::new();
        for (dep_group, data) in &dep_groups_with_data {
            if let Some((_name, live_cell, pre_data)) = pre_inputs_cells
                .iter()
                .find(|(name, _, _)| name == &dep_group.name)
            {
                if pre_data == data {
                    println!("Unchanged dep group: {}", dep_group.name);
                    unchanged_groups.push((dep_group, data, live_cell));
                } else {
                    println!("Changed dep group: {}", dep_group.name);
                    changed_groups.push((dep_group, data, live_cell));
                }
            } else {
                println!("New dep group: {}", dep_group.name);
                new_groups.push((dep_group, data));
            }
        }

        let mut group_recipes = Vec::new();
        for (dep_group, _, _) in &unchanged_groups {
            let unchanged_recipe = self
                .recipe
                .dep_group_recipes
                .iter()
                .find(|group_recipe| group_recipe.name == dep_group.name)
                .expect("unchaged recipe");
            group_recipes.push(unchanged_recipe.clone());
        }
        if new_groups.is_empty() && changed_groups.is_empty() {
            // No dep group transaction needed
            return Ok((group_recipes, None));
        }

        let lock: packed::Script = self.config.lock.to_owned().into();
        let mut inputs_cells = Vec::new();
        for (_cell, _data, live_cell) in &changed_groups {
            self.wallet
                .lock_out_points(vec![live_cell.out_point()].into_iter());
            inputs_cells.push((*live_cell).clone());
        }
        if inputs_cells.is_empty() {
            inputs_cells.extend(
                self.wallet
                    .collect_live_cells(Capacity::shannons(1))
                    .into_iter(),
            );
            self.wallet
                .lock_out_points(inputs_cells.iter().map(|c| c.out_point()));
        }

        let mut outputs = Vec::new();
        let mut outputs_data = Vec::new();
        for (output_index, (dep_group, data)) in new_groups.iter().enumerate() {
            let output = packed::CellOutput::new_builder()
                .lock(lock.clone())
                .build_exact_capacity(Capacity::bytes(data.len()).expect("bytes"))
                .expect("build");

            // NOTE: Update tx_hash when transaction built
            let recipe = build_dep_group_recipe(
                H256::default(),
                output_index as u32,
                &output,
                dep_group,
                data,
            );
            group_recipes.push(recipe);
            outputs.push(output);
            outputs_data.push((*data).clone());
        }
        for (idx, (dep_group, data, _)) in changed_groups.iter().enumerate() {
            let output_index = new_groups.len() + idx;
            let output = packed::CellOutput::new_builder()
                .lock(lock.clone())
                .build_exact_capacity(Capacity::bytes(data.len()).expect("bytes"))
                .expect("build");

            // NOTE: Update tx_hash when transaction built
            let recipe = build_dep_group_recipe(
                H256::default(),
                output_index as u32,
                &output,
                dep_group,
                data,
            );

            group_recipes.push(recipe);
            outputs.push(output);
            outputs_data.push((*data).clone());
        }

        let tx = TransactionBuilder::default()
            .inputs(inputs_cells.iter().map(|cell| cell.input()))
            .outputs(outputs)
            .outputs_data(outputs_data.into_iter().map(|data| data.pack()).pack())
            .build();
        let tx = self.wallet.complete_tx_lock_deps(tx);
        let inputs_capacity = inputs_cells.iter().map(|cell| cell.capacity).sum::<u64>();
        let tx =
            self.wallet
                .complete_tx_inputs(tx, Capacity::shannons(inputs_capacity), self.tx_fee);
        let tx_hash: H256 = tx.hash().unpack();
        group_recipes
            .iter_mut()
            .skip(unchanged_groups.len())
            .for_each(|group_recipe| {
                group_recipe.tx_hash = tx_hash.clone();
            });
        self.wallet.lock_tx_inputs(&tx);
        Ok((group_recipes, Some(tx)))
    }

    pub fn sign_baked_tx(&self, baked_tx: &mut BakedTransaction) -> Result<()> {
        let password = self.wallet.read_password().expect("read password");

        if let Some(tx) = baked_tx.cells.take() {
            baked_tx.cells = Some(self.wallet.sign_tx(tx, password.clone())?);
        }
        if let Some(tx) = baked_tx.dep_groups.take() {
            baked_tx.dep_groups = Some(self.wallet.sign_tx(tx, password)?);
        }
        Ok(())
    }

    pub fn execute_recipe(&mut self, baked_tx: BakedTransaction) -> Result<()> {
        if let Some(tx) = baked_tx.cells {
            let tx_hash: H256 = tx.hash().unpack();
            println!("Sending cells tx {}", tx_hash);

            if log_enabled!(Debug) {
                let tx_without_data = tx
                    .as_advanced_builder()
                    .set_outputs_data(Vec::new())
                    .build();
                debug!("send transaction error: {}", tx_without_data);
            }
            self.wallet.send_transaction(tx)?;
        }
        if let Some(tx) = baked_tx.dep_groups {
            let tx_hash: H256 = tx.hash().unpack();
            println!("Sending dep_group tx {}", tx_hash);

            if log_enabled!(Debug) {
                let tx_without_data = tx
                    .as_advanced_builder()
                    .set_outputs_data(Vec::new())
                    .build();
                debug!("send transaction error: {}", tx_without_data);
            }

            self.wallet.send_transaction(tx)?;
        }
        Ok(())
    }
}

fn build_cell_recipe(
    tx_hash: H256,
    output_index: u32,
    output: &packed::CellOutput,
    cell: &Cell,
    data: &Bytes,
) -> CellRecipe {
    let occupied_capacity = output
        .occupied_capacity(Capacity::bytes(data.len()).expect("capacity"))
        .expect("capacity")
        .as_u64();
    let type_id = if cell.enable_type_id {
        Some(
            output
                .type_()
                .to_opt()
                .expect("type id")
                .calc_script_hash()
                .unpack(),
        )
    } else {
        None
    };
    CellRecipe {
        index: output_index,
        name: cell.name.to_owned(),
        data_hash: packed::CellOutput::calc_data_hash(data).unpack(),
        occupied_capacity,
        tx_hash,
        type_id,
    }
}

fn build_dep_group_recipe(
    tx_hash: H256,
    output_index: u32,
    output: &packed::CellOutput,
    dep_group: &DepGroup,
    data: &Bytes,
) -> DepGroupRecipe {
    let occupied_capacity = output
        .occupied_capacity(Capacity::bytes(data.len()).expect("capacity"))
        .expect("capacity")
        .as_u64();
    DepGroupRecipe {
        tx_hash,
        index: output_index,
        name: dep_group.name.to_owned(),
        occupied_capacity,
    }
}

fn is_type_id_script(script: &packed::Script) -> bool {
    script.code_hash() == TYPE_ID_CODE_HASH.pack()
        && script.hash_type() == ScriptHashType::Type.into()
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

fn load_deployable_cells_data(cells: &[Cell]) -> Result<Vec<(Cell, Bytes)>> {
    let mut cells_data: Vec<(Cell, Bytes)> = Vec::new();
    for cell in cells {
        match cell.location.to_owned() {
            CellLocation::OutPoint { .. } => {}
            CellLocation::File { file } => {
                let mut data = Vec::new();
                match fs::File::open(&file).and_then(|mut f| f.read_to_end(&mut data)) {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("failed to read cell data from '{}', err: {}", file, &err);
                        return Err(err.into());
                    }
                }
                cells_data.push((cell.to_owned(), data.into()));
            }
        }
    }
    Ok(cells_data)
}
