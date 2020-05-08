use super::{deployment_process::DeploymentProcess, plan::Plan, recipe::DeploymentRecipe};
use crate::config::Deployment;
use crate::util::ask_for_confirm;
use crate::wallet::{cli_types::LiveCell, Wallet};
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use ckb_tool::ckb_types::core::{Capacity, TransactionView};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

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

    /// create a snapshot in migration dir
    fn snapshot_recipe(&self, recipe: &DeploymentRecipe) -> Result<()> {
        let now: DateTime<Utc> = Utc::now();
        let snapshot_name = now.format("%Y-%m-%d-%H%M%S.toml").to_string();
        let mut path = self.migration_dir.clone();
        path.push(snapshot_name);
        let content = serde_json::to_vec(recipe)?;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(&content)?;
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

    fn collect_migration_live_cells(&self, wallet: &Wallet) -> Result<Vec<LiveCell>> {
        // read last migration
        let file_names: Vec<_> = fs::read_dir(&self.migration_dir)?
            .map(|d| d.map(|d| d.file_name()))
            .collect::<Result<_, _>>()?;
        let last_migration_file = file_names.into_iter().max();
        let mut cells = Vec::new();
        if last_migration_file.is_none() {
            return Ok(cells);
        }
        let last_migration_file = last_migration_file.unwrap();
        let recipe = self.load_snapshot(last_migration_file.into_string().unwrap())?;

        // query cells txs
        for tx_recipe in recipe.cell_txs {
            if let Some(tx) = wallet.query_transaction(&tx_recipe.tx_hash)? {
                cells.extend(tx_recipe.cells.iter().map(|cell| {
                    let output = &tx.transaction.inner.outputs[cell.index as usize];
                    LiveCell {
                        tx_hash: tx.transaction.hash.clone(),
                        index: cell.index,
                        capacity: output.capacity.value(),
                        mature: true,
                    }
                }));
            }
        }

        // query dep groups txs
        for tx_recipe in recipe.dep_group_txs {
            if let Some(tx) = wallet.query_transaction(&tx_recipe.tx_hash)? {
                cells.extend(tx_recipe.dep_groups.iter().map(|cell| {
                    let output = &tx.transaction.inner.outputs[cell.index as usize];
                    LiveCell {
                        tx_hash: tx.transaction.hash.clone(),
                        index: cell.index,
                        capacity: output.capacity.value(),
                        mature: true,
                    }
                }));
            }
        }

        Ok(cells)
    }

    pub fn deploy(&self, wallet: Wallet, opt: DeployOption) -> Result<()> {
        if !self.migration_dir.exists() {
            fs::create_dir_all(&self.migration_dir)?;
            println!("Create directory {:?}", self.migration_dir);
        }
        let mut pre_inputs = Vec::new();
        let deployment = self.deployment.clone();
        if opt.migrate {
            pre_inputs.extend(self.collect_migration_live_cells(&wallet)?);
        }
        let mut process = DeploymentProcess::new(deployment, wallet, opt.tx_fee);
        let (recipe, txs) = process.prepare_recipe(pre_inputs.clone())?;
        if txs.is_empty() {
            return Err(anyhow!("Nothing to deploy"));
        }
        self.output_deployment_plan(&recipe, &txs, &pre_inputs, &opt);
        if ask_for_confirm("Confirm deployment?")? {
            self.snapshot_recipe(&recipe)?;
            let txs = process.sign_txs(txs)?;
            process.execute_recipe(recipe, txs)?;
            println!("Deployment complete");
        } else {
            println!("Cancelled");
        }
        Ok(())
    }

    fn output_deployment_plan(
        &self,
        recipe: &DeploymentRecipe,
        txs: &[TransactionView],
        pre_inputs: &[LiveCell],
        opt: &DeployOption,
    ) {
        let migrated_capacity = pre_inputs.iter().map(|cell| cell.capacity).sum::<u64>();
        let total_used_capacity = txs
            .iter()
            .map(|tx| tx.outputs_capacity().expect("capacity").as_u64())
            .sum::<u64>();
        let new_capacity = total_used_capacity - migrated_capacity;
        let plan = Plan::new(
            migrated_capacity,
            new_capacity,
            total_used_capacity,
            opt.tx_fee.as_u64(),
            recipe.to_owned(),
        );
        let plan = serde_yaml::to_string(&plan).unwrap();
        println!("Deployment plan:");
        println!("{}", plan);
    }
}
