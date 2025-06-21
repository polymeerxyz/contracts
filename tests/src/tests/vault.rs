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

#[test]
fn test_vault_lock() {
    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("vault-lock");
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
