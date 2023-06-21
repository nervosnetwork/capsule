use crate::tx_verifier::OutputsDataVerifier;
use ckb_chain_spec::consensus::{ConsensusBuilder, TYPE_ID_CODE_HASH};
use ckb_error::Error as CKBError;
use ckb_script::{TransactionScriptsVerifier, TxVerifyEnv};
use ckb_traits::{CellDataProvider, ExtensionProvider, HeaderProvider};
use ckb_types::{
    bytes::Bytes,
    core::{
        cell::{CellMeta, CellMetaBuilder, ResolvedTransaction},
        hardfork::{HardForks, CKB2021, CKB2023},
        Capacity, Cycle, DepType, EpochExt, HeaderBuilder, HeaderView, ScriptHashType,
        TransactionInfo, TransactionView,
    },
    packed::{Byte32, CellDep, CellOutput, OutPoint, Script},
    prelude::*,
};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Return a random hash
pub fn random_hash() -> Byte32 {
    let mut rng = thread_rng();
    let mut buf = [0u8; 32];
    rng.fill(&mut buf);
    buf.pack()
}

/// Return a random OutPoint
pub fn random_out_point() -> OutPoint {
    OutPoint::new_builder().tx_hash(random_hash()).build()
}

/// Return a random Type ID Script
pub fn random_type_id_script() -> Script {
    let args = random_hash().as_bytes();
    debug_assert_eq!(args.len(), 32);
    Script::new_builder()
        .code_hash(TYPE_ID_CODE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(args.pack())
        .build()
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Message {
    pub id: Byte32,
    pub message: String,
}

/// Verification Context
#[derive(Clone, Default)]
pub struct Context {
    pub cells: HashMap<OutPoint, (CellOutput, Bytes)>,
    pub transaction_infos: HashMap<OutPoint, TransactionInfo>,
    pub headers: HashMap<Byte32, HeaderView>,
    pub epoches: HashMap<Byte32, EpochExt>,
    pub block_extensions: HashMap<Byte32, Bytes>,
    pub cells_by_data_hash: HashMap<Byte32, OutPoint>,
    pub cells_by_type_hash: HashMap<Byte32, OutPoint>,
    capture_debug: bool,
    captured_messages: Arc<Mutex<Vec<Message>>>,
}

impl Context {
    #[deprecated(since = "0.1.1", note = "Please use the deploy_cell function instead")]
    pub fn deploy_contract(&mut self, data: Bytes) -> OutPoint {
        self.deploy_cell(data)
    }

    /// Deploy a cell
    /// return the out-point of the cell
    pub fn deploy_cell(&mut self, data: Bytes) -> OutPoint {
        let data_hash = CellOutput::calc_data_hash(&data);
        if let Some(out_point) = self.cells_by_data_hash.get(&data_hash) {
            // contract has been deployed
            return out_point.to_owned();
        }
        let mut rng = thread_rng();
        let tx_hash = {
            let mut buf = [0u8; 32];
            rng.fill(&mut buf);
            buf.pack()
        };
        let out_point = OutPoint::new(tx_hash, 0);
        let type_id_script = random_type_id_script();
        let type_id_hash = type_id_script.calc_script_hash();
        let cell = {
            let cell = CellOutput::new_builder()
                .type_(Some(type_id_script).pack())
                .build();
            let occupied_capacity = cell
                .occupied_capacity(Capacity::bytes(data.len()).expect("data occupied capacity"))
                .expect("cell capacity");
            cell.as_builder().capacity(occupied_capacity.pack()).build()
        };
        self.cells.insert(out_point.clone(), (cell, data));
        self.cells_by_data_hash.insert(data_hash, out_point.clone());
        self.cells_by_type_hash
            .insert(type_id_hash, out_point.clone());
        out_point
    }

    /// Insert a block header into context
    pub fn insert_header(&mut self, header: HeaderView) {
        self.headers.insert(header.hash(), header);
    }

    /// Link a cell with a block
    /// to make the load_header_by_cell syscalls works
    pub fn link_cell_with_block(
        &mut self,
        out_point: OutPoint,
        block_hash: Byte32,
        tx_index: usize,
    ) {
        let header = self
            .headers
            .get(&block_hash)
            .expect("can't find the header");
        self.transaction_infos.insert(
            out_point,
            TransactionInfo::new(header.number(), header.epoch(), block_hash, tx_index),
        );
    }

    #[deprecated(
        since = "0.1.1",
        note = "Please use the get_cell_by_data_hash function instead"
    )]
    pub fn get_contract_out_point(&self, data_hash: &Byte32) -> Option<OutPoint> {
        self.get_cell_by_data_hash(data_hash)
    }

    /// Get the out-point of a cell by data_hash
    /// the cell must has deployed to this context
    pub fn get_cell_by_data_hash(&self, data_hash: &Byte32) -> Option<OutPoint> {
        self.cells_by_data_hash.get(data_hash).cloned()
    }

    /// Create a cell with data
    /// return the out-point
    pub fn create_cell(&mut self, cell: CellOutput, data: Bytes) -> OutPoint {
        let out_point = random_out_point();
        self.create_cell_with_out_point(out_point.clone(), cell, data);
        out_point
    }

    /// Create cell with specified out-point and cell data
    pub fn create_cell_with_out_point(
        &mut self,
        out_point: OutPoint,
        cell: CellOutput,
        data: Bytes,
    ) {
        let data_hash = CellOutput::calc_data_hash(&data);
        self.cells_by_data_hash.insert(data_hash, out_point.clone());
        if let Some(_type) = cell.type_().to_opt() {
            let type_hash = _type.calc_script_hash();
            self.cells_by_type_hash.insert(type_hash, out_point.clone());
        }
        self.cells.insert(out_point, (cell, data));
    }

    #[deprecated(
        since = "0.1.1",
        note = "Please use the create_cell_with_out_point function instead"
    )]
    pub fn insert_cell(&mut self, out_point: OutPoint, cell: CellOutput, data: Bytes) {
        self.create_cell_with_out_point(out_point, cell, data)
    }

    /// Get cell output and data by out-point
    pub fn get_cell(&self, out_point: &OutPoint) -> Option<(CellOutput, Bytes)> {
        self.cells.get(out_point).cloned()
    }

    /// Build script with out_point, hash_type, args
    /// return none if the out-point is not exist
    pub fn build_script_with_hash_type(
        &mut self,
        out_point: &OutPoint,
        hash_type: ScriptHashType,
        args: Bytes,
    ) -> Option<Script> {
        let (cell, contract_data) = self.cells.get(out_point)?;
        let code_hash = match hash_type {
            ScriptHashType::Data | ScriptHashType::Data1 | ScriptHashType::Data2 => {
                CellOutput::calc_data_hash(contract_data)
            }
            ScriptHashType::Type => cell
                .type_()
                .to_opt()
                .expect("get cell's type hash")
                .calc_script_hash(),
        };
        Some(
            Script::new_builder()
                .code_hash(code_hash)
                .hash_type(hash_type.into())
                .args(args.pack())
                .build(),
        )
    }
    /// Build script with out_point, args and hash_type(ScriptHashType::Type)
    /// return none if the out-point is not exist
    pub fn build_script(&mut self, out_point: &OutPoint, args: Bytes) -> Option<Script> {
        self.build_script_with_hash_type(out_point, ScriptHashType::Type, args)
    }

    fn find_cell_dep_for_script(&self, script: &Script) -> CellDep {
        let out_point = match ScriptHashType::try_from(u8::from(script.hash_type()))
            .expect("invalid script hash type")
        {
            ScriptHashType::Data | ScriptHashType::Data1 | ScriptHashType::Data2 => self
                .get_cell_by_data_hash(&script.code_hash())
                .expect("find contract out point by data_hash"),
            ScriptHashType::Type => self
                .cells_by_type_hash
                .get(&script.code_hash())
                .cloned()
                .expect("find contract out point by type_hash"),
        };

        CellDep::new_builder()
            .out_point(out_point)
            .dep_type(DepType::Code.into())
            .build()
    }

    /// Complete cell deps for a transaction
    /// this function searches context cells; generate cell dep for referenced scripts.
    pub fn complete_tx(&mut self, tx: TransactionView) -> TransactionView {
        let mut cell_deps: Vec<CellDep> = Vec::new();

        for cell_dep in tx.cell_deps_iter() {
            cell_deps.push(cell_dep);
        }

        for i in tx.input_pts_iter() {
            if let Some((cell, _data)) = self.cells.get(&i) {
                let dep = self.find_cell_dep_for_script(&cell.lock());
                if !cell_deps.contains(&dep) {
                    cell_deps.push(dep);
                }
                if let Some(script) = cell.type_().to_opt() {
                    let dep = self.find_cell_dep_for_script(&script);
                    if !cell_deps.contains(&dep) {
                        cell_deps.push(dep);
                    }
                }
            }
        }

        for (cell, _data) in tx.outputs_with_data_iter() {
            if let Some(script) = cell.type_().to_opt() {
                let dep = self.find_cell_dep_for_script(&script);
                if !cell_deps.contains(&dep) {
                    cell_deps.push(dep);
                }
            }
        }

        tx.as_advanced_builder()
            .set_cell_deps(Vec::new())
            .cell_deps(cell_deps.pack())
            .build()
    }

    fn build_resolved_tx(&self, tx: &TransactionView) -> ResolvedTransaction {
        let input_cells = tx
            .inputs()
            .into_iter()
            .map(|input| {
                let previous_out_point = input.previous_output();
                let (input_output, input_data) = self.cells.get(&previous_out_point).unwrap();
                let tx_info_opt = self.transaction_infos.get(&previous_out_point);
                let mut b = CellMetaBuilder::from_cell_output(
                    input_output.to_owned(),
                    input_data.to_vec().into(),
                )
                .out_point(previous_out_point);
                if let Some(tx_info) = tx_info_opt {
                    b = b.transaction_info(tx_info.to_owned());
                }
                b.build()
            })
            .collect();
        let resolved_cell_deps = tx
            .cell_deps()
            .into_iter()
            .map(|deps_out_point| {
                let (dep_output, dep_data) = self.cells.get(&deps_out_point.out_point()).unwrap();
                let tx_info_opt = self.transaction_infos.get(&deps_out_point.out_point());
                let mut b = CellMetaBuilder::from_cell_output(
                    dep_output.to_owned(),
                    dep_data.to_vec().into(),
                )
                .out_point(deps_out_point.out_point());
                if let Some(tx_info) = tx_info_opt {
                    b = b.transaction_info(tx_info.to_owned());
                }
                b.build()
            })
            .collect();
        ResolvedTransaction {
            transaction: tx.clone(),
            resolved_cell_deps,
            resolved_inputs: input_cells,
            resolved_dep_groups: vec![],
        }
    }

    // check format and consensus rules
    fn verify_tx_consensus(&self, tx: &TransactionView) -> Result<(), CKBError> {
        OutputsDataVerifier::new(tx).verify()?;
        Ok(())
    }

    pub fn capture_debug(&self) -> bool {
        self.capture_debug
    }

    /// Capture debug output, default value is false
    pub fn set_capture_debug(&mut self, capture_debug: bool) {
        self.capture_debug = capture_debug;
    }

    /// return captured messages
    pub fn captured_messages(&self) -> Vec<Message> {
        self.captured_messages.lock().unwrap().clone()
    }

    /// Verify the transaction in CKB-VM
    pub fn verify_tx(&self, tx: &TransactionView, max_cycles: u64) -> Result<Cycle, CKBError> {
        self.verify_tx_consensus(tx)?;
        let resolved_tx = self.build_resolved_tx(tx);
        let consensus = ConsensusBuilder::default()
            .hardfork_switch(HardForks {
                ckb2021: CKB2021::new_dev_default(),
                ckb2023: CKB2023::new_dev_default(),
            })
            .build();
        let tip = HeaderBuilder::default().number(0.pack()).build();
        let tx_verify_env = TxVerifyEnv::new_submit(&tip);
        let mut verifier = TransactionScriptsVerifier::new(
            Arc::new(resolved_tx),
            self.clone(),
            Arc::new(consensus),
            Arc::new(tx_verify_env),
        );
        if self.capture_debug {
            let captured_messages = self.captured_messages.clone();
            verifier.set_debug_printer(move |id, message| {
                let msg = Message {
                    id: id.clone(),
                    message: message.to_string(),
                };
                captured_messages.lock().unwrap().push(msg);
            });
        } else {
            verifier.set_debug_printer(|_id, msg| {
                println!("[contract debug] {}", msg);
            });
        }
        verifier.verify(max_cycles)
    }
}

impl CellDataProvider for Context {
    // load Cell Data
    fn load_cell_data(&self, cell: &CellMeta) -> Option<Bytes> {
        cell.mem_cell_data
            .as_ref()
            .map(|data| Bytes::from(data.to_vec()))
            .or_else(|| self.get_cell_data(&cell.out_point))
    }

    fn get_cell_data(&self, out_point: &OutPoint) -> Option<Bytes> {
        self.cells
            .get(out_point)
            .map(|(_, data)| Bytes::from(data.to_vec()))
    }

    fn get_cell_data_hash(&self, out_point: &OutPoint) -> Option<Byte32> {
        self.cells
            .get(out_point)
            .map(|(_, data)| CellOutput::calc_data_hash(data))
    }
}

impl HeaderProvider for Context {
    // load header
    fn get_header(&self, block_hash: &Byte32) -> Option<HeaderView> {
        self.headers.get(block_hash).cloned()
    }
}

impl ExtensionProvider for Context {
    fn get_block_extension(
        &self,
        hash: &ckb_types::packed::Byte32,
    ) -> Option<ckb_types::packed::Bytes> {
        self.block_extensions.get(hash).map(|b| b.pack())
    }
}
