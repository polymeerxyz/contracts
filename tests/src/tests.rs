use crate::{
    data::{populate_proof_data, populate_verification_witness},
    Loader,
};
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_crypto::secp::Privkey,
    ckb_hash::new_blake2b,
    ckb_types::{
        bytes::Bytes,
        core::{TransactionBuilder, TransactionView},
        packed::*,
        prelude::*,
        H256,
    },
    context::Context,
};
use common::type_id::calculate_type_id;

// Include your tests here
// See https://github.com/xxuejie/ckb-native-build-sample/blob/main/tests/src/tests.rs for more examples
// generated unit test for contract proof
#[test]
fn test_proof() {
    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("proof");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_cell_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let lock_script = context
        .build_script(&always_success_out_point, Default::default())
        .expect("script");
    let lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();

    // prepare cell deps
    let cell_deps: Vec<CellDep> = vec![contract_cell_dep, lock_script_dep];

    // prepare inputs
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();

    // prepare outputs
    let type_script = context
        .build_script(&contract_out_point, Bytes::new())
        .expect("script");

    let proof_data = populate_proof_data();
    let type_args = calculate_type_id(&input, 0);

    let outputs = vec![
        // proof cell
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(
                Some(
                    type_script
                        .as_builder()
                        .args(type_args.as_slice().pack())
                        .build(),
                )
                .pack(),
            )
            .build(),
        // change cell
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script)
            .build(),
    ];

    // prepare outputs data
    let outputs_data = [proof_data.as_bytes(), Bytes::from("")];

    // build transaction
    let signed_witness_args = WitnessArgs::new_builder().build();
    let data_witness_args = WitnessArgs::new_builder()
        .input_type(Some(populate_verification_witness().as_bytes()).pack())
        .build();

    let tx = TransactionBuilder::default()
        .cell_deps(cell_deps)
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witness(signed_witness_args.as_bytes().pack())
        .witness(data_witness_args.as_bytes().pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

// generated unit test for contract claim
#[test]
fn test_claim() {
    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("claim");
    let out_point = context.deploy_cell(contract_bin);

    // prepare scripts
    let lock_script = context
        .build_script(&out_point, Bytes::from(vec![42]))
        .expect("script");

    // prepare cells
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script)
            .build(),
    ];

    let outputs_data = vec![Bytes::new(); 2];

    // build transaction
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[allow(dead_code)]
fn sign_tx(tx: TransactionView, key: &Privkey) -> TransactionView {
    const SIGNATURE_SIZE: usize = 65;

    let witnesses_len = tx.witnesses().len();
    let tx_hash = tx.hash();
    let mut blake2b = new_blake2b();
    let mut message = [0u8; 32];
    blake2b.update(&tx_hash.raw_data());

    // digest the first witness
    let witness = WitnessArgs::default();
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

    (1..witnesses_len).for_each(|n| {
        let witness = tx.witnesses().get(n).unwrap();
        let witness_len = witness.raw_data().len() as u64;
        blake2b.update(&witness_len.to_le_bytes());
        blake2b.update(&witness.raw_data());
    });
    blake2b.finalize(&mut message);

    let message = H256::from(message);
    let sig = key.sign_recoverable(&message).expect("sign");

    let mut signed_witnesses: Vec<ckb_testtool::ckb_types::packed::Bytes> = Vec::new();
    signed_witnesses.push(
        witness
            .as_builder()
            .lock(Some(Bytes::from(sig.serialize())).pack())
            .build()
            .as_bytes()
            .pack(),
    );
    for i in 1..witnesses_len {
        signed_witnesses.push(tx.witnesses().get(i).unwrap().as_bytes().pack());
    }

    tx.as_advanced_builder()
        .set_witnesses(signed_witnesses)
        .build()
}

// generated unit test for contract vault
#[test]
fn test_vault() {
    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("vault");
    let out_point = context.deploy_cell(contract_bin);

    // prepare scripts
    let lock_script = context
        .build_script(&out_point, Bytes::from(vec![42]))
        .expect("script");

    // prepare cells
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script)
            .build(),
    ];

    let outputs_data = vec![Bytes::new(); 2];

    // build transaction
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}
