#![cfg_attr(not(any(feature = "library", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "library", test))]
extern crate alloc;

#[cfg(not(any(feature = "library", test)))]
ckb_std::entry!(program_entry);
#[cfg(not(any(feature = "library", test)))]
// By default, the following heap configuration is used:
// * 16KB fixed heap
// * 1.2MB(rounded up to be 16-byte aligned) dynamic heap
// * Minimal memory block in dynamic heap is 64 bytes
// For more details, please refer to ckb-std's default_alloc macro
// and the buddy-alloc alloc implementation.
ckb_std::default_alloc!(16384, 1258306, 64);

use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    debug,
    high_level::{
        load_cell, load_cell_data, load_cell_lock_hash, load_input_out_point, load_script,
        load_witness_args, QueryIter,
    },
};
use common::schema::{
    distribution::{ClaimWitness, DistributionCellData},
    proof::ProofCellData,
};
use distribution_type::error::{BizError, Error};

pub fn program_entry() -> i8 {
    match entry() {
        Ok(()) => 0,
        Err(err) => err.into(),
    }
}

fn entry() -> Result<(), Error> {
    debug!("distribution type contract is executing");

    let group_inputs_count = QueryIter::new(load_cell, Source::GroupInput).count();
    if group_inputs_count != 1 {
        // This script doesn't handle creation, only updates/consumption.
        // We could add creation logic, but for now, we'll assume it's created correctly.
        Err(BizError::ClaimTransactionInvalid)?
    }

    let dist_data_bytes = load_cell_data(0, Source::GroupInput)?;
    let dist_data = DistributionCellData::from_slice(&dist_data_bytes)
        .map_err(|_| BizError::DistributionDataInvalid)?;

    match load_witness_args(0, Source::GroupInput) {
        Ok(witness_args) => {
            let witness_args_bytes = witness_args
                .lock()
                .to_opt()
                .ok_or(BizError::WitnessDataInvalid)?
                .raw_data();

            let claim_witness = ClaimWitness::from_slice(&witness_args_bytes)
                .map_err(|_| BizError::WitnessDataInvalid)?;

            let proof_capacity = verify_and_get_proof_cell(&dist_data, &claim_witness)?;
            verify_claim_outputs(&dist_data, &claim_witness, proof_capacity)?;
        }
        Err(_) => {
            verify_reclamation_outputs(&dist_data)?;
        }
    }
    Ok(())
}

fn verify_and_get_proof_cell(
    dist_data: &DistributionCellData,
    claim_witness: &ClaimWitness,
) -> Result<u64, Error> {
    let expected_proof_code_hash = dist_data.proof_script_code_hash();

    let proof_indices = QueryIter::new(load_cell, Source::Input)
        .enumerate()
        .filter_map(|(idx, cell)| {
            let code_hash_opt = cell.type_().to_opt().map(|s| s.code_hash().as_bytes());
            if Some(expected_proof_code_hash.as_bytes()) == code_hash_opt {
                let capacity: u64 = cell.capacity().unpack();
                Some((idx, capacity, cell.calc_lock_hash()))
            } else {
                None
            }
        })
        .collect::<alloc::vec::Vec<_>>();

    if proof_indices.len() != 1 {
        Err(BizError::ProofCellCountInvalid)?;
    }

    let (index, proof_capacity, proof_lock_hash) = proof_indices[0].clone();

    let actual_proof_outpoint = load_input_out_point(index, Source::Input)?;
    if actual_proof_outpoint.as_bytes() != claim_witness.proof_cell_out_point().as_bytes() {
        Err(BizError::ProofOutPointMismatch)?;
    }

    let proof_cell_data_bytes = load_cell_data(index, Source::Input)?;
    let proof_data =
        ProofCellData::from_slice(&proof_cell_data_bytes).map_err(|_| BizError::ProofDataInvalid)?;

    if proof_data.campaign_id().as_bytes() != dist_data.campaign_id().as_bytes() {
        Err(BizError::CampaignIdMismatch)?;
    }

    if proof_data.subscriber_lock_hash().as_bytes()
        != claim_witness.subscriber_lock_hash().as_bytes()
    {
        Err(BizError::SubscriberLockHashMismatch)?;
    }

    if proof_lock_hash.as_bytes() != claim_witness.subscriber_lock_hash().as_bytes() {
        Err(BizError::ProofLockHashMismatch)?;
    }

    Ok(proof_capacity)
}

fn verify_claim_outputs(
    dist_data: &DistributionCellData,
    claim_witness: &ClaimWitness,
    proof_capacity: u64,
) -> Result<(), Error> {
    let input_dist_cell = load_cell(0, Source::GroupInput)?;
    let input_capacity: u64 = input_dist_cell.capacity().unpack();
    let reward_amount = dist_data.uniform_reward_amount().unpack();
    let expected_reward_capacity = reward_amount + proof_capacity;

    let is_final_claim = input_capacity == reward_amount;
    let expected_output_count = if is_final_claim { 1 } else { 2 };

    if QueryIter::new(load_cell, Source::Output).count() != expected_output_count {
        Err(BizError::ClaimTransactionInvalid)?;
    }
    let expected_reward_lock_hash = claim_witness.subscriber_lock_hash();

    let mut reward_cell_found = false;
    let mut new_dist_cell_found = false;

    for i in 0..expected_output_count {
        let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;

        if output_lock_hash == expected_reward_lock_hash.as_slice() {
            if reward_cell_found {
                Err(BizError::ClaimTransactionInvalid)?;
            }

            let reward_cell = load_cell(i, Source::Output)?;
            let reward_capacity: u64 = reward_cell.capacity().unpack();
            if reward_capacity != expected_reward_capacity {
                Err(BizError::RewardAmountInvalid)?;
            }
            reward_cell_found = true;
        } else {
            if is_final_claim {
                Err(BizError::FinalClaimInvalid)?;
            }

            if new_dist_cell_found {
                Err(BizError::ClaimTransactionInvalid)?;
            }

            let script_hash = load_script()?.calc_script_hash();
            let output_cell = load_cell(i, Source::Output)?;
            let output_type_hash = output_cell.type_().to_opt().map(|s| s.calc_script_hash());
            if output_type_hash != Some(script_hash) {
                Err(BizError::TypeScriptUpdateForbidden)?;
            }

            if output_cell.lock().as_bytes() != input_dist_cell.lock().as_bytes() {
                Err(BizError::ClaimTransactionInvalid)?;
            }

            if load_cell_data(i, Source::Output)? != load_cell_data(0, Source::GroupInput)? {
                Err(BizError::ShardDataUpdateImmutable)?;
            }

            let output_capacity: u64 = output_cell.capacity().unpack();
            if output_capacity != input_capacity - reward_amount {
                Err(BizError::ShardCapacityInvalid)?;
            }
            new_dist_cell_found = true;
        }
    }

    if !reward_cell_found {
        Err(BizError::RewardCellMissing)?;
    }

    if !is_final_claim && !new_dist_cell_found {
        Err(BizError::ClaimTransactionInvalid)?;
    }

    Ok(())
}

fn verify_reclamation_outputs(dist_data: &DistributionCellData) -> Result<(), Error> {
    if QueryIter::new(load_cell, Source::Output).count() != 1 {
        Err(BizError::DistributionTransactionInvalid)?;
    }

    let output_lock_hash = load_cell_lock_hash(0, Source::Output)?;
    if output_lock_hash != dist_data.admin_lock_hash().as_slice() {
        Err(BizError::AdminRefundCellMissing)?;
    }

    let input_capacity: u64 = load_cell(0, Source::GroupInput)?.capacity().unpack();
    let output_capacity: u64 = load_cell(0, Source::Output)?.capacity().unpack();
    if input_capacity != output_capacity {
        Err(BizError::AdminRefundAmountInvalid)?;
    }

    Ok(())
}
