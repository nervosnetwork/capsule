use std::collections::HashMap;

use ckb_script::TransactionScriptsVerifier;
use ckb_traits::{CellDataProvider, HeaderProvider};
use ckb_types::core::EpochExt;
use ckb_types::{
    bytes::Bytes,
    bytes::BytesMut,
    core::{
        cell::{CellMetaBuilder, ResolvedTransaction},
        Capacity, DepType, HeaderView, ScriptHashType, TransactionBuilder, TransactionView,
    },
    packed::{Byte32, CellDep, CellOutput, OutPoint, Script, WitnessArgsBuilder},
    prelude::*,
};

use bytes::BufMut;
use lazy_static::lazy_static;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub const MAX_CYCLES: u64 = std::u64::MAX;

lazy_static! {
    pub static ref CKB_LUA_BINARY: Bytes =
        Bytes::from(&include_bytes!("../../contracts/lua/build/debug/lua-loader")[..]);
    pub static ref LUA_SCRIPT: Bytes =
        Bytes::from(&include_bytes!("../../contracts/lua/src/{{ name }}.lua")[..]);
}

#[derive(Default)]
pub struct DummyDataLoader {
    pub cells: HashMap<OutPoint, (CellOutput, Bytes)>,
    pub headers: HashMap<Byte32, HeaderView>,
    pub epoches: HashMap<Byte32, EpochExt>,
}

impl DummyDataLoader {
    fn new() -> Self {
        Self::default()
    }
}

impl CellDataProvider for DummyDataLoader {
    fn get_cell_data(&self, out_point: &OutPoint) -> Option<ckb_types::bytes::Bytes> {
        self.cells.get(out_point).map(|(_, data)| data.clone())
    }

    fn get_cell_data_hash(&self, out_point: &OutPoint) -> Option<Byte32> {
        self.cells
            .get(out_point)
            .map(|(_, data)| CellOutput::calc_data_hash(data))
    }
}

impl HeaderProvider for DummyDataLoader {
    // load header
    fn get_header(&self, block_hash: &Byte32) -> Option<HeaderView> {
        self.headers.get(block_hash).cloned()
    }
}

fn debug_printer(script: &Byte32, msg: &str) {
    let slice = script.as_slice();
    let str = format!(
        "Script({:x}{:x}{:x}{:x}{:x})",
        slice[0], slice[1], slice[2], slice[3], slice[4]
    );
    println!("{:?}: {}", str, msg);
}

fn gen_tx(dummy: &mut DummyDataLoader) -> TransactionView {
    let mut rng = <StdRng as SeedableRng>::from_seed([42u8; 32]);

    // setup lib_ckb_lua dep
    let lib_ckb_lua_out_point = {
        let contract_tx_hash = {
            let mut buf = [0u8; 32];
            rng.fill(&mut buf);
            buf.pack()
        };
        OutPoint::new(contract_tx_hash, 0)
    };
    // dep contract code
    let lib_ckb_lua_cell = CellOutput::new_builder()
        .capacity(
            Capacity::bytes(LUA_SCRIPT.len())
                .expect("script capacity")
                .pack(),
        )
        .build();
    let lib_ckb_lua_cell_data_hash = CellOutput::calc_data_hash(&LUA_SCRIPT);
    dummy.cells.insert(
        lib_ckb_lua_out_point.clone(),
        (lib_ckb_lua_cell, LUA_SCRIPT.clone()),
    );

    // setup lua loader dep
    let lua_loader_out_point = {
        let contract_tx_hash = {
            let mut buf = [0u8; 32];
            rng.fill(&mut buf);
            buf.pack()
        };
        OutPoint::new(contract_tx_hash, 0)
    };
    // dep contract code
    let lua_loader_cell = CellOutput::new_builder()
        .capacity(
            Capacity::bytes(CKB_LUA_BINARY.len())
                .expect("script capacity")
                .pack(),
        )
        .build();
    let lua_loader_cell_data_hash = CellOutput::calc_data_hash(&CKB_LUA_BINARY);
    dummy.cells.insert(
        lua_loader_out_point.clone(),
        (lua_loader_cell, CKB_LUA_BINARY.clone()),
    );

    // setup default tx builder
    let dummy_capacity = Capacity::shannons(42);
    let mut tx_builder = TransactionBuilder::default()
        .cell_deps(vec![
            CellDep::new_builder()
                .out_point(lua_loader_out_point)
                .dep_type(DepType::Code.into())
                .build(),
            CellDep::new_builder()
                .out_point(lib_ckb_lua_out_point)
                .dep_type(DepType::Code.into())
                .build(),
        ])
        .output(
            CellOutput::new_builder()
                .capacity(dummy_capacity.pack())
                .build(),
        )
        .output_data(Bytes::new().pack());

    let previous_tx_hash = {
        let mut buf = [0u8; 32];
        rng.fill(&mut buf);
        buf.pack()
    };
    let out_point = OutPoint::new(previous_tx_hash, 0);

    let mut buf = BytesMut::with_capacity(2 + lib_ckb_lua_cell_data_hash.as_slice().len() + 1);
    buf.extend_from_slice(&[0x00u8; 2]);
    buf.extend_from_slice(lib_ckb_lua_cell_data_hash.as_slice());
    buf.put_u8(ScriptHashType::Data1.into());
    let args = buf.freeze();

    let script = Script::new_builder()
        .args(args.pack())
        .code_hash(lua_loader_cell_data_hash)
        .hash_type(ScriptHashType::Data1.into())
        .build();
    let output_cell = CellOutput::new_builder()
        .capacity(dummy_capacity.pack())
        .type_(Some(script).pack())
        .build();
    dummy
        .cells
        .insert(out_point, (output_cell.clone(), Bytes::new()));
    let mut random_extra_witness = [0u8; 32];
    rng.fill(&mut random_extra_witness);
    let witness_args = WitnessArgsBuilder::default()
        .output_type(Some(Bytes::from(random_extra_witness.to_vec())).pack())
        .build();
    tx_builder = tx_builder
        .output(output_cell)
        .witness(witness_args.as_bytes().pack());

    tx_builder.build()
}

fn build_resolved_tx(data_loader: &DummyDataLoader, tx: &TransactionView) -> ResolvedTransaction {
    let resolved_cell_deps = tx
        .cell_deps()
        .into_iter()
        .map(|deps_out_point| {
            let (dep_output, dep_data) =
                data_loader.cells.get(&deps_out_point.out_point()).unwrap();
            CellMetaBuilder::from_cell_output(dep_output.to_owned(), dep_data.to_owned())
                .out_point(deps_out_point.out_point())
                .build()
        })
        .collect();

    let mut resolved_inputs = Vec::new();
    for i in 0..tx.inputs().len() {
        let previous_out_point = tx.inputs().get(i).unwrap().previous_output();
        let (input_output, input_data) = data_loader.cells.get(&previous_out_point).unwrap();
        resolved_inputs.push(
            CellMetaBuilder::from_cell_output(input_output.to_owned(), input_data.to_owned())
                .out_point(previous_out_point)
                .build(),
        );
    }

    ResolvedTransaction {
        transaction: tx.clone(),
        resolved_cell_deps,
        resolved_inputs,
        resolved_dep_groups: vec![],
    }
}

#[test]
fn run_lua_script() {
    let mut data_loader = DummyDataLoader::new();
    let tx = gen_tx(&mut data_loader);
    let resolved_tx = build_resolved_tx(&data_loader, &tx);
    let mut verifier = TransactionScriptsVerifier::new(&resolved_tx, &data_loader);
    verifier.set_debug_printer(debug_printer);
    let verify_result = verifier.verify(MAX_CYCLES);
    verify_result.expect("pass verification");
}
