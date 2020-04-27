use ckb_tool::ckb_hash::new_blake2b;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::TransactionView,
    packed::{self, WitnessArgs},
    prelude::*,
    H256,
};
use anyhow::{Result, anyhow};
use std::process::Output;

pub const SIGNATURE_SIZE: usize = 65;

pub fn tx_sign_message(tx: &TransactionView, begin_index: usize, len: usize) -> H256 {
    let mut blake2b = new_blake2b();
    let mut message = [0u8; 32];
    blake2b.update(&tx.hash().raw_data());
    // digest the first witness
    let witness = WitnessArgs::new_unchecked(tx.witnesses().get(begin_index).unwrap().unpack());
    let zero_lock: Bytes = {
        let mut buf = Vec::new();
        buf.resize(SIGNATURE_SIZE, 0);
        buf.into()
    };
    let witness_for_digest = witness
        .clone()
        .as_builder()
        .lock(Some(zero_lock).pack())
        .build();
    let witness_len = witness_for_digest.as_bytes().len() as u64;
    blake2b.update(&witness_len.to_le_bytes());
    blake2b.update(&witness_for_digest.as_bytes());
    ((begin_index + 1)..(begin_index + len)).for_each(|n| {
        let witness = tx.witnesses().get(n).unwrap();
        let witness_len = witness.raw_data().len() as u64;
        blake2b.update(&witness_len.to_le_bytes());
        blake2b.update(&witness.raw_data());
    });
    blake2b.finalize(&mut message);
    let message = H256::from(message);
    message
}

pub fn attach_signature(
    tx: TransactionView,
    signature: Bytes,
    begin_index: usize,
) -> TransactionView {
    assert_eq!(signature.len(), SIGNATURE_SIZE);
    let mut signed_witnesses: Vec<packed::Bytes> = tx
        .inputs()
        .into_iter()
        .enumerate()
        .map(|(i, _)| {
            if i == begin_index {
                let witness =
                    WitnessArgs::new_unchecked(tx.witnesses().get(begin_index).unwrap().unpack());
                witness
                    .as_builder()
                    .lock(Some(signature.clone()).pack())
                    .build()
                    .as_bytes()
                    .pack()
            } else {
                tx.witnesses().get(i).unwrap_or_default()
            }
        })
        .collect();
    for i in signed_witnesses.len()..tx.witnesses().len() {
        signed_witnesses.push(tx.witnesses().get(i).unwrap());
    }
    // calculate message
    tx.as_advanced_builder()
        .set_witnesses(signed_witnesses)
        .build()
}

pub fn handle_cmd(output: Output) -> Result<Vec<u8>> {
    if output.status.success() {
        Ok(output.stdout)
    } else {
        eprintln!("run command err: {:?}", output.status.code());
        eprintln!(
            "error output: \n{}",
            String::from_utf8(output.stderr).unwrap_or_default()
        );
        Err(anyhow!("exit code: {}", output.status.code().unwrap_or(-1)))
    }
}
