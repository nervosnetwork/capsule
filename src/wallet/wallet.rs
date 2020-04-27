use super::cli_types::{Address, LiveCellInfo, SignatureOutput};
use super::collector::Collector;
use super::password::Password;
use super::util;

use anyhow::Result;
use ckb_tool::ckb_jsonrpc_types::{TransactionWithStatus};
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{BlockView, Capacity, DepType, TransactionView},
    packed,
    prelude::*,
    H256,
};
use ckb_tool::faster_hex::hex_decode;
use ckb_tool::faster_hex::hex_encode;
use ckb_tool::rpc_client::RpcClient;
use std::collections::HashSet;
use std::io::Write;
use std::process::{Command, Stdio};

pub const DEFAULT_CKB_CLI_BIN_NAME: &str = "ckb-cli";
pub const DEFAULT_CKB_RPC_URL: &str = "http://localhost:8114";

pub struct Wallet {
    bin: String,
    rpc_client: RpcClient,
    address: Address,
    genesis: BlockView,
    collector: Collector,
}

impl Wallet {
    pub fn load(ckb_cli_bin: String, rpc_client: RpcClient, address: Address) -> Self {
        let genesis = rpc_client
            .get_block_by_number(0u64.into())
            .expect("genesis");
        let collector = Collector::new(ckb_cli_bin.clone());
        Wallet {
            bin: ckb_cli_bin,
            rpc_client,
            address,
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

    pub fn complete_tx_inputs<'a>(
        &self,
        tx: TransactionView,
        inputs_live_cells: impl Iterator<Item = &'a LiveCellInfo>,
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
                let capacity: u64 = c.capacity();
                capacity
            })
            .sum::<u64>();
        let mut inputs: Vec<_> = tx.inputs().into_iter().collect();
        inputs.extend(live_cells.into_iter().map(|cell| {
            cell.input()
        }));
        let original_inputs_capacity = inputs_live_cells
            .map(|c| {
                let capacity: u64 = c.capacity();
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
            .set_inputs(inputs)
            .output(change_output)
            .output_data(Default::default())
            .build();
        tx
    }

    pub fn read_password(&self) -> Result<Password> {
        println!("Password:");
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        Ok(Password::new(buf))
    }

    pub fn sign_tx(&self, tx: TransactionView, password: Password) -> Result<TransactionView> {
        // complete witnesses
        let mut witnesses: Vec<Bytes> = tx.witnesses().unpack();
        if witnesses.is_empty() {
            // input group witness
            witnesses.push(packed::WitnessArgs::default().as_bytes());
        }
        witnesses.extend(
            (witnesses.len()..tx.inputs().len())
                .into_iter()
                .map(|_| Bytes::new()),
        );
        let tx = tx.as_advanced_builder().witnesses(witnesses.pack()).build();
        let witnesses_len = tx.witnesses().len();
        let message: [u8; 32] = util::tx_sign_message(&tx, 0, witnesses_len).into();
        let address_hex = self
            .address()
            .display_with_network(self.address().network());
        let message_hex = {
            let mut dst = [0u8; 64];
            hex_encode(&message, &mut dst).expect("hex");
            String::from_utf8(dst.to_vec()).expect("utf8")
        };
        let mut child = Command::new(&self.bin)
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .arg("util")
            .arg("sign-message")
            .arg("--recoverable")
            .arg("--output-format")
            .arg("json")
            .arg("--from-account")
            .arg(address_hex)
            .arg("--message")
            .arg(message_hex)
            .spawn()?;
        unsafe {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin
                .write_all(password.take().as_bytes())
                .expect("Failed to write to stdin");
        }

        let output = util::handle_cmd(child.wait_with_output()?).expect("sign tx");
        let output = String::from_utf8(output).expect("parse utf8");
        let output = output.trim_start_matches("Password:").trim();
        let output: SignatureOutput = serde_json::from_str(output).expect("parse json");
        if !output.recoverable {
            panic!("expect recoverable signature")
        }
        let output_signature = output.signature.trim_start_matches("0x");
        let mut signature = [0u8; 65];
        hex_decode(output_signature.as_bytes(), &mut signature).expect("dehex");
        let tx = util::attach_signature(tx, signature.to_vec().into(), 0);
        Ok(tx)
    }

    pub fn query_transaction(&self, tx_hash: &[u8; 32]) -> Result<Option<TransactionWithStatus>> {
        let tx_hash: H256 = tx_hash.to_owned().into();
        let tx_opt = self.rpc_client().get_transaction(tx_hash);
        Ok(tx_opt)
    }

    pub fn send_transaction(&mut self, tx: TransactionView) -> Result<H256> {
        let tx_hash: packed::Byte32 = self.rpc_client().send_transaction(tx.data().into());
        self.lock_tx_inputs(&tx);
        Ok(tx_hash.unpack())
    }

    fn lock_out_points(&mut self, out_points: impl Iterator<Item = packed::OutPoint>) {
        for out_point in out_points {
            self.collector.lock_cell(out_point);
        }
    }

    pub fn lock_cells(&mut self, cells: impl Iterator<Item = LiveCellInfo>) {
        let out_points = cells.map(|cell| {
            packed::OutPoint::new_builder()
                .tx_hash(cell.tx_hash.pack())
                .index(cell.index.output_index.pack())
                .build()
        });
        self.lock_out_points(out_points);
    }

    pub fn lock_tx_inputs(&mut self, tx: &TransactionView) {
        self.lock_out_points(tx.input_pts_iter());
    }

    pub fn collect_live_cells(&self, capacity: Capacity) -> HashSet<LiveCellInfo> {
        self.collector
            .collect_live_cells(self.address().to_owned(), capacity)
    }

    fn lock_script(&self) -> packed::Script {
        self.address().payload().into()
    }

    fn address(&self) -> &Address {
        &self.address
    }

    fn rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }
}
