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
        load_cell, load_cell_data, load_cell_lock_hash, load_input_out_point, load_input_since,
        load_script, load_witness_args, QueryIter,
    },
};
use common::{
    schema::{
        distribution::{ClaimWitness, DistributionCellData},
        proof::ProofCellData,
    },
    NULL_HASH,
};
use distribution_type::error::{BizError, Error};

const SINCE_TIMESTAMP_FLAG: u64 = 0x4000_0000_0000_0000;

pub fn program_entry() -> i8 {
    match entry() {
        Ok(()) => 0,
        Err(err) => err.into(),
    }
}

fn entry() -> Result<(), Error> {
    debug!("distribution type contract is executing");

    let inputs_count = QueryIter::new(load_cell, Source::GroupInput).count();
    let outputs_count = QueryIter::new(load_cell, Source::GroupOutput).count();

    match (inputs_count, outputs_count) {
        (0, count) if count > 0 => {
            // Case 1: Creation. 0 inputs with this type, N > 0 outputs.
            verify_creation(count)
        }
        (1, 1) => {
            // Case 2: Update. 1 input, 1 output. This must be a normal claim.
            let since = load_input_since(0, Source::GroupInput)?;
            if since != 0 {
                Err(BizError::ClaimTransactionInvalid)?;
            }

            let dist_data_bytes = load_cell_data(0, Source::GroupInput)?;
            let dist_data = DistributionCellData::from_slice(&dist_data_bytes)
                .map_err(|_| BizError::ShardCreationDataInvalid)?;

            let witness_args = load_witness_args(0, Source::GroupInput)?;
            let witness_args_bytes = witness_args
                .lock()
                .to_opt()
                .ok_or(BizError::WitnessDataInvalid)?
                .raw_data();

            let claim_witness = ClaimWitness::from_slice(&witness_args_bytes)
                .map_err(|_| BizError::WitnessDataInvalid)?;

            verify_claim_update(&dist_data, &claim_witness)
        }
        (1, 0) => {
            // Case 3: Destruction. 1 input, 0 outputs. Final claim or reclamation.
            let since = load_input_since(0, Source::GroupInput)?;

            let dist_data_bytes = load_cell_data(0, Source::GroupInput)?;
            let dist_data = DistributionCellData::from_slice(&dist_data_bytes)
                .map_err(|_| BizError::ShardCreationDataInvalid)?;

            verify_destruction(&dist_data, since)
        }
        _ => Err(BizError::DistributionTransactionInvalid)?,
    }
}

fn verify_creation(outputs_count: usize) -> Result<(), Error> {
    let first_shard_data_bytes = load_cell_data(0, Source::GroupOutput)?;
    let first_shard_data = DistributionCellData::from_slice(&first_shard_data_bytes)
        .map_err(|_| BizError::ShardCreationDataInvalid)?;

    let campaign_id = first_shard_data.campaign_id();
    if campaign_id.as_slice() == NULL_HASH {
        Err(BizError::ShardCreationDataInvalid)?;
    }

    let proof_script_code_hash = first_shard_data.proof_script_code_hash();
    if proof_script_code_hash.as_slice() == NULL_HASH {
        Err(BizError::ShardCreationDataInvalid)?;
    }

    let admin_lock_hash = first_shard_data.admin_lock_hash();
    if admin_lock_hash.as_slice() == NULL_HASH {
        Err(BizError::ShardCreationDataInvalid)?;
    }

    let uniform_reward_amount = first_shard_data.uniform_reward_amount();
    let uniform_reward_amount_unpacked: u64 = uniform_reward_amount.unpack();
    if uniform_reward_amount_unpacked == 0 {
        Err(BizError::ShardCreationDataInvalid)?;
    }

    let deadline = first_shard_data.deadline();
    let deadline_unpacked: u64 = deadline.unpack();
    if deadline_unpacked == 0 {
        Err(BizError::ShardCreationDataInvalid)?;
    }

    let merkle_root = first_shard_data.merkle_root();
    if merkle_root.as_slice() == NULL_HASH {
        Err(BizError::ShardCreationDataInvalid)?;
    }

    if outputs_count > 1 {
        for i in 1..outputs_count {
            let current_shard_data_bytes = load_cell_data(i, Source::GroupOutput)?;
            let current_shard_data = DistributionCellData::from_slice(&current_shard_data_bytes)
                .map_err(|_| BizError::ShardCreationDataInvalid)?;

            if current_shard_data.campaign_id().as_bytes() != campaign_id.as_bytes() {
                Err(BizError::ShardCreationDataInconsistent)?;
            }
            if current_shard_data.proof_script_code_hash().as_bytes()
                != proof_script_code_hash.as_bytes()
            {
                Err(BizError::ShardCreationDataInconsistent)?;
            }
            if current_shard_data.admin_lock_hash().as_bytes() != admin_lock_hash.as_bytes() {
                Err(BizError::ShardCreationDataInconsistent)?;
            }
            if current_shard_data.uniform_reward_amount().as_bytes()
                != uniform_reward_amount.as_bytes()
            {
                Err(BizError::ShardCreationDataInconsistent)?;
            }
            if current_shard_data.deadline().as_bytes() != deadline.as_bytes() {
                Err(BizError::ShardCreationDataInconsistent)?;
            }
            if current_shard_data.merkle_root().as_slice() == NULL_HASH {
                Err(BizError::ShardCreationDataInconsistent)?;
            }
        }
    }

    Ok(())
}

fn verify_and_get_proof_cell_capacity(
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
    let proof_data = ProofCellData::from_slice(&proof_cell_data_bytes)
        .map_err(|_| BizError::ProofDataInvalid)?;

    if proof_data.campaign_id().as_bytes() != dist_data.campaign_id().as_bytes() {
        Err(BizError::ProofCampaignIdMismatch)?;
    }

    if proof_data.subscriber_lock_hash().as_bytes()
        != claim_witness.subscriber_lock_hash().as_bytes()
    {
        Err(BizError::ProofSubscriberLockHashMismatch)?;
    }

    if proof_lock_hash.as_bytes() != claim_witness.subscriber_lock_hash().as_bytes() {
        Err(BizError::ProofLockHashMismatch)?;
    }

    Ok(proof_capacity)
}

fn verify_claim_update(
    dist_data: &DistributionCellData,
    claim_witness: &ClaimWitness,
) -> Result<(), Error> {
    let proof_cell_capacity = verify_and_get_proof_cell_capacity(dist_data, claim_witness)?;

    let input_dist_cell = load_cell(0, Source::GroupInput)?;
    let input_capacity: u64 = input_dist_cell.capacity().unpack();
    let reward_amount_unpacked: u64 = dist_data.uniform_reward_amount().unpack();
    let expected_reward_capacity = reward_amount_unpacked + proof_cell_capacity;

    let expected_reward_lock_hash: [u8; 32] = claim_witness.subscriber_lock_hash().into();
    let script_hash = load_script()?.calc_script_hash();

    let mut reward_cell_found = false;
    let mut new_shard_cell_found = false;

    // Iterate over all outputs to find the required reward cell and new shard cell.
    // This allows for other outputs, such as a change cell for the claimant.
    for i in 0..QueryIter::new(load_cell, Source::Output).count() {
        let output_cell = load_cell(i, Source::Output)?;
        let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;
        let output_capacity: u64 = output_cell.capacity().unpack();

        if output_lock_hash == expected_reward_lock_hash
            && output_capacity == expected_reward_capacity
            && output_cell.type_().to_opt().is_none()
        {
            // This is the claimant's reward cell.
            if reward_cell_found {
                // Cannot have more than one reward cell.
                Err(BizError::ClaimTransactionInvalid)?;
            }
            reward_cell_found = true;
        } else if let Some(type_script) = output_cell.type_().to_opt() {
            if type_script.calc_script_hash() == script_hash {
                // This is the updated distribution shard cell.
                if new_shard_cell_found {
                    Err(BizError::ClaimTransactionInvalid)?;
                }
                if output_cell.lock() != input_dist_cell.lock() {
                    Err(BizError::ShardTypeScriptImmutable)?;
                }
                if load_cell_data(i, Source::Output)? != load_cell_data(0, Source::GroupInput)? {
                    Err(BizError::ShardDataUpdateImmutable)?;
                }
                if output_capacity != input_capacity - reward_amount_unpacked {
                    Err(BizError::ShardCapacityUpdateInvalid)?;
                }
                new_shard_cell_found = true;
            }
        }
        // Any other cell is considered a change cell and is ignored.
    }

    if !reward_cell_found || !new_shard_cell_found {
        Err(BizError::ClaimTransactionInvalid)?;
    }

    Ok(())
}

fn verify_destruction(dist_data: &DistributionCellData, since: u64) -> Result<(), Error> {
    match load_witness_args(0, Source::GroupInput) {
        Ok(witness_args) => {
            // Final Claim
            if since != 0 {
                Err(BizError::ClaimTransactionInvalid)?;
            }

            let witness_args_bytes = witness_args
                .lock()
                .to_opt()
                .ok_or(BizError::WitnessDataInvalid)?
                .raw_data();
            let claim_witness = ClaimWitness::from_slice(&witness_args_bytes)
                .map_err(|_| BizError::WitnessDataInvalid)?;

            let proof_cell_capacity =
                verify_and_get_proof_cell_capacity(dist_data, &claim_witness)?;
            let input_capacity: u64 = load_cell(0, Source::GroupInput)?.capacity().unpack();
            let reward_amount_unpacked: u64 = dist_data.uniform_reward_amount().unpack();
            if input_capacity != reward_amount_unpacked {
                Err(BizError::FinalClaimCapacityInvalid)?;
            }

            let expected_reward_lock_hash: [u8; 32] = claim_witness.subscriber_lock_hash().into();
            let expected_reward_capacity = reward_amount_unpacked + proof_cell_capacity;

            let mut reward_cell_found = false;
            for i in 0..QueryIter::new(load_cell, Source::Output).count() {
                let output_cell = load_cell(i, Source::Output)?;
                let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;
                let output_capacity: u64 = output_cell.capacity().unpack();

                if output_lock_hash == expected_reward_lock_hash
                    && output_capacity == expected_reward_capacity
                    && output_cell.type_().to_opt().is_none()
                {
                    if reward_cell_found {
                        // Cannot have more than one reward cell.
                        Err(BizError::ClaimTransactionInvalid)?;
                    }
                    reward_cell_found = true;
                }
                // Any other cell is considered a change cell and is ignored.
            }

            if !reward_cell_found {
                Err(BizError::RewardLockHashMismatch)?;
            }
        }
        Err(_) => {
            // Reclamation
            let deadline_unpacked: u64 = dist_data.deadline().unpack();
            if since != (SINCE_TIMESTAMP_FLAG | deadline_unpacked) {
                Err(BizError::ReclamationSinceInvalid)?;
            }

            let expected_admin_lock_hash: [u8; 32] = dist_data.admin_lock_hash().into();
            let input_capacity: u64 = load_cell(0, Source::GroupInput)?.capacity().unpack();

            let mut reclamation_cell_found = false;
            for i in 0..QueryIter::new(load_cell, Source::Output).count() {
                let output_cell = load_cell(i, Source::Output)?;
                let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;
                let output_capacity: u64 = output_cell.capacity().unpack();

                if output_lock_hash == expected_admin_lock_hash
                    && output_capacity == input_capacity
                    && output_cell.type_().to_opt().is_none()
                {
                    if reclamation_cell_found {
                        // Cannot have more than one reclamation cell.
                        Err(BizError::DistributionTransactionInvalid)?;
                    }
                    reclamation_cell_found = true;
                }
                // Any other cell is considered a change cell and is ignored.
            }

            if !reclamation_cell_found {
                Err(BizError::ReclamationLockHashMismatch)?;
            }
        }
    }

    Ok(())
}
