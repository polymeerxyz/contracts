use crate::{
    data::{self, populate_claim_witness, populate_proof_data, populate_vault_data},
    util, Loader,
};
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*},
    context::Context,
};
use common::{
    base::Byte32, schema::proof::ProofCellData, type_id::calculate_type_id, utils::decode_hex,
};

// Include your tests here
// See https://github.com/xxuejie/ckb-native-build-sample/blob/main/tests/src/tests.rs for more examples

#[test]
fn test_distribution() {
    // 1. Setup: deploy contracts, prepare scripts
    // let mut context = Context::default();
    // let dist_bin = Loader::default().load_binary("distribution");
    // let dist_out_point = context.deploy_cell(dist_bin);
    // let dist_script_dep = CellDep::new_builder()
    //     .out_point(dist_out_point.clone())
    //     .build();

    // let proof_bin = Loader::default().load_binary("proof");
    // let proof_out_point = context.deploy_cell(proof_bin.clone());
    // let proof_script_dep = CellDep::new_builder()
    //     .out_point(proof_out_point.clone())
    //     .build();
    // let proof_code_hash = CellOutput::calc_data_hash(&proof_bin);

    // let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    // let subscriber_lock_script = context
    //     .build_script(&always_success_out_point, Bytes::from(vec![1])) // claimant 1
    //     .expect("script");
    // let subscriber_lock_hash = subscriber_lock_script.calc_script_hash();
    // let always_success_dep = CellDep::new_builder()
    //     .out_point(always_success_out_point.clone())
    //     .build();

    // // 2. Prepare Merkle Tree for two claimants
    // let proof_input_out_point = context.create_cell(
    //     CellOutput::new_builder()
    //         .capacity(1000u64.pack())
    //         .lock(subscriber_lock_script.clone())
    //         .build(),
    //     Bytes::new(),
    // );
    // let mut leaf_data = vec![];
    // leaf_data.extend_from_slice(proof_input_out_point.as_slice());
    // leaf_data.extend_from_slice(subscriber_lock_hash.as_slice());
    // let leaf0 = util::blake2b_256(leaf_data);

    // let other_subscriber_lock = context
    //     .build_script(&always_success_out_point, Bytes::from(vec![2])) // claimant 2
    //     .expect("script");
    // let other_subscriber_lock_hash = other_subscriber_lock.calc_script_hash();
    // let other_out_point = context.create_cell(
    //     CellOutput::new_builder()
    //         .capacity(1000u64.pack())
    //         .lock(other_subscriber_lock)
    //         .build(),
    //     Bytes::new(),
    // );
    // let mut other_leaf_data = vec![];
    // other_leaf_data.extend_from_slice(other_out_point.as_slice());
    // other_leaf_data.extend_from_slice(other_subscriber_lock_hash.as_slice());
    // let leaf1 = util::blake2b_256(other_leaf_data);

    // let leaves = vec![leaf0, leaf1];
    // let merkle_root = util::build_merkle_root(&leaves);
    // let merkle_proof = util::build_merkle_proof(&leaves, 0);

    // // 3. Prepare inputs
    // // Input 0: Distribution Shard Cell
    // let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();
    // let reward_amount = 100_000_000u64; // 1 CKB
    // let dist_capacity = reward_amount * leaves.len() as u64;

    // let dist_lock_script = context
    //     .build_script(&dist_out_point, Default::default())
    //     .unwrap();
    // let dist_data = data::populate_distribution_data(
    //     &campaign_id,
    //     &proof_code_hash,
    //     &merkle_root,
    //     reward_amount,
    //     0,
    // );
    // let dist_input_out_point = context.create_cell(
    //     CellOutput::new_builder()
    //         .capacity(dist_capacity.pack())
    //         .lock(dist_lock_script.clone())
    //         .build(),
    //     dist_data.as_bytes(),
    // );
    // let dist_input = CellInput::new_builder()
    //     .previous_output(dist_input_out_point)
    //     .build();

    // // Input 1: Proof Cell
    // let proof_type_script = context
    //     .build_script(&proof_out_point, Bytes::from(vec![0; 32])) // dummy type id
    //     .unwrap();
    // let proof_data = data::populate_proof_data(&subscriber_lock_hash, &campaign_id);
    // // We reuse the outpoint from merkle leaf calculation as the proof cell's previous_output
    // let proof_input = CellInput::new_builder()
    //     .previous_output(proof_input_out_point.clone())
    //     .build();
    // context.create_cell_with_outpoint(
    //     proof_input_out_point.clone(),
    //     CellOutput::new_builder()
    //         .capacity(500_000_000_000u64.pack()) // large capacity
    //         .lock(subscriber_lock_script.clone())
    //         .type_(Some(proof_type_script).pack())
    //         .build(),
    //     proof_data.as_bytes(),
    // );

    // // 4. Prepare outputs
    // // Output 0: New Distribution Shard Cell
    // let new_dist_capacity = dist_capacity - reward_amount;
    // let new_dist_output = CellOutput::new_builder()
    //     .capacity(new_dist_capacity.pack())
    //     .lock(dist_lock_script)
    //     .build();

    // // Output 1: Reward Cell
    // let reward_output = CellOutput::new_builder()
    //     .capacity(reward_amount.pack())
    //     .lock(subscriber_lock_script)
    //     .build();

    // let outputs = vec![new_dist_output, reward_output];
    // let outputs_data = vec![dist_data.as_bytes(), Bytes::new()];

    // // 5. Prepare witness
    // let claim_witness =
    //     populate_claim_witness(&proof_input_out_point, &subscriber_lock_hash, &merkle_proof);
    // let witness_args = WitnessArgs::new_builder()
    //     .lock(Some(claim_witness.as_bytes()).pack())
    //     .build();

    // // 6. Build transaction
    // let tx = TransactionBuilder::default()
    //     .inputs(vec![dist_input, proof_input])
    //     .outputs(outputs)
    //     .outputs_data(outputs_data.pack())
    //     .cell_dep(dist_script_dep)
    //     .cell_dep(proof_script_dep)
    //     .cell_dep(always_success_dep)
    //     .witness(witness_args.as_bytes().pack())
    //     .build();

    // let tx = context.complete_tx(tx);

    // // 7. Run verification
    // let cycles = context
    //     .verify_tx(&tx, 20_000_000)
    //     .expect("pass verification");
    // println!("consume cycles for distribution: {}", cycles);
}

#[test]
fn test_create_proof() {
    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("proof");
    let contract_out_point = context.deploy_cell(contract_bin);

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let subscriber_lock_script = context
        .build_script(&always_success_out_point, Default::default())
        .expect("script");
    let subscriber_lock_hash =
        Byte32::from_slice(subscriber_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare inputs
    let capacity = 1000 * 100_000_000u64;
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(subscriber_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();

    // prepare script
    let proof_type_script = context
        .build_script(
            &contract_out_point,
            Bytes::copy_from_slice(&calculate_type_id(&input, 0)),
        )
        .expect("script");

    // prepare outputs data
    let campaign_id = Byte32::from_slice(&[2; 32]).unwrap();
    let proof_data = populate_proof_data(&subscriber_lock_hash, &campaign_id);

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(
            CellDep::new_builder()
                .out_point(contract_out_point.clone())
                .build(),
        )
        .cell_dep(
            CellDep::new_builder()
                .out_point(always_success_out_point)
                .build(),
        )
        .input(input)
        .output(
            CellOutput::new_builder()
                .lock(subscriber_lock_script.clone())
                .type_(Some(proof_type_script).pack())
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .lock(subscriber_lock_script.clone())
                .build(),
        )
        .output_data(proof_data.as_bytes().pack())
        .output_data(Bytes::from("").pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

// generated unit test for contract vault
#[test]
fn test_create_vault() {
    // deploy contracts, prepare scripts
    let mut context = Context::default();
    let vault_bin = Loader::default().load_binary("vault");
    let vault_out_point = context.deploy_cell(vault_bin);
    let vault_cell_dep = CellDep::new_builder()
        .out_point(vault_out_point.clone())
        .build();

    let dist_bin = Loader::default().load_binary("distribution");
    let dist_code_hash =
        Byte32::from_slice(CellOutput::calc_data_hash(&dist_bin).as_slice()).unwrap();

    let proof_bin = Loader::default().load_binary("proof");
    let proof_code_hash =
        Byte32::from_slice(CellOutput::calc_data_hash(&proof_bin).as_slice()).unwrap();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .unwrap();

    let creator_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let creator_lock_hash =
        Byte32::from_slice(creator_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare input
    let capacity = 10000 * 100_000_000u64; // 10000 CKB
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(creator_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();

    // prepare script
    let vault_type_script = context
        .build_script(&vault_out_point, dist_code_hash.as_bytes())
        .unwrap();

    // prepare ouput data
    let vault_capacity = 10000 * 100_000_000u64; // 10000 CKB
    let fee_percentage = 500u16; // 5.00%
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();
    let vault_data = populate_vault_data(
        &campaign_id,
        &creator_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        fee_percentage,
    );

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(vault_cell_dep)
        .cell_dep(always_success_dep)
        .input(input)
        .output(
            CellOutput::new_builder()
                .lock(admin_lock_script)
                .type_(Some(vault_type_script).pack())
                .capacity(vault_capacity.pack())
                .build(),
        )
        .output(CellOutput::new_builder().lock(creator_lock_script).build())
        .output_data(vault_data.as_bytes().pack())
        .output_data(Bytes::from("").pack())
        .witness(WitnessArgs::new_builder().build().as_bytes().pack())
        .build();

    let tx = context.complete_tx(tx);

    // 5. Run verification
    let cycles = context
        .verify_tx(&tx, 20_000_000)
        .expect("pass verification");
    println!("consume cycles for vault distribution: {}", cycles);
}

#[test]
fn test_proof_data() {
    let proof_data_raw = "78a65fe089399eed9edcc4363d52e7b81ae64b806bfc215bef4cac02c8c3182fd733d666c2454834fe42de9e585d83011d63cf578dab5e451b62e32a889feeeaacbdf8e03f6547bb67a4ebfd485c34cda7ea6b940a48d25ba349e7e27ef5e8f74472b33b4e1845ebe82f2ce5f511bbe012f144c5f3d7b539909adffc83ccda61";
    let proof_data = decode_hex::<ProofCellData>(proof_data_raw);
    assert!(proof_data.is_ok());
}
