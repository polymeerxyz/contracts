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

    let lock_script_hash = Byte32::from_slice(lock_script.calc_script_hash().as_slice()).unwrap();
    let campaign_id = Byte32::from_slice(&[2; 32]).unwrap();
    let proof_data = populate_proof_data(&lock_script_hash, &campaign_id);
    let sample = hex_string(&proof_data.as_bytes());
    println!("proof_data: {}", sample);
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
    println!("outputs_data: {:?}", outputs_data);

    // build transaction
    let witness_args = WitnessArgs::new_builder().build();

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

// generated unit test for contract vault
#[test]
fn test_vault() {
    // 1. Setup: deploy contracts, prepare scripts
    // let mut context = Context::default();
    // let vault_bin = Loader::default().load_binary("vault");
    // let vault_out_point = context.deploy_cell(vault_bin);
    // let vault_script_dep = CellDep::new_builder()
    //     .out_point(vault_out_point.clone())
    //     .build();

    // let dist_bin = Loader::default().load_binary("distribution");
    // let dist_out_point = context.deploy_cell(dist_bin.clone());
    // let dist_script_dep = CellDep::new_builder()
    //     .out_point(dist_out_point.clone())
    //     .build();
    // let dist_code_hash = CellOutput::calc_data_hash(&dist_bin);

    // let proof_bin = Loader::default().load_binary("proof");
    // let proof_code_hash = CellOutput::calc_data_hash(&proof_bin);

    // let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    // let always_success_dep = CellDep::new_builder()
    //     .out_point(always_success_out_point.clone())
    //     .build();

    // let admin_lock_script = context
    //     .build_script(&always_success_out_point, Bytes::from(vec![1]))
    //     .unwrap();
    // let creator_lock_script = context
    //     .build_script(&always_success_out_point, Bytes::from(vec![2]))
    //     .unwrap();
    // let creator_lock_hash = creator_lock_script.calc_script_hash();

    // // 2. Prepare input: Vault Cell
    // let vault_capacity = 10000 * 100_000_000u64; // 10000 CKB
    // let fee_percentage = 500u16; // 5.00%
    // let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();

    // let vault_data = populate_vault_data(
    //     &campaign_id,
    //     &creator_lock_hash,
    //     &proof_code_hash,
    //     fee_percentage,
    // );

    // let vault_type_script = context
    //     .build_script(&vault_out_point, dist_code_hash.as_bytes())
    //     .unwrap();

    // let vault_input_out_point = context.create_cell(
    //     CellOutput::new_builder()
    //         .capacity(vault_capacity.pack())
    //         .lock(admin_lock_script.clone())
    //         .type_(Some(vault_type_script).pack())
    //         .build(),
    //     vault_data.as_bytes(),
    // );
    // let vault_input = CellInput::new_builder()
    //     .previous_output(vault_input_out_point)
    //     .build();

    // // 3. Prepare outputs: Distribution Shards and Fee Cell
    // let dist_lock_script = context
    //     .build_script(&dist_out_point, Default::default())
    //     .unwrap();
    // let uniform_reward_amount = 95 * 100_000_000u64; // 95 CKB
    // let merkle_root = [0u8; 32];

    // // Shard 1: 50 claimants
    // let shard1_capacity = uniform_reward_amount * 50;
    // let shard1_data = data::populate_distribution_data(
    //     &campaign_id,
    //     &proof_code_hash,
    //     &merkle_root,
    //     uniform_reward_amount,
    //     0, // shard_id
    // );
    // let shard1_output = CellOutput::new_builder()
    //     .capacity(shard1_capacity.pack())
    //     .lock(dist_lock_script.clone())
    //     .build();

    // // Shard 2: 50 claimants
    // let shard2_capacity = uniform_reward_amount * 50;
    // let shard2_data = data::populate_distribution_data(
    //     &campaign_id,
    //     &proof_code_hash,
    //     &merkle_root,
    //     uniform_reward_amount,
    //     1, // shard_id
    // );
    // let shard2_output = CellOutput::new_builder()
    //     .capacity(shard2_capacity.pack())
    //     .lock(dist_lock_script)
    //     .build();

    // // Fee Cell
    // let fee_capacity = vault_capacity * (fee_percentage as u64) / 10000;
    // let fee_output = CellOutput::new_builder()
    //     .capacity(fee_capacity.pack())
    //     .lock(admin_lock_script)
    //     .build();

    // assert_eq!(
    //     vault_capacity,
    //     shard1_capacity + shard2_capacity + fee_capacity
    // );

    // let outputs = vec![shard1_output, shard2_output, fee_output];
    // let outputs_data = vec![shard1_data.as_bytes(), shard2_data.as_bytes(), Bytes::new()];

    // // 4. Build transaction
    // let tx = TransactionBuilder::default()
    //     .input(vault_input)
    //     .outputs(outputs)
    //     .outputs_data(outputs_data.pack())
    //     .cell_dep(vault_script_dep)
    //     .cell_dep(dist_script_dep)
    //     .cell_dep(always_success_dep)
    //     .witness(WitnessArgs::new_builder().build().as_bytes().pack())
    //     .build();

    // let tx = context.complete_tx(tx);

    // // 5. Run verification
    // let cycles = context
    //     .verify_tx(&tx, 20_000_000)
    //     .expect("pass verification");
    // println!("consume cycles for vault distribution: {}", cycles);
}

#[test]
fn test_proof_data() {
    let proof_data_raw = "9400000014000000340000005400000074000000412d494f4b3049664d504c502d61754856315963390000000000000000000000422d494f4b3049664d504c502d61754856315963390000000000000000000000432d494f4b3049664d504c502d617548563159633900000000000000000000004472b33b4e1845ebe82f2ce5f511bbe012f144c5f3d7b539909adffc83ccda61";
    let proof_data = decode_hex::<ProofCellData>(proof_data_raw).unwrap();

    println!("proof_data: {:?}", proof_data.entity_id());
    println!("proof_data: {:?}", proof_data.campaign_id());
    println!("proof_data: {:?}", proof_data.proof());
    println!("proof_data: {:?}", proof_data.subscriber_lock_hash());
}
