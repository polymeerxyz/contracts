use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*},
    context::Context,
};
use common::base::Byte32;

use crate::{data::populate_proof_data, util::calculate_type_id, Loader};

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
