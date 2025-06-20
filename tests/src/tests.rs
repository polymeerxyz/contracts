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
    ckb_types::{
        bytes::Bytes,
        core::{HeaderBuilder, TransactionBuilder},
        packed::*,
        prelude::*,
    },
    context::Context,
};
use common::{base::Byte32, schema::distribution::OutPoint, type_id::calculate_type_id};

#[test]
fn test_claim_distribution() {
    // deploy contracts
    let mut context = Context::default();
    let dist_lock_bin = Loader::default().load_binary("distribution-lock");
    let dist_lock_out_point = context.deploy_cell(dist_lock_bin);
    let dist_lock_dep = CellDep::new_builder()
        .out_point(dist_lock_out_point.clone())
        .build();

    let dist_type_bin = Loader::default().load_binary("distribution-type");
    let dist_type_out_point = context.deploy_cell(dist_type_bin);
    let dist_type_dep = CellDep::new_builder()
        .out_point(dist_type_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let proof_bin = Loader::default().load_binary("proof-type");
    let proof_out_point = context.deploy_cell(proof_bin.clone());
    let proof_script_dep = CellDep::new_builder()
        .out_point(proof_out_point.clone())
        .build();
    let proof_code_hash = get_code_hash(&mut context, &proof_out_point);

    // prepare scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![0]))
        .unwrap();
    let admin_lock_hash =
        Byte32::from_slice(admin_lock_script.calc_script_hash().as_slice()).unwrap();

    let subscriber_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .unwrap();
    let subscriber_lock_hash =
        Byte32::from_slice(subscriber_lock_script.calc_script_hash().as_slice()).unwrap();

    let other_subscriber_lock = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let other_subscriber_lock_hash =
        Byte32::from_slice(other_subscriber_lock.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();
    let proof_data = populate_proof_data(&subscriber_lock_hash, &campaign_id);
    let proof_type_script = context
        .build_script(&proof_out_point, Bytes::from(vec![0; 32])) // dummy type id
        .unwrap();
    let proof_cell_capacity = 254 * 100_000_000u64;
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

    // Add a cell for the subscriber to pay for fees
    let subscriber_fee_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * 100_000_000u64).pack())
            .lock(subscriber_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let subscriber_fee_input = CellInput::new_builder()
        .previous_output(subscriber_fee_input_out_point)
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
    let reward_amount = 100 * 100_000_000u64;
    let dist_capacity = reward_amount * leaves.len() as u64;
    let dist_lock_script = context
        .build_script(&dist_lock_out_point, Default::default())
        .unwrap();
    let dist_type_script = context
        .build_script(&dist_type_out_point, Default::default())
        .unwrap();
    let deadline = 1_000_000u64;
    let dist_data = data::populate_distribution_data(
        &campaign_id,
        &admin_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        &merkle_root,
        reward_amount,
        deadline,
    );
    let dist_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(dist_capacity.pack())
            .lock(dist_lock_script.clone())
            .type_(Some(dist_type_script.clone()).pack())
            .build(),
        dist_data.as_bytes(),
    );
    let dist_input = CellInput::new_builder()
        .previous_output(dist_input_out_point)
        .since(0.pack())
        .build();

    // prepare outputs
    let new_dist_capacity = dist_capacity - reward_amount;
    let dist_output = CellOutput::new_builder()
        .capacity(new_dist_capacity.pack())
        .lock(dist_lock_script)
        .type_(Some(dist_type_script).pack())
        .build();

    let reward_output = CellOutput::new_builder()
        .capacity((reward_amount + proof_cell_capacity).pack())
        .lock(subscriber_lock_script.clone())
        .build();

    let subscriber_change_output = CellOutput::new_builder()
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
        .cell_dep(dist_lock_dep)
        .cell_dep(dist_type_dep)
        .cell_dep(proof_script_dep)
        .inputs([dist_input, proof_input, subscriber_fee_input])
        .outputs([dist_output, reward_output, subscriber_change_output])
        .outputs_data([dist_data.as_bytes(), Bytes::from(""), Bytes::from("")].pack())
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
fn test_final_claim_distribution_no_dust() {
    // deploy contracts
    let mut context = Context::default();
    let dist_lock_bin = Loader::default().load_binary("distribution-lock");
    let dist_lock_out_point = context.deploy_cell(dist_lock_bin);
    let dist_lock_dep = CellDep::new_builder()
        .out_point(dist_lock_out_point.clone())
        .build();

    let dist_type_bin = Loader::default().load_binary("distribution-type");
    let dist_type_out_point = context.deploy_cell(dist_type_bin);
    let dist_type_dep = CellDep::new_builder()
        .out_point(dist_type_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let proof_bin = Loader::default().load_binary("proof-type");
    let proof_out_point = context.deploy_cell(proof_bin.clone());
    let proof_script_dep = CellDep::new_builder()
        .out_point(proof_out_point.clone())
        .build();
    let proof_code_hash = get_code_hash(&mut context, &proof_out_point);

    // prepare scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![0]))
        .unwrap();
    let admin_lock_hash =
        Byte32::from_slice(admin_lock_script.calc_script_hash().as_slice()).unwrap();

    let subscriber_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .unwrap();
    let subscriber_lock_hash =
        Byte32::from_slice(subscriber_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();
    let proof_data = populate_proof_data(&subscriber_lock_hash, &campaign_id);
    let proof_type_script = context
        .build_script(&proof_out_point, Bytes::from(vec![0; 32])) // dummy type id
        .unwrap();
    let proof_cell_capacity = 254 * 100_000_000u64;
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

    // Add a cell for the subscriber to pay for fees
    let subscriber_fee_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * 100_000_000u64).pack())
            .lock(subscriber_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let subscriber_fee_input = CellInput::new_builder()
        .previous_output(subscriber_fee_input_out_point)
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
    let reward_amount = 100 * 100_000_000u64;
    let dist_capacity = reward_amount; // No dust
    let dist_lock_script = context
        .build_script(&dist_lock_out_point, Default::default())
        .unwrap();
    let dist_type_script = context
        .build_script(&dist_type_out_point, Default::default())
        .unwrap();
    let deadline = 1_000_000u64;
    let dist_data = data::populate_distribution_data(
        &campaign_id,
        &admin_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        &merkle_root,
        reward_amount,
        deadline,
    );
    let dist_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(dist_capacity.pack())
            .lock(dist_lock_script.clone())
            .type_(Some(dist_type_script).pack())
            .build(),
        dist_data.as_bytes(),
    );
    let dist_input = CellInput::new_builder()
        .previous_output(dist_input_out_point)
        .since(0.pack())
        .build();

    // prepare outputs
    let reward_output = CellOutput::new_builder()
        .capacity((reward_amount + proof_cell_capacity).pack())
        .lock(subscriber_lock_script.clone())
        .build();

    let subscriber_change_output = CellOutput::new_builder()
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
        .cell_dep(dist_lock_dep)
        .cell_dep(dist_type_dep)
        .cell_dep(proof_script_dep)
        .inputs([dist_input, proof_input, subscriber_fee_input])
        .outputs([reward_output, subscriber_change_output])
        .outputs_data([Bytes::new(), Bytes::new()].pack())
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
fn test_reclaim_distribution() {
    // deploy contracts
    let mut context = Context::default();
    let dist_lock_bin = Loader::default().load_binary("distribution-lock");
    let dist_lock_out_point = context.deploy_cell(dist_lock_bin);
    let dist_lock_dep = CellDep::new_builder()
        .out_point(dist_lock_out_point.clone())
        .build();

    let dist_type_bin = Loader::default().load_binary("distribution-type");
    let dist_type_out_point = context.deploy_cell(dist_type_bin);
    let dist_type_dep = CellDep::new_builder()
        .out_point(dist_type_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let proof_bin = Loader::default().load_binary("proof-type");
    let proof_out_point = context.deploy_cell(proof_bin.clone());
    let proof_code_hash = get_code_hash(&mut context, &proof_out_point);

    // prepare scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![0]))
        .unwrap();
    let admin_lock_hash =
        Byte32::from_slice(admin_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();
    let merkle_root = [0u8; 32];
    let reward_amount = 100 * 100_000_000u64;
    let deadline_s = 1_000_000u64; // Deadline in seconds.

    // prepare distribution shard
    let dist_capacity = reward_amount * 10;
    let dist_lock_script = context
        .build_script(&dist_lock_out_point, Default::default())
        .unwrap();
    let dist_type_script = context
        .build_script(&dist_type_out_point, Default::default())
        .unwrap();
    let dist_data = data::populate_distribution_data(
        &campaign_id,
        &admin_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        &merkle_root,
        reward_amount,
        deadline_s,
    );
    let dist_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(dist_capacity.pack())
            .lock(dist_lock_script.clone())
            .type_(Some(dist_type_script.clone()).pack())
            .build(),
        dist_data.as_bytes(),
    );

    // Add a cell for the admin to pay for fees
    let admin_fee_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * 100_000_000u64).pack())
            .lock(admin_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let admin_fee_input = CellInput::new_builder()
        .previous_output(admin_fee_input_out_point)
        .build();

    // prepare input with `since`
    // The `since` value must be >= the deadline in the cell data.
    // The on-chain median timestamp must be >= the `since` value.
    let since_timestamp_s = deadline_s + 10; // Reclaim 10 seconds after deadline.
    let since = 0x4000_0000_0000_0000u64 | since_timestamp_s;
    let dist_input = CellInput::new_builder()
        .previous_output(dist_input_out_point)
        .since(since.pack())
        .build();

    // prepare output (reclamation to admin)
    let reclaim_output = CellOutput::new_builder()
        .capacity(dist_capacity.pack())
        .lock(admin_lock_script.clone())
        .build();

    let admin_change_output = CellOutput::new_builder().lock(admin_lock_script).build();

    // prepare header dep for `since` and script validation
    // The header timestamp must be in milliseconds and its value in seconds must be >= the since value.
    let header_timestamp_ms = since_timestamp_s * 1000;
    let header = HeaderBuilder::default()
        .timestamp(header_timestamp_ms.pack())
        .build();
    context.insert_header(header.clone());
    let header_dep = header.hash();

    // build transaction (NO witness for the dist cell group)
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(dist_lock_dep)
        .cell_dep(dist_type_dep)
        .header_dep(header_dep)
        .inputs([dist_input, admin_fee_input])
        .outputs([reclaim_output, admin_change_output])
        .outputs_data([Bytes::new().pack(), Bytes::new().pack()])
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles for distribution reclaim: {}", cycles);
}

#[test]
fn test_create_proof() {
    // deploy contracts
    let mut context = Context::default();
    let proof_bin: Bytes = Loader::default().load_binary("proof-type");
    let proof_out_point = context.deploy_cell(proof_bin);
    let proof_cell_dep = CellDep::new_builder()
        .out_point(proof_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    // prepare scripts
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
    // deploy contracts
    let mut context = Context::default();
    let vault_bin = Loader::default().load_binary("vault-type");
    let vault_out_point = context.deploy_cell(vault_bin);
    let vault_cell_dep = CellDep::new_builder()
        .out_point(vault_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let dist_lock_bin = Loader::default().load_binary("distribution-lock");
    let dist_lock_out_point = context.deploy_cell(dist_lock_bin);
    let dist_lock_code_hash = get_code_hash(&mut context, &dist_lock_out_point);

    let dist_type_bin = Loader::default().load_binary("distribution-type");
    let dist_type_out_point = context.deploy_cell(dist_type_bin);
    let dist_type_code_hash = get_code_hash(&mut context, &dist_type_out_point);

    let proof_bin = Loader::default().load_binary("proof-type");
    let proof_out_point = context.deploy_cell(proof_bin);
    let proof_code_hash = get_code_hash(&mut context, &proof_out_point);

    // prepare scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .unwrap();
    let creator_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let creator_lock_hash =
        Byte32::from_slice(creator_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare input
    let capacity = 10000 * 100_000_000u64;
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

    // prepare script args
    let mut vault_args = vec![];
    vault_args.extend_from_slice(dist_lock_code_hash.as_slice());
    vault_args.extend_from_slice(dist_type_code_hash.as_slice());
    let vault_type_script = context
        .build_script(&vault_out_point, Bytes::from(vault_args))
        .unwrap();

    // prepare ouput data
    let vault_capacity = 10000 * 100_000_000u64;
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
fn test_create_distribution() {
    // deploy contracts
    let mut context = Context::default();
    let vault_bin = Loader::default().load_binary("vault-type");
    let vault_out_point = context.deploy_cell(vault_bin);
    let vault_script_dep = CellDep::new_builder()
        .out_point(vault_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let dist_lock_bin = Loader::default().load_binary("distribution-lock");
    let dist_lock_out_point = context.deploy_cell(dist_lock_bin);
    let dist_lock_dep = CellDep::new_builder()
        .out_point(dist_lock_out_point.clone())
        .build();
    let dist_lock_code_hash = get_code_hash(&mut context, &dist_lock_out_point);

    let dist_type_bin = Loader::default().load_binary("distribution-type");
    let dist_type_out_point = context.deploy_cell(dist_type_bin);
    let dist_type_dep = CellDep::new_builder()
        .out_point(dist_type_out_point.clone())
        .build();
    let dist_type_code_hash = get_code_hash(&mut context, &dist_type_out_point);

    let proof_bin = Loader::default().load_binary("proof-type");
    let proof_out_point = context.deploy_cell(proof_bin);
    let proof_code_hash = get_code_hash(&mut context, &proof_out_point);

    // prepare scripts
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
    let vault_capacity = 10000 * 100_000_000u64;
    let fee_percentage = 500u16; // 5.00%
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();

    let vault_data = populate_vault_data(
        &campaign_id,
        &creator_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        fee_percentage,
    );

    let mut vault_args = vec![];
    vault_args.extend_from_slice(dist_lock_code_hash.as_slice());
    vault_args.extend_from_slice(dist_type_code_hash.as_slice());
    let vault_type_script = context
        .build_script(&vault_out_point, Bytes::from(vault_args))
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

    // Add a cell for the admin to pay for fees
    let admin_fee_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * 100_000_000u64).pack())
            .lock(admin_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let admin_fee_input = CellInput::new_builder()
        .previous_output(admin_fee_input_out_point)
        .build();

    // prepare outputs
    let dist_lock_script = context
        .build_script(&dist_lock_out_point, Default::default())
        .unwrap();
    let dist_type_script = context
        .build_script(&dist_type_out_point, Default::default())
        .unwrap();
    let uniform_reward_amount = 95 * 100_000_000u64;
    let merkle_root = [1u8; 32];
    let deadline = 1_000_000u64;

    // Shard 1: 50 claimants
    let shard1_capacity = uniform_reward_amount * 50;
    let shard1_data = populate_distribution_data(
        &campaign_id,
        &admin_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        &merkle_root,
        uniform_reward_amount,
        deadline,
    );
    let shard1_output = CellOutput::new_builder()
        .capacity(shard1_capacity.pack())
        .lock(dist_lock_script.clone())
        .type_(Some(dist_type_script.clone()).pack())
        .build();

    // Shard 2: 50 claimants
    let shard2_capacity = uniform_reward_amount * 50;
    let shard2_data = populate_distribution_data(
        &campaign_id,
        &admin_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        &merkle_root,
        uniform_reward_amount,
        deadline,
    );
    let shard2_output = CellOutput::new_builder()
        .capacity(shard2_capacity.pack())
        .lock(dist_lock_script)
        .type_(Some(dist_type_script).pack())
        .build();

    // Fee Cell
    let fee_capacity = vault_capacity * (fee_percentage as u64) / 10000;
    let fee_output = CellOutput::new_builder()
        .capacity(fee_capacity.pack())
        .lock(admin_lock_script.clone())
        .build();

    let admin_change_output = CellOutput::new_builder().lock(admin_lock_script).build();

    assert_eq!(
        vault_capacity,
        shard1_capacity + shard2_capacity + fee_capacity
    );

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(dist_lock_dep)
        .cell_dep(dist_type_dep)
        .cell_dep(vault_script_dep)
        .inputs([vault_input, admin_fee_input])
        .outputs([
            shard1_output,
            shard2_output,
            fee_output,
            admin_change_output,
        ])
        .outputs_data(
            [
                shard1_data.as_bytes(),
                shard2_data.as_bytes(),
                Bytes::new(),
                Bytes::new(),
            ]
            .pack(),
        )
        .witness(WitnessArgs::new_builder().build().as_bytes().pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 20_000_000)
        .expect("pass verification");
    println!("consume cycles for distribution create: {}", cycles);
}

#[test]
fn test_partial_refund_vault() {
    // deploy contracts
    let mut context = Context::default();
    let vault_bin = Loader::default().load_binary("vault-type");
    let vault_out_point = context.deploy_cell(vault_bin);
    let vault_script_dep = CellDep::new_builder()
        .out_point(vault_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let dist_lock_bin = Loader::default().load_binary("distribution-lock");
    let dist_lock_out_point = context.deploy_cell(dist_lock_bin);
    let dist_lock_code_hash = get_code_hash(&mut context, &dist_lock_out_point);

    let dist_type_bin = Loader::default().load_binary("distribution-type");
    let dist_type_out_point = context.deploy_cell(dist_type_bin);
    let dist_type_code_hash = get_code_hash(&mut context, &dist_type_out_point);

    let proof_bin = Loader::default().load_binary("proof-type");
    let proof_out_point = context.deploy_cell(proof_bin);
    let proof_code_hash = get_code_hash(&mut context, &proof_out_point);

    // prepare scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .unwrap();
    let creator_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let creator_lock_hash =
        Byte32::from_slice(creator_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let vault_capacity = 10000 * 100_000_000u64;
    let refund_capacity = 1000 * 100_000_000u64;
    let fee_percentage = 500u16; // 5.00%
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();

    let vault_data = populate_vault_data(
        &campaign_id,
        &creator_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        fee_percentage,
    );

    let mut vault_args = vec![];
    vault_args.extend_from_slice(dist_lock_code_hash.as_slice());
    vault_args.extend_from_slice(dist_type_code_hash.as_slice());
    let vault_type_script = context
        .build_script(&vault_out_point, Bytes::from(vault_args))
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

    // Add a cell for the admin to pay for fees
    let admin_fee_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * 100_000_000u64).pack())
            .lock(admin_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let admin_fee_input = CellInput::new_builder()
        .previous_output(admin_fee_input_out_point)
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
    let admin_change_output = CellOutput::new_builder().lock(admin_lock_script).build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(vault_script_dep)
        .inputs([vault_input, admin_fee_input])
        .outputs([vault_output, refund_output, admin_change_output])
        .outputs_data([vault_data.as_bytes(), Bytes::from(""), Bytes::from("")].pack())
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
    // deploy contracts
    let mut context = Context::default();
    let vault_bin = Loader::default().load_binary("vault-type");
    let vault_out_point = context.deploy_cell(vault_bin);
    let vault_script_dep = CellDep::new_builder()
        .out_point(vault_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let dist_lock_bin = Loader::default().load_binary("distribution-lock");
    let dist_lock_out_point = context.deploy_cell(dist_lock_bin);
    let dist_lock_code_hash = get_code_hash(&mut context, &dist_lock_out_point);

    let dist_type_bin = Loader::default().load_binary("distribution-type");
    let dist_type_out_point = context.deploy_cell(dist_type_bin);
    let dist_type_code_hash = get_code_hash(&mut context, &dist_type_out_point);

    let proof_bin = Loader::default().load_binary("proof-type");
    let proof_out_point = context.deploy_cell(proof_bin);
    let proof_code_hash = get_code_hash(&mut context, &proof_out_point);

    // prepare scripts
    let admin_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .unwrap();
    let creator_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let creator_lock_hash =
        Byte32::from_slice(creator_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare data
    let vault_capacity = 10000 * 100_000_000u64;
    let fee_percentage = 500u16; // 5.00%
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();

    let vault_data = populate_vault_data(
        &campaign_id,
        &creator_lock_hash,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        fee_percentage,
    );

    let mut vault_args = vec![];
    vault_args.extend_from_slice(dist_lock_code_hash.as_slice());
    vault_args.extend_from_slice(dist_type_code_hash.as_slice());
    let vault_type_script = context
        .build_script(&vault_out_point, Bytes::from(vault_args))
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

    // Add a cell for the admin to pay for fees
    let admin_fee_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * 100_000_000u64).pack())
            .lock(admin_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let admin_fee_input = CellInput::new_builder()
        .previous_output(admin_fee_input_out_point)
        .build();

    // prepare output
    let refund_output = CellOutput::new_builder()
        .capacity(vault_capacity.pack())
        .lock(creator_lock_script)
        .build();

    let admin_change_output = CellOutput::new_builder().lock(admin_lock_script).build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(vault_script_dep)
        .inputs([vault_input, admin_fee_input])
        .outputs([refund_output, admin_change_output])
        .outputs_data([Bytes::new().pack(), Bytes::new().pack()])
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles for vault full refund: {}", cycles);
}
