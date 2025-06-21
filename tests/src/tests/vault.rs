use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*},
    context::Context,
};
use common::base::Byte32;

use crate::{data::populate_vault_data, hash::get_code_hash, Loader};

#[test]
fn test_create_vault() {
    // deploy contracts
    let mut context = Context::default();
    let vault_type_bin = Loader::default().load_binary("vault-type");
    let vault_type_out_point = context.deploy_cell(vault_type_bin);
    let vault_type_dep = CellDep::new_builder()
        .out_point(vault_type_out_point.clone())
        .build();

    let vault_lock_bin = Loader::default().load_binary("vault-lock");
    let vault_lock_out_point = context.deploy_cell(vault_lock_bin);
    let vault_lock_dep = CellDep::new_builder()
        .out_point(vault_lock_out_point.clone())
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
    let admin_lock_hash =
        Byte32::from_slice(admin_lock_script.calc_script_hash().as_slice()).unwrap();

    let creator_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let creator_lock_hash =
        Byte32::from_slice(creator_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare input from creator
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

    // prepare vault lock script
    let mut vault_lock_args = vec![];
    vault_lock_args.extend_from_slice(creator_lock_hash.as_slice());
    vault_lock_args.extend_from_slice(admin_lock_hash.as_slice());
    let vault_lock_script = context
        .build_script(&vault_lock_out_point, Bytes::from(vault_lock_args))
        .unwrap();

    // prepare vault type script args
    let mut vault_type_args = vec![];
    vault_type_args.extend_from_slice(dist_lock_code_hash.as_slice());
    vault_type_args.extend_from_slice(dist_type_code_hash.as_slice());
    let vault_type_script = context
        .build_script(&vault_type_out_point, Bytes::from(vault_type_args))
        .unwrap();

    // prepare ouput data
    let vault_capacity = 10000 * 100_000_000u64;
    let fee_percentage = 500u16; // 5.00%
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();
    let vault_data = populate_vault_data(
        &campaign_id,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        fee_percentage,
    );

    //prepare output
    let vault_output = CellOutput::new_builder()
        .lock(vault_lock_script)
        .type_(Some(vault_type_script).pack())
        .capacity(vault_capacity.pack())
        .build();
    let change_output = CellOutput::new_builder().lock(creator_lock_script).build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(vault_type_dep)
        .cell_dep(vault_lock_dep)
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
fn test_partial_refund_vault() {
    // deploy contracts
    let mut context = Context::default();
    let vault_type_bin = Loader::default().load_binary("vault-type");
    let vault_type_out_point = context.deploy_cell(vault_type_bin);
    let vault_type_dep = CellDep::new_builder()
        .out_point(vault_type_out_point.clone())
        .build();

    let vault_lock_bin = Loader::default().load_binary("vault-lock");
    let vault_lock_out_point = context.deploy_cell(vault_lock_bin);
    let vault_lock_dep = CellDep::new_builder()
        .out_point(vault_lock_out_point.clone())
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
    let admin_lock_hash =
        Byte32::from_slice(admin_lock_script.calc_script_hash().as_slice()).unwrap();

    let creator_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let creator_lock_hash =
        Byte32::from_slice(creator_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare vault lock script
    let mut vault_lock_args = vec![];
    vault_lock_args.extend_from_slice(creator_lock_hash.as_slice());
    vault_lock_args.extend_from_slice(admin_lock_hash.as_slice());
    let vault_lock_script = context
        .build_script(&vault_lock_out_point, Bytes::from(vault_lock_args))
        .unwrap();

    // prepare vault type script
    let mut vault_type_args = vec![];
    vault_type_args.extend_from_slice(dist_lock_code_hash.as_slice());
    vault_type_args.extend_from_slice(dist_type_code_hash.as_slice());
    let vault_type_script = context
        .build_script(&vault_type_out_point, Bytes::from(vault_type_args))
        .unwrap();

    // prepare data
    let vault_capacity = 10000 * 100_000_000u64;
    let refund_capacity = 1000 * 100_000_000u64;
    let fee_percentage = 500u16; // 5.00%
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();

    let vault_data = populate_vault_data(
        &campaign_id,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        fee_percentage,
    );

    let vault_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(vault_capacity.pack())
            .lock(vault_lock_script.clone())
            .type_(Some(vault_type_script.clone()).pack())
            .build(),
        vault_data.clone().as_bytes(),
    );
    let vault_input = CellInput::new_builder()
        .previous_output(vault_input_out_point)
        .build();

    // Add a cell for the creator to pay for fees and provide signature
    let creator_fee_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * 100_000_000u64).pack())
            .lock(creator_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let creator_fee_input = CellInput::new_builder()
        .previous_output(creator_fee_input_out_point)
        .build();

    // prepare output
    let vault_output = CellOutput::new_builder()
        .capacity((vault_capacity - refund_capacity).pack())
        .lock(vault_lock_script.clone())
        .type_(Some(vault_type_script).pack())
        .build();
    let refund_output = CellOutput::new_builder()
        .capacity(refund_capacity.pack())
        .lock(creator_lock_script.clone())
        .build();
    let creator_change_output = CellOutput::new_builder().lock(creator_lock_script).build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(vault_type_dep)
        .cell_dep(vault_lock_dep)
        .inputs([vault_input, creator_fee_input])
        .outputs([vault_output, refund_output, creator_change_output])
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
    let vault_type_bin = Loader::default().load_binary("vault-type");
    let vault_type_out_point = context.deploy_cell(vault_type_bin);
    let vault_type_dep = CellDep::new_builder()
        .out_point(vault_type_out_point.clone())
        .build();

    let vault_lock_bin = Loader::default().load_binary("vault-lock");
    let vault_lock_out_point = context.deploy_cell(vault_lock_bin);
    let vault_lock_dep = CellDep::new_builder()
        .out_point(vault_lock_out_point.clone())
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
    let admin_lock_hash =
        Byte32::from_slice(admin_lock_script.calc_script_hash().as_slice()).unwrap();

    let creator_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .unwrap();
    let creator_lock_hash =
        Byte32::from_slice(creator_lock_script.calc_script_hash().as_slice()).unwrap();

    // prepare vault lock script
    let mut vault_lock_args = vec![];
    vault_lock_args.extend_from_slice(creator_lock_hash.as_slice());
    vault_lock_args.extend_from_slice(admin_lock_hash.as_slice());
    let vault_lock_script = context
        .build_script(&vault_lock_out_point, Bytes::from(vault_lock_args))
        .unwrap();

    // prepare vault type script
    let mut vault_type_args = vec![];
    vault_type_args.extend_from_slice(dist_lock_code_hash.as_slice());
    vault_type_args.extend_from_slice(dist_type_code_hash.as_slice());
    let vault_type_script = context
        .build_script(&vault_type_out_point, Bytes::from(vault_type_args))
        .unwrap();

    // prepare data
    let vault_capacity = 10000 * 100_000_000u64;
    let fee_percentage = 500u16; // 5.00%
    let campaign_id = Byte32::from_slice(&[1; 32]).unwrap();

    let vault_data = populate_vault_data(
        &campaign_id,
        &Byte32::from_slice(proof_code_hash.as_slice()).unwrap(),
        fee_percentage,
    );

    let vault_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(vault_capacity.pack())
            .lock(vault_lock_script.clone())
            .type_(Some(vault_type_script.clone()).pack())
            .build(),
        vault_data.clone().as_bytes(),
    );
    let vault_input = CellInput::new_builder()
        .previous_output(vault_input_out_point)
        .build();

    // Add a cell for the creator to pay for fees and provide signature
    let creator_fee_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * 100_000_000u64).pack())
            .lock(creator_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let creator_fee_input = CellInput::new_builder()
        .previous_output(creator_fee_input_out_point)
        .build();

    // prepare output
    let refund_output = CellOutput::new_builder()
        .capacity(vault_capacity.pack())
        .lock(creator_lock_script.clone())
        .build();

    let creator_change_output = CellOutput::new_builder().lock(creator_lock_script).build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(always_success_dep)
        .cell_dep(vault_type_dep)
        .cell_dep(vault_lock_dep)
        .inputs([vault_input, creator_fee_input])
        .outputs([refund_output, creator_change_output])
        .outputs_data([Bytes::new().pack(), Bytes::new().pack()])
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles for vault full refund: {}", cycles);
}
