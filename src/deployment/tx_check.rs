use crate::wallet::Wallet;
use anyhow::{anyhow, Result};
use ckb_tool::ckb_chain_spec::consensus::TYPE_ID_CODE_HASH;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{BlockView, ScriptHashType, TransactionView},
    packed,
    prelude::*,
};
use std::collections::HashSet;
use std::convert::TryFrom;

fn find_output(
    genesis: &BlockView,
    out_point: &packed::OutPoint,
) -> Option<(packed::CellOutput, packed::Byte32)> {
    genesis
        .transactions()
        .into_iter()
        .find(|tx| tx.hash() == out_point.tx_hash())
        .map(|tx| {
            let index: u32 = out_point.index().unpack();
            let output = tx.outputs().get(index as usize).expect("output");
            let data: Bytes = tx
                .outputs_data()
                .get(index as usize)
                .expect("data")
                .to_owned()
                .unpack();
            let data_hash = packed::CellOutput::calc_data_hash(&data);
            (output.clone(), data_hash)
        })
}

pub fn tx_check(wallet: &Wallet, tx: &TransactionView) -> Result<()> {
    let mut dep_data_hashes: HashSet<packed::Byte32> = HashSet::new();
    let mut dep_type_hashes: HashSet<packed::Byte32> = HashSet::new();

    // insert type_id
    dep_type_hashes.insert(TYPE_ID_CODE_HASH.pack());

    // we won't generate dep_group in capsule, so here cell deps is enough
    for dep in tx.cell_deps() {
        match find_output(wallet.genesis(), &dep.out_point()) {
            Some((cell_output, data_hash)) => {
                dep_data_hashes.insert(data_hash);
                if let Some(type_) = cell_output.type_().to_opt() {
                    dep_type_hashes.insert(type_.calc_script_hash());
                }
            }
            None => {
                return Err(anyhow!("cant't find dep cell {} in the genesis", dep));
            }
        }
    }

    //TODO check inputs's lock scripts & type scripts

    //TODO check outputs's type scripts
    for (i, output) in tx.outputs().into_iter().enumerate() {
        if let Some(type_) = output.type_().to_opt() {
            let code_hash = type_.code_hash();
            match ScriptHashType::try_from(type_.hash_type()).expect("hash type") {
                ScriptHashType::Data if !dep_data_hashes.contains(&code_hash) => {
                    return Err(anyhow!(
                        "can't find data hash: {} in the dep; source: Output({})#type_",
                        code_hash,
                        i
                    ));
                }
                ScriptHashType::Type if !dep_type_hashes.contains(&code_hash) => {
                    return Err(anyhow!(
                        "can't find type hash: {} in the dep; source: Output({})#type_",
                        code_hash,
                        i
                    ));
                }
                _ => {}
            }
        }
    }
    Ok(())
}
