use super::BakedTransaction;
use super::{deployment_process::DeploymentProcess, plan::Plan, recipe::DeploymentRecipe};
use crate::config::Deployment;
use crate::util::cli::ask_for_confirm;
use crate::wallet::{cli_types::LiveCell, Wallet};
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use ckb_tool::ckb_types::{bytes::Bytes, core::Capacity};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

const CURRENT_SNAPSHOT: &str = "current.json";

#[derive(Clone, Copy, Debug)]
pub struct DeployOption {
    pub migrate: bool,
    pub tx_fee: Capacity,
}

/// Deployment manage
/// 1. manage migrations
/// 2, handle deploy new / rerun / migrate
pub struct Manage {
    migration_dir: PathBuf,
    deployment: Deployment,
}

impl Manage {
    pub fn new(migration_dir: PathBuf, deployment: Deployment) -> Self {
        Manage {
            migration_dir,
            deployment,
        }
    }

    /// check current snapshot
    fn check_incomplete_snapshot(&self) -> Result<()> {
        let mut path = self.migration_dir.clone();
        path.push(CURRENT_SNAPSHOT);
        if path.exists() {
            return Err(anyhow!("Find a incomplete deployment record {:?}. Please take look at the deployment record file and manualy fix the incomplete deployment.", path));
        }
        Ok(())
    }

    /// create a snapshot in migration dir
    fn snapshot_recipe(&self, recipe: &DeploymentRecipe) -> Result<PathBuf> {
        let mut path = self.migration_dir.clone();
        path.push(CURRENT_SNAPSHOT);
        let content = serde_json::to_vec_pretty(recipe)?;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?
            .write_all(&content)?;
        Ok(path)
    }

    /// complete a snapshot
    fn complete_snapshot(&self, src: PathBuf) -> Result<()> {
        let now: DateTime<Utc> = Utc::now();
        let snapshot_name = now.format("%Y-%m-%d-%H%M%S.json").to_string();
        let mut path = self.migration_dir.clone();
        path.push(snapshot_name);
        fs::copy(&src, &path)?;
        fs::remove_file(src)?;
        Ok(())
    }

    fn load_snapshot(&self, snapshot_name: String) -> Result<DeploymentRecipe> {
        let mut path = self.migration_dir.clone();
        path.push(snapshot_name);
        let mut buf = Vec::new();
        fs::File::open(path)?.read_to_end(&mut buf)?;
        let recipe = serde_json::from_slice(&buf)?;
        Ok(recipe)
    }

    fn collect_migration_live_cells(
        &self,
        wallet: &Wallet,
    ) -> Result<(
        DeploymentRecipe,
        Vec<(String, LiveCell, Bytes)>,
        Vec<(String, LiveCell, Bytes)>,
    )> {
        // read last migration
        let file_names: Vec<_> = fs::read_dir(&self.migration_dir)?
            .map(|d| d.map(|d| d.file_name()))
            .collect::<Result<_, _>>()?;
        let last_migration_file = file_names.into_iter().max();
        let mut cells = Vec::new();
        let mut dep_groups = Vec::new();
        if last_migration_file.is_none() {
            return Ok((Default::default(), cells, dep_groups));
        }
        let last_migration_file = last_migration_file.unwrap();
        let recipe = self.load_snapshot(last_migration_file.into_string().unwrap())?;

        // query cells recipes
        for cell in recipe.cell_recipes.clone() {
            if let Some(tx) = wallet.query_transaction(&cell.tx_hash)? {
                let output = &tx.transaction.inner.outputs[cell.index as usize];
                let output_data = tx.transaction.inner.outputs_data[cell.index as usize]
                    .clone()
                    .into_bytes();
                let live_cell = LiveCell {
                    tx_hash: tx.transaction.hash.clone(),
                    index: cell.index,
                    capacity: output.capacity.value(),
                    mature: true,
                };
                log::debug!("Got cell_recipes pre-input: name={}", cell.name);
                cells.push((cell.name.clone(), live_cell, output_data));
            }
        }

        // query dep groups recipes
        for dep_group in recipe.dep_group_recipes.clone() {
            if let Some(tx) = wallet.query_transaction(&dep_group.tx_hash)? {
                let output = &tx.transaction.inner.outputs[dep_group.index as usize];
                let output_data = tx.transaction.inner.outputs_data[dep_group.index as usize]
                    .clone()
                    .into_bytes();
                let live_cell = LiveCell {
                    tx_hash: tx.transaction.hash.clone(),
                    index: dep_group.index,
                    capacity: output.capacity.value(),
                    mature: true,
                };
                log::debug!("Got dep_group_recipes pre-input: name={}", dep_group.name);
                dep_groups.push((dep_group.name.clone(), live_cell, output_data));
            }
        }

        Ok((recipe, cells, dep_groups))
    }

    pub fn deploy(&self, wallet: Wallet, opt: DeployOption) -> Result<()> {
        if !self.migration_dir.exists() {
            fs::create_dir_all(&self.migration_dir)?;
            println!("Create directory {:?}", self.migration_dir);
        }
        // check incomplete snapshot
        self.check_incomplete_snapshot()?;
        let deployment = self.deployment.clone();
        let (deployment_recipe, cells_pre_inputs, dep_groups_pre_inputs) = if opt.migrate {
            self.collect_migration_live_cells(&wallet)?
        } else {
            (DeploymentRecipe::default(), Vec::new(), Vec::new())
        };
        let mut process = DeploymentProcess::new(deployment, deployment_recipe, wallet, opt.tx_fee);
        let (recipe, mut baked_tx) =
            process.prepare_recipe(cells_pre_inputs.clone(), dep_groups_pre_inputs.clone())?;
        if baked_tx.is_empty() {
            return Err(anyhow!("Nothing to deploy"));
        }
        self.output_deployment_plan(
            &recipe,
            &baked_tx,
            &cells_pre_inputs[..],
            &dep_groups_pre_inputs[..],
            &opt,
        );
        if ask_for_confirm("Confirm deployment?")? {
            process.sign_baked_tx(&mut baked_tx)?;
            let snapshot_path = self.snapshot_recipe(&recipe)?;
            process.execute_recipe(baked_tx)?;
            self.complete_snapshot(snapshot_path)?;
            println!("Deployment complete");
        } else {
            println!("Cancelled");
        }
        Ok(())
    }

    fn output_deployment_plan(
        &self,
        recipe: &DeploymentRecipe,
        baked_tx: &BakedTransaction,
        cells_pre_inputs: &[(String, LiveCell, Bytes)],
        dep_groups_pre_inputs: &[(String, LiveCell, Bytes)],
        opt: &DeployOption,
    ) {
        // TODO: Should not include unchanged cells
        let migrated_capacity = cells_pre_inputs
            .iter()
            .chain(dep_groups_pre_inputs.iter())
            .map(|(_name, cell, _)| cell.capacity)
            .sum::<u64>();
        let total_occupied_capacity = baked_tx
            .cells
            .iter()
            .chain(baked_tx.dep_groups.iter())
            .map(|tx| {
                tx.outputs_with_data_iter()
                    .filter_map(|(output, data)| {
                        if data.is_empty() {
                            None
                        } else {
                            let data_capacity = Capacity::bytes(data.len()).expect("bytes");
                            Some(
                                output
                                    .occupied_capacity(data_capacity)
                                    .expect("occupied")
                                    .as_u64(),
                            )
                        }
                    })
                    .sum::<u64>()
            })
            .sum::<u64>();
        let new_capacity = total_occupied_capacity - migrated_capacity;
        let plan = Plan::new(
            migrated_capacity,
            new_capacity,
            total_occupied_capacity,
            opt.tx_fee.as_u64() * baked_tx.len() as u64,
            recipe.to_owned(),
        );
        let plan = serde_yaml::to_string(&plan).unwrap();
        println!("Deployment plan:");
        println!("{}", plan);
    }
}
