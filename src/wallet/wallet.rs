use super::collector::Collector;
use super::util;

use anyhow::Result;
use ckb_tool::ckb_jsonrpc_types::{LiveCell, TransactionWithStatus};
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{BlockView, Capacity, DepType, ScriptHashType, TransactionBuilder, TransactionView},
    packed,
    prelude::*,
    H256,
};
use ckb_tool::faster_hex::hex_encode;
use ckb_tool::rpc_client::RpcClient;
use std::process::Command;

pub const DEFAULT_CKB_CLI_BIN_NAME: &str = "ckb-cli";
pub const DEFAULT_CKB_RPC_URL: &str = "http://localhost:8114";

pub struct Wallet {
    bin: String,
    rpc_client: RpcClient,
    lock_arg: [u8; 20],
    genesis: BlockView,
    collector: Collector,
}

impl Wallet {
    pub fn load(ckb_cli_bin: String, rpc_client: RpcClient, lock_arg: [u8; 20]) -> Self {
        let genesis = rpc_client
            .get_block_by_number(0u64.into())
            .expect("genesis");
        let collector = Collector::new();
        Wallet {
            bin: ckb_cli_bin,
            rpc_client,
            lock_arg,
            genesis: genesis.into(),
            collector,
        }
    }

    pub fn complete_tx_lock_deps(&self, tx: TransactionView) -> TransactionView {
        let tx_hash = self.genesis.transactions().get(1).unwrap().hash();
        let out_point = packed::OutPoint::new_builder()
            .tx_hash(tx_hash)
            .index(0u32.pack())
            .build();
        let cell_dep = packed::CellDep::new_builder()
            .out_point(out_point)
            .dep_type(DepType::DepGroup.into())
            .build();
        tx.as_advanced_builder().cell_dep(cell_dep).build()
    }

    pub fn complete_tx_inputs(
        &self,
        tx: TransactionView,
        inputs_live_cells: Vec<LiveCell>,
        fee: Capacity,
    ) -> TransactionView {
        // create change cell
        let (change_output, change_occupied_capacity) = {
            let change_output = packed::CellOutput::new_builder()
                .lock(self.lock_script())
                .build();
            let capacity: Capacity = change_output.occupied_capacity(Capacity::zero()).unwrap();
            let change_output = change_output
                .as_builder()
                .capacity(capacity.as_u64().pack())
                .build();
            (change_output, capacity)
        };
        // calculate required capacity
        let required_capacity = tx
            .outputs_capacity()
            .expect("outputs_capacity")
            .safe_add(fee)
            .expect("capacity")
            .safe_add(change_occupied_capacity)
            .expect("capacity");
        // collect inputs
        let live_cells = self.collect_live_cells(required_capacity);
        let inputs_capacity = live_cells
            .iter()
            .map(|c| {
                let capacity: u64 = c.cell_output.capacity.into();
                capacity
            })
            .sum::<u64>();
        let mut inputs: Vec<_> = tx.inputs().into_iter().collect();
        inputs.extend(live_cells.into_iter().map(|cell| {
            let out_point = packed::OutPoint::new_builder()
                .tx_hash(cell.created_by.tx_hash.pack())
                .index((cell.created_by.index.value() as u32).pack())
                .build();
            packed::CellInput::new_builder()
                .previous_output(out_point)
                .build()
        }));
        let original_inputs_capacity = inputs_live_cells
            .iter()
            .map(|c| {
                let capacity: u64 = c.cell_output.capacity.into();
                capacity
            })
            .sum::<u64>();
        // calculate change capacity
        let change_capacity =
            original_inputs_capacity + inputs_capacity - required_capacity.as_u64();
        let change_output = change_output
            .as_builder()
            .capacity(change_capacity.pack())
            .build();
        let tx = tx
            .as_advanced_builder()
            .inputs(inputs)
            .output(change_output)
            .output_data(Default::default())
            .build();
        tx
    }

    pub fn sign_tx(&self, tx: TransactionView) -> TransactionView {
        let witnesses_len = tx.witnesses().len();
        let message: [u8; 32] = util::tx_sign_message(tx, 0, witnesses_len).into();
        let lock_arg_hex = {
            let mut dst = [0u8; 40];
            hex_encode(&self.lock_arg, &mut dst).expect("hex");
            String::from_utf8(dst.to_vec()).expect("utf8")
        };
        let message_hex = {
            let mut dst = [0u8; 64];
            hex_encode(&message, &mut dst).expect("hex");
            String::from_utf8(dst.to_vec()).expect("utf8")
        };
        let output = self
            .build_cmd()
            .arg("util")
            .arg("sign-message")
            .arg("--output-format")
            .arg("json")
            .arg("--from-account")
            .arg(lock_arg_hex)
            .arg("--message")
            .arg(message_hex)
            .output()
            .expect("sign tx");
        println!(
            "sign tx output {}",
            String::from_utf8(output.stdout).unwrap()
        );
        let signature = unreachable!();
        let tx = util::attach_signature(tx, signature, 0);
        tx
    }

    pub fn query_transaction(&self, tx_hash: &[u8; 32]) -> Result<Option<TransactionWithStatus>> {
        let tx_hash: H256 = tx_hash.to_owned().into();
        let tx_opt = self.rpc_client().get_transaction(tx_hash);
        Ok(tx_opt)
    }

    pub fn send_transaction(&mut self, tx: TransactionView) -> Result<[u8; 32]> {
        let tx_hash: packed::Byte32 = self.rpc_client().send_transaction(tx.data().into());
        for out_point in tx.input_pts_iter() {
            self.collector.lock_cell(out_point);
        }
        Ok(tx_hash.unpack())
    }

    pub fn collect_live_cells(&self, capacity: Capacity) -> Vec<LiveCell> {
        self.collector
            .collect_live_cells(self.rpc_client(), self.lock_hash(), capacity)
    }

    fn lock_script(&self) -> packed::Script {
        let output = self
            .genesis
            .transactions()
            .get(0)
            .unwrap()
            .outputs()
            .get(1)
            .unwrap();
        let type_id = output
            .type_()
            .to_opt()
            .as_ref()
            .expect("lock script type id")
            .calc_script_hash();
        let lock_script = packed::Script::new_builder()
            .code_hash(type_id)
            .hash_type(ScriptHashType::Type.into())
            .args(Bytes::from(self.lock_arg.to_vec()).pack())
            .build();
        lock_script
    }

    fn lock_hash(&self) -> packed::Byte32 {
        self.lock_script().calc_script_hash()
    }

    fn rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }

    fn build_cmd(&self) -> Command {
        Command::new(&self.bin)
    }
}
