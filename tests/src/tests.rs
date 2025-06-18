use crate::{
    data::{
        self, populate_claim_witness, populate_distribution_data, populate_proof_data,
        populate_vault_data,
    },
    hash::get_code_hash,
    util, Loader,
};
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*},
    context::Context,
};
use common::{
    base::Byte32,
    schema::{distribution::OutPoint, proof::ProofCellData},
    type_id::calculate_type_id,
    utils::decode_hex,
};

// Include your tests here
// See https://github.com/xxuejie/ckb-native-build-sample/blob/main/tests/src/tests.rs for more examples

#[test]
fn test_claim_distribution() {
    // deploy contracts, prepare scripts
    let mut context = Context::default();
    let dist_bin = Loader::default().load_binary("distribution");
    let dist_out_point = context.deploy_cell(dist_bin);
    let dist_script_dep = CellDep::new_builder()
        .out_point(dist_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let proof_bin = Loader::default().load_binary("proof");
    let proof_out_point = context.deploy_cell(proof_bin.clone());
    let proof_script_dep = CellDep::new_builder()
        .out_point(proof_out_point.clone())
        .build();
    let proof_code_hash = get_code_hash(&mut context, &proof_out_point);

    // lock scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![0]))
        .unwrap();
    let admin_lock_hash =
        Byte32::from_slice(admin_lock_script.calc_script_hash().as_slice()).unwrap();

    let subscriber_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1])) // claimant 1
        .unwrap();
    let subscriber_lock_hash =
        Byte32::from_slice(subscriber_lock_script.calc_script_hash().as_slice()).unwrap();

    let other_subscriber_lock = context
        .build_script(&always_success_out_point, Bytes::from(vec![2])) // claimant 2
        .unwrap();
    let other_subscriber_lock_hash =
        Byte32::from_slice(other_subscriber_lock.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();
    let proof_data = populate_proof_data(&subscriber_lock_hash, &campaign_id);
    let proof_type_script = context
        .build_script(&proof_out_point, Bytes::from(vec![0; 32])) // dummy type id
        .unwrap();
    let proof_cell_capacity = 254 * 100_000_000u64; // 254 CKB
    let proof_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(proof_cell_capacity.pack())
            .lock(subscriber_lock_script.clone())
            .type_(Some(proof_type_script).pack())
            .build(),
        proof_data.as_bytes(),
    );
    let proof_input = CellInput::new_builder()
        .previous_output(proof_input_out_point.clone())
        .build();

    // prepare Merkle Tree
    let mut leaf_data_1 = vec![];
    leaf_data_1.extend_from_slice(proof_input_out_point.as_slice());
    leaf_data_1.extend_from_slice(subscriber_lock_hash.as_slice());
    let leaf0 = util::blake2b_256(leaf_data_1);

    let other_proof_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(other_subscriber_lock)
            .build(),
        Bytes::new(),
    );
    let mut leaf_data_2 = vec![];
    leaf_data_2.extend_from_slice(other_proof_out_point.as_slice());
    leaf_data_2.extend_from_slice(other_subscriber_lock_hash.as_slice());
    let leaf1 = util::blake2b_256(leaf_data_2);

    let leaves = vec![leaf0, leaf1];
    let merkle_root = util::build_merkle_root(&leaves);
    let merkle_proof = util::build_merkle_proof(&leaves, 0);

    // prepare distribution shard
    let reward_amount = 100 * 100_000_000u64; // 100 CKB
    let dist_capacity = reward_amount * leaves.len() as u64;
    let dist_lock_script = context
        .build_script(&dist_out_point, Default::default())
        .unwrap();
    let dist_data = data::populate_distribution_data(
        &campaign_id,
        &admin_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        &merkle_root,
        reward_amount,
    );
    let dist_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(dist_capacity.pack())
            .lock(dist_lock_script.clone())
            .build(),
        dist_data.as_bytes(),
    );
    let dist_input = CellInput::new_builder()
        .previous_output(dist_input_out_point)
        .build();

    // prepare outputs
    let new_dist_capacity = dist_capacity - reward_amount;
    let dist_output = CellOutput::new_builder()
        .capacity(new_dist_capacity.pack())
        .lock(dist_lock_script)
        .build();

    let reward_output = CellOutput::new_builder()
        .capacity(reward_amount.pack())
        .lock(subscriber_lock_script)
        .build();

    // prepare witness
    let proof_cell_out_point_for_witness =
        OutPoint::from_slice(proof_input_out_point.as_slice()).unwrap();
    let claim_witness = populate_claim_witness(
        &proof_cell_out_point_for_witness,
        &subscriber_lock_hash,
        &merkle_proof,
    );

    let witness_for_dist = WitnessArgs::new_builder()
        .lock(Some(claim_witness.as_bytes()).pack())
        .build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(dist_script_dep)
        .cell_dep(proof_script_dep)
        .inputs([dist_input, proof_input])
        .outputs([dist_output, reward_output])
        .outputs_data([dist_data.as_bytes(), Bytes::from("")].pack())
        .witness(witness_for_dist.as_bytes().pack())
        .build();

    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles for distribution claim: {}", cycles);
}

#[test]
fn test_final_claim_distribution_with_dust() {
    // deploy contracts, prepare scripts
    let mut context = Context::default();
    let dist_bin = Loader::default().load_binary("distribution");
    let dist_out_point = context.deploy_cell(dist_bin);
    let dist_script_dep = CellDep::new_builder()
        .out_point(dist_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let proof_bin = Loader::default().load_binary("proof");
    let proof_out_point = context.deploy_cell(proof_bin.clone());
    let proof_script_dep = CellDep::new_builder()
        .out_point(proof_out_point.clone())
        .build();
    let proof_code_hash = get_code_hash(&mut context, &proof_out_point);

    // lock scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![0]))
        .unwrap();
    let admin_lock_hash =
        Byte32::from_slice(admin_lock_script.calc_script_hash().as_slice()).unwrap();

    let subscriber_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1])) // claimant 1
        .unwrap();
    let subscriber_lock_hash =
        Byte32::from_slice(subscriber_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();
    let proof_data = populate_proof_data(&subscriber_lock_hash, &campaign_id);
    let proof_type_script = context
        .build_script(&proof_out_point, Bytes::from(vec![0; 32])) // dummy type id
        .unwrap();
    let proof_cell_capacity = 254 * 100_000_000u64; // 254 CKB
    let proof_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(proof_cell_capacity.pack())
            .lock(subscriber_lock_script.clone())
            .type_(Some(proof_type_script).pack())
            .build(),
        proof_data.as_bytes(),
    );
    let proof_input = CellInput::new_builder()
        .previous_output(proof_input_out_point.clone())
        .build();

    // prepare Merkle Tree (only one leaf for final claim)
    let mut leaf_data = vec![];
    leaf_data.extend_from_slice(proof_input_out_point.as_slice());
    leaf_data.extend_from_slice(subscriber_lock_hash.as_slice());
    let leaf0 = util::blake2b_256(leaf_data);

    let leaves = vec![leaf0];
    let merkle_root = util::build_merkle_root(&leaves);
    let merkle_proof = util::build_merkle_proof(&leaves, 0);

    // prepare distribution shard
    let reward_amount = 100 * 100_000_000u64; // 100 CKB
    let dust_amount = 546 * 100_000_000u64; // 546 CKB (min cell capacity)
    let dist_capacity = reward_amount + dust_amount;
    let dist_lock_script = context
        .build_script(&dist_out_point, Default::default())
        .unwrap();
    let dist_data = data::populate_distribution_data(
        &campaign_id,
        &admin_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        &merkle_root,
        reward_amount,
    );
    let dist_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(dist_capacity.pack())
            .lock(dist_lock_script.clone())
            .build(),
        dist_data.as_bytes(),
    );
    let dist_input = CellInput::new_builder()
        .previous_output(dist_input_out_point)
        .build();

    // prepare outputs
    let reward_output = CellOutput::new_builder()
        .capacity(reward_amount.pack())
        .lock(subscriber_lock_script)
        .build();

    let admin_refund_output = CellOutput::new_builder()
        .capacity(dust_amount.pack())
        .lock(admin_lock_script)
        .build();
    println!(
        "lock_script_hash {:?}",
        admin_refund_output.calc_lock_hash().as_slice()
    );

    // prepare witness
    let proof_cell_out_point_for_witness =
        OutPoint::from_slice(proof_input_out_point.as_slice()).unwrap();
    let claim_witness = populate_claim_witness(
        &proof_cell_out_point_for_witness,
        &subscriber_lock_hash,
        &merkle_proof,
    );

    let witness_for_dist = WitnessArgs::new_builder()
        .lock(Some(claim_witness.as_bytes()).pack())
        .build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(dist_script_dep)
        .cell_dep(proof_script_dep)
        .inputs([dist_input, proof_input])
        .outputs([reward_output, admin_refund_output])
        .outputs_data([Bytes::new(), Bytes::new()].pack())
        .witness(witness_for_dist.as_bytes().pack())
        .build();

    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!(
        "consume cycles for distribution final claim with dust: {}",
        cycles
    );
}

#[test]
fn test_final_claim_distribution_no_dust() {
    // deploy contracts, prepare scripts
    let mut context = Context::default();
    let dist_bin = Loader::default().load_binary("distribution");
    let dist_out_point = context.deploy_cell(dist_bin);
    let dist_script_dep = CellDep::new_builder()
        .out_point(dist_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let proof_bin = Loader::default().load_binary("proof");
    let proof_out_point = context.deploy_cell(proof_bin.clone());
    let proof_script_dep = CellDep::new_builder()
        .out_point(proof_out_point.clone())
        .build();
    let proof_code_hash = get_code_hash(&mut context, &proof_out_point);

    // lock scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![0]))
        .unwrap();
    let admin_lock_hash =
        Byte32::from_slice(admin_lock_script.calc_script_hash().as_slice()).unwrap();

    let subscriber_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1])) // claimant 1
        .unwrap();
    let subscriber_lock_hash =
        Byte32::from_slice(subscriber_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();
    let proof_data = populate_proof_data(&subscriber_lock_hash, &campaign_id);
    let proof_type_script = context
        .build_script(&proof_out_point, Bytes::from(vec![0; 32])) // dummy type id
        .unwrap();
    let proof_cell_capacity = 254 * 100_000_000u64; // 254 CKB
    let proof_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(proof_cell_capacity.pack())
            .lock(subscriber_lock_script.clone())
            .type_(Some(proof_type_script).pack())
            .build(),
        proof_data.as_bytes(),
    );
    let proof_input = CellInput::new_builder()
        .previous_output(proof_input_out_point.clone())
        .build();

    // prepare Merkle Tree (only one leaf for final claim)
    let mut leaf_data = vec![];
    leaf_data.extend_from_slice(proof_input_out_point.as_slice());
    leaf_data.extend_from_slice(subscriber_lock_hash.as_slice());
    let leaf0 = util::blake2b_256(leaf_data);

    let leaves = vec![leaf0];
    let merkle_root = util::build_merkle_root(&leaves);
    let merkle_proof = util::build_merkle_proof(&leaves, 0);

    // prepare distribution shard
    let reward_amount = 100 * 100_000_000u64; // 100 CKB
    let dist_capacity = reward_amount; // No dust
    let dist_lock_script = context
        .build_script(&dist_out_point, Default::default())
        .unwrap();
    let dist_data = data::populate_distribution_data(
        &campaign_id,
        &admin_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        &merkle_root,
        reward_amount,
    );
    let dist_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(dist_capacity.pack())
            .lock(dist_lock_script.clone())
            .build(),
        dist_data.as_bytes(),
    );
    let dist_input = CellInput::new_builder()
        .previous_output(dist_input_out_point)
        .build();

    // prepare outputs
    let reward_output = CellOutput::new_builder()
        .capacity(reward_amount.pack())
        .lock(subscriber_lock_script)
        .build();

    // prepare witness
    let proof_cell_out_point_for_witness =
        OutPoint::from_slice(proof_input_out_point.as_slice()).unwrap();
    let claim_witness = populate_claim_witness(
        &proof_cell_out_point_for_witness,
        &subscriber_lock_hash,
        &merkle_proof,
    );

    let witness_for_dist = WitnessArgs::new_builder()
        .lock(Some(claim_witness.as_bytes()).pack())
        .build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(dist_script_dep)
        .cell_dep(proof_script_dep)
        .inputs([dist_input, proof_input])
        .outputs([reward_output])
        .outputs_data([Bytes::new()].pack())
        .witness(witness_for_dist.as_bytes().pack())
        .build();

    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!(
        "consume cycles for distribution final claim no dust: {}",
        cycles
    );
}

#[test]
fn test_create_proof() {
    // deploy contracts, prepare scripts
    let mut context = Context::default();
    let proof_bin: Bytes = Loader::default().load_binary("proof");
    let proof_out_point = context.deploy_cell(proof_bin);
    let proof_cell_dep = CellDep::new_builder()
        .out_point(proof_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    // lock scripts
    let subscriber_lock_script = context
        .build_script(&always_success_out_point, Default::default())
        .unwrap();
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
            &proof_out_point,
            Bytes::copy_from_slice(&calculate_type_id(&input, 0)),
        )
        .unwrap();

    // prepare outputs data
    let campaign_id = Byte32::from_slice(&[2; 32]).unwrap();
    let proof_data = populate_proof_data(&subscriber_lock_hash, &campaign_id);

    // prepare output
    let proof_output = CellOutput::new_builder()
        .lock(subscriber_lock_script.clone())
        .type_(Some(proof_type_script).pack())
        .build();
    let change_output = CellOutput::new_builder()
        .lock(subscriber_lock_script.clone())
        .build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(proof_cell_dep)
        .cell_dep(always_success_dep)
        .input(input)
        .outputs([proof_output, change_output])
        .outputs_data([proof_data.as_bytes(), Bytes::from("")].pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles for proof create: {}", cycles);
}

#[test]
fn test_create_vault() {
    // deploy contracts, prepare scripts
    let mut context = Context::default();
    let vault_bin = Loader::default().load_binary("vault");
    let vault_out_point = context.deploy_cell(vault_bin);
    let vault_cell_dep = CellDep::new_builder()
        .out_point(vault_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let dist_bin = Loader::default().load_binary("distribution");
    let dist_code_hash =
        Byte32::from_slice(CellOutput::calc_data_hash(&dist_bin).as_slice()).unwrap();

    let proof_bin = Loader::default().load_binary("proof");
    let proof_code_hash =
        Byte32::from_slice(CellOutput::calc_data_hash(&proof_bin).as_slice()).unwrap();

    // lock scripts
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

    //prepare output
    let vault_output = CellOutput::new_builder()
        .lock(admin_lock_script)
        .type_(Some(vault_type_script).pack())
        .capacity(vault_capacity.pack())
        .build();
    let change_output = CellOutput::new_builder().lock(creator_lock_script).build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(vault_cell_dep)
        .input(input)
        .outputs([vault_output, change_output])
        .outputs_data([vault_data.as_bytes(), Bytes::from("")].pack())
        .witness(WitnessArgs::new_builder().build().as_bytes().pack())
        .build();

    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles for vault create: {}", cycles);
}

#[test]
fn test_spend_vault() {
    // deploy contracts, prepare scripts
    let mut context = Context::default();
    let vault_bin = Loader::default().load_binary("vault");
    let vault_out_point = context.deploy_cell(vault_bin);
    let vault_script_dep = CellDep::new_builder()
        .out_point(vault_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let dist_bin = Loader::default().load_binary("distribution");
    let dist_out_point = context.deploy_cell(dist_bin.clone());
    let dist_script_dep = CellDep::new_builder()
        .out_point(dist_out_point.clone())
        .build();
    let dist_code_hash = get_code_hash(&mut context, &dist_out_point);

    let proof_bin = Loader::default().load_binary("proof");
    let proof_code_hash =
        Byte32::from_slice(CellOutput::calc_data_hash(&proof_bin).as_slice()).unwrap();

    // code hash

    // lock scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .unwrap();
    let admin_lock_hash =
        Byte32::from_slice(admin_lock_script.calc_script_hash().as_slice()).unwrap();

    let creator_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let creator_lock_hash =
        Byte32::from_slice(creator_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let vault_capacity = 10000 * 100_000_000u64; // 10000 CKB
    let fee_percentage = 500u16; // 5.00%
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();

    let vault_data = populate_vault_data(
        &campaign_id,
        &creator_lock_hash,
        &proof_code_hash,
        fee_percentage,
    );

    let vault_type_script = context
        .build_script(&vault_out_point, dist_code_hash.as_bytes())
        .unwrap();

    let vault_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(vault_capacity.pack())
            .lock(admin_lock_script.clone())
            .type_(Some(vault_type_script).pack())
            .build(),
        vault_data.as_bytes(),
    );
    let vault_input = CellInput::new_builder()
        .previous_output(vault_input_out_point)
        .build();

    // prepare outputs
    let dist_lock_script = context
        .build_script(&dist_out_point, Default::default())
        .unwrap();
    let uniform_reward_amount = 95 * 100_000_000u64; // 95 CKB
    let merkle_root = [0u8; 32];

    // Shard 1: 50 claimants
    let shard1_capacity = uniform_reward_amount * 50;
    let shard1_data = populate_distribution_data(
        &campaign_id,
        &admin_lock_hash,
        &proof_code_hash,
        &merkle_root,
        uniform_reward_amount,
    );
    let shard1_output = CellOutput::new_builder()
        .capacity(shard1_capacity.pack())
        .lock(dist_lock_script.clone())
        .build();

    // Shard 2: 50 claimants
    let shard2_capacity = uniform_reward_amount * 50;
    let shard2_data = populate_distribution_data(
        &campaign_id,
        &admin_lock_hash,
        &proof_code_hash,
        &merkle_root,
        uniform_reward_amount,
    );
    let shard2_output = CellOutput::new_builder()
        .capacity(shard2_capacity.pack())
        .lock(dist_lock_script)
        .build();

    // Fee Cell
    let fee_capacity = vault_capacity * (fee_percentage as u64) / 10000;
    let fee_output = CellOutput::new_builder()
        .capacity(fee_capacity.pack())
        .lock(admin_lock_script)
        .build();

    assert_eq!(
        vault_capacity,
        shard1_capacity + shard2_capacity + fee_capacity
    );

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(dist_script_dep)
        .cell_dep(vault_script_dep)
        .input(vault_input)
        .outputs([shard1_output, shard2_output, fee_output])
        .outputs_data([shard1_data.as_bytes(), shard2_data.as_bytes(), Bytes::new()].pack())
        .witness(WitnessArgs::new_builder().build().as_bytes().pack())
        .build();

    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 20_000_000)
        .expect("pass verification");
    println!("consume cycles for vault spend: {}", cycles);
}

#[test]
fn test_partial_refund_vault() {
    // deploy contracts, prepare scripts
    let mut context = Context::default();
    let vault_bin = Loader::default().load_binary("vault");
    let vault_out_point = context.deploy_cell(vault_bin);
    let vault_script_dep = CellDep::new_builder()
        .out_point(vault_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let dist_bin = Loader::default().load_binary("distribution");
    let dist_code_hash =
        Byte32::from_slice(CellOutput::calc_data_hash(&dist_bin).as_slice()).unwrap();

    let proof_bin = Loader::default().load_binary("proof");
    let proof_code_hash =
        Byte32::from_slice(CellOutput::calc_data_hash(&proof_bin).as_slice()).unwrap();

    // lock scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .unwrap();
    let creator_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let creator_lock_hash =
        Byte32::from_slice(creator_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let vault_capacity = 10000 * 100_000_000u64; // 10000 CKB
    let refund_capacity = 1000 * 100_000_000u64; // 10000 CKB
    let fee_percentage = 500u16; // 5.00%
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();

    let vault_data = populate_vault_data(
        &campaign_id,
        &creator_lock_hash,
        &proof_code_hash,
        fee_percentage,
    );

    let vault_type_script = context
        .build_script(&vault_out_point, dist_code_hash.as_bytes())
        .unwrap();

    let vault_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(vault_capacity.pack())
            .lock(admin_lock_script.clone())
            .type_(Some(vault_type_script.clone()).pack())
            .build(),
        vault_data.clone().as_bytes(),
    );
    let vault_input = CellInput::new_builder()
        .previous_output(vault_input_out_point)
        .build();

    // prepare output
    let vault_output = CellOutput::new_builder()
        .capacity((vault_capacity - refund_capacity).pack())
        .lock(admin_lock_script.clone())
        .type_(Some(vault_type_script).pack())
        .build();
    let refund_output = CellOutput::new_builder()
        .capacity(refund_capacity.pack())
        .lock(creator_lock_script)
        .build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(vault_script_dep)
        .input(vault_input)
        .outputs([vault_output, refund_output])
        .outputs_data([vault_data.as_bytes(), Bytes::from("")].pack())
        .build();

    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles for vault partial refund: {}", cycles);
}

#[test]
fn test_full_refund_vault() {
    // deploy contracts, prepare scripts
    let mut context = Context::default();
    let vault_bin = Loader::default().load_binary("vault");
    let vault_out_point = context.deploy_cell(vault_bin);
    let vault_script_dep = CellDep::new_builder()
        .out_point(vault_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let dist_bin = Loader::default().load_binary("distribution");
    let dist_code_hash =
        Byte32::from_slice(CellOutput::calc_data_hash(&dist_bin).as_slice()).unwrap();

    let proof_bin = Loader::default().load_binary("proof");
    let proof_code_hash =
        Byte32::from_slice(CellOutput::calc_data_hash(&proof_bin).as_slice()).unwrap();

    // lock scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .unwrap();
    let creator_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let creator_lock_hash =
        Byte32::from_slice(creator_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let vault_capacity = 10000 * 100_000_000u64; // 10000 CKB
    let refund_capacity = 10000 * 100_000_000u64; // 10000 CKB
    let fee_percentage = 500u16; // 5.00%
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();

    let vault_data = populate_vault_data(
        &campaign_id,
        &creator_lock_hash,
        &proof_code_hash,
        fee_percentage,
    );

    let vault_type_script = context
        .build_script(&vault_out_point, dist_code_hash.as_bytes())
        .unwrap();

    let vault_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(vault_capacity.pack())
            .lock(admin_lock_script.clone())
            .type_(Some(vault_type_script.clone()).pack())
            .build(),
        vault_data.clone().as_bytes(),
    );
    let vault_input = CellInput::new_builder()
        .previous_output(vault_input_out_point)
        .build();

    // prepare output
    let refund_output = CellOutput::new_builder()
        .capacity(refund_capacity.pack())
        .lock(creator_lock_script)
        .build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(vault_script_dep)
        .input(vault_input)
        .output(refund_output)
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles for vault full refund: {}", cycles);
}

#[test]
fn test_proof_data() {
    let proof_data_raw = "78a65fe089399eed9edcc4363d52e7b81ae64b806bfc215bef4cac02c8c3182fd733d666c2454834fe42de9e585d83011d63cf578dab5e451b62e32a889feeeaacbdf8e03f6547bb67a4ebfd485c34cda7ea6b940a48d25ba349e7e27ef5e8f74472b33b4e1845ebe82f2ce5f511bbe012f144c5f3d7b539909adffc83ccda61";
    let proof_data = decode_hex::<ProofCellData>(proof_data_raw);
    assert!(proof_data.is_ok());
}
