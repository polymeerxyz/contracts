use crate::Loader;
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*},
    context::Context,
};
use common::generated::data::ProofData;
use faster_hex::hex_decode;

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

    let type_hash = get_type_hash();
    let outputs = vec![
        // proof cell
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(
                Some(
                    type_script
                        .as_builder()
                        .args(type_hash.as_slice().pack())
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
    let proof_data = get_proof_data();
    let outputs_data = [proof_data.as_bytes(), Bytes::from("")];

    // build transaction
    let witness_args = get_witness_args();

    let tx = TransactionBuilder::default()
        .cell_deps(cell_deps)
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witness(witness_args.as_bytes().pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

fn get_type_hash() -> [u8; 32] {
    let raw_type_hash = b"3c9dd0a5f89092516f51c362a8bcab7971005669de203e09c2db8fa044c861d6";
    let mut type_hash_dst = [0u8; 32];
    let _ = hex_decode(raw_type_hash.as_slice(), &mut type_hash_dst);
    type_hash_dst
}

fn get_proof_data() -> ProofData {
    let raw_proof_data = b"432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a0000000000000000000000e0fec9c1999e9bcd5615deab922872f39116f35432c2417ccbf2de49f95d849a";
    let mut proof_data_dst = [0u8; 96];
    let _ = hex_decode(raw_proof_data.as_slice(), &mut proof_data_dst);
    let proof_data = ProofData::from_slice(&proof_data_dst);
    proof_data.unwrap()
}

fn get_witness_args() -> WitnessArgs {
    let raw_witness = b"e80100001000000010000000e8010000d4010000d40100000800000006000000432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d350a58000000000432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d35fe7c000000001432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d36f2a0000000002432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d37e6c4000000003432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d38dae8000000004432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d39cf0c000000005";
    let mut witness_dst: [u8; 488] = [0u8; 488];
    let _ = hex_decode(raw_witness.as_slice(), &mut witness_dst);
    let witness = WitnessArgs::from_slice(&witness_dst);
    witness.unwrap()
}
