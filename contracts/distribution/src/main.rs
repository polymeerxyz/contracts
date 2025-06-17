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

use ckb_hash::new_blake2b;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    debug,
    high_level::{
        load_cell, load_cell_data, load_cell_lock_hash, load_cell_type_hash, load_input_out_point,
        QueryIter,
    },
};
use common::schema::{
    distribution::{ClaimWitness, DistributionCellData},
    proof::ProofCellData,
};
use distribution::{
    context::{load_context, VmContext},
    error::{BizError, Error},
};

pub fn program_entry() -> i8 {
    match entry() {
        Ok(()) => 0,
        Err(err) => err.into(),
    }
}

fn entry() -> Result<(), Error> {
    debug!("distribution contract is executing");

    // A claim transaction can only spend one Distribution Shard Cell at a time.
    // This is implicitly checked by `load_context` which loads from `Source::GroupInput` at index 0.
    // A claim transaction can create 0 (final claim) or 1 (normal claim) new Distribution Shard Cells.
    let group_outputs_count = QueryIter::new(load_cell, Source::GroupOutput).count();
    if group_outputs_count > 1 {
        Err(BizError::InvalidClaimTransaction)?
    }

    let context = load_context()?;
    verify_merkle_proof(&context.dist_data, &context.claim_witness)?;
    verify_proof_cell(&context.dist_data, &context.claim_witness)?;
    verify_outputs(&context)?;

    Ok(())
}

fn verify_merkle_proof(
    dist_data: &DistributionCellData,
    witness: &ClaimWitness,
) -> Result<(), Error> {
    // Create the leaf hash from the proof cell outpoint and subscriber lock hash
    let mut leaf_hasher = new_blake2b();
    leaf_hasher.update(&witness.proof_cell_out_point().as_bytes());
    leaf_hasher.update(&witness.subscriber_lock_hash().as_bytes());
    let mut leaf_hash = [0u8; 32];
    leaf_hasher.finalize(&mut leaf_hash);

    // Verify the merkle path
    let mut computed_hash = leaf_hash;
    for sibling_hash in witness.merkle_proof().into_iter() {
        // Compute parent hash by concatenating and hashing the two child hashes
        let mut parent_hasher = new_blake2b();
        if computed_hash.as_ref() < sibling_hash.as_slice() {
            parent_hasher.update(&computed_hash);
            parent_hasher.update(sibling_hash.as_slice());
        } else {
            parent_hasher.update(sibling_hash.as_slice());
            parent_hasher.update(&computed_hash);
        }
        parent_hasher.finalize(&mut computed_hash);
    }

    // Final computed hash must match the merkle root in the distribution cell
    if computed_hash != dist_data.merkle_root().as_slice() {
        Err(BizError::InvalidMerkleProof)?;
    }

    Ok(())
}

fn verify_proof_cell(
    dist_data: &DistributionCellData,
    witness: &ClaimWitness,
) -> Result<(), Error> {
    let proof_cell_indices = QueryIter::new(load_cell, Source::Input)
        .enumerate()
        .filter_map(|(idx, cell)| {
            let code_hash_opt = cell.type_().to_opt().map(|s| s.code_hash().as_bytes());
            if Some(dist_data.proof_script_code_hash().as_bytes()) == code_hash_opt {
                Some(idx)
            } else {
                None
            }
        })
        .collect::<alloc::vec::Vec<_>>();

    // Ensure exactly one proof cell is being spent
    if proof_cell_indices.len() != 1 {
        Err(BizError::InvalidProofCellCount)?;
    }

    let index = proof_cell_indices[0];

    // Check 1: The OutPoint in the witness must match the actual OutPoint being spent.
    let actual_proof_outpoint = load_input_out_point(index, Source::Input)?;
    if actual_proof_outpoint.as_bytes() != witness.proof_cell_out_point().as_bytes() {
        Err(BizError::InvalidMerkleProof)?;
    }

    // Check 2: Inspect the Proof Cell's internal data for consistency.
    let proof_cell_data_bytes = load_cell_data(index, Source::Input)?;
    let proof_data = ProofCellData::from_slice(&proof_cell_data_bytes)
        .map_err(|_| BizError::InvalidProofData)?;

    if proof_data.campaign_id().as_bytes() != dist_data.campaign_id().as_bytes() {
        Err(BizError::InvalidCampaignId)?;
    }
    if proof_data.subscriber_lock_hash().as_bytes() != witness.subscriber_lock_hash().as_bytes() {
        Err(BizError::InvalidSubscriberLockHash)?;
    }

    // Check 3: Verify the proof cell's lock hash matches the subscriber's lock hash
    let proof_cell_lock_hash = load_cell_lock_hash(index, Source::Input)?;
    if proof_cell_lock_hash != witness.subscriber_lock_hash().as_slice() {
        Err(BizError::InvalidProofLockHash)?;
    }

    Ok(())
}

/// Verifies the transaction's outputs (Reward Cell and new Distribution Shard Cell).
fn verify_outputs(context: &VmContext) -> Result<(), Error> {
    let reward_amount = context.dist_data.uniform_reward_amount().unpack();
    let script_hash = context.script.calc_script_hash();
    let total_output_count = QueryIter::new(load_cell, Source::Output).count();

    // Find if a new distribution shard cell is created
    let new_dist_cell_output_indices: alloc::vec::Vec<_> =
        QueryIter::new(load_cell, Source::Output)
            .enumerate()
            .filter(|(_i, cell)| {
                cell.lock().calc_script_hash().as_bytes() == script_hash.as_bytes()
            })
            .map(|(i, _cell)| i)
            .collect();

    if new_dist_cell_output_indices.len() > 1 {
        // Cannot create more than one new shard cell
        debug!("1");
        return Err(BizError::InvalidClaimTransaction.into());
    }

    if !new_dist_cell_output_indices.is_empty() {
        // --- NORMAL CLAIM ---
        let index = new_dist_cell_output_indices[0];

        // Expect 2 outputs: reward cell and new dist cell.
        if total_output_count != 2 {
            debug!("2");
            return Err(BizError::InvalidClaimTransaction.into());
        }

        // Verify the new shard cell
        let output_cell = load_cell(index, Source::Output)?;
        let output_cell_capacity: u64 = output_cell.capacity().unpack();

        // The type script of a Distribution Shard Cell must be null.
        if load_cell_type_hash(index, Source::Output)?.is_some() {
            return Err(BizError::InvalidTypeScriptUpdate.into());
        }

        let input_data = load_cell_data(0, Source::GroupInput)?;
        let output_data = load_cell_data(index, Source::Output)?;
        if input_data != output_data {
            return Err(BizError::InvalidShardDataUpdate.into());
        }

        if output_cell_capacity != context.dist_capacity - reward_amount {
            return Err(BizError::InvalidShardCapacity.into());
        }

        // Find and verify the reward cell.
        let mut reward_cell_found = false;
        for i in 0..total_output_count {
            if i == index {
                continue;
            } // Skip the new dist cell
            let reward_cell = load_cell(i, Source::Output)?;
            let reward_lock_hash = load_cell_lock_hash(i, Source::Output)?;
            if reward_lock_hash == context.claim_witness.subscriber_lock_hash().as_slice() {
                let reward_cell_capacity: u64 = reward_cell.capacity().unpack();
                if reward_cell_capacity != reward_amount {
                    return Err(BizError::InvalidRewardAmount.into());
                }
                reward_cell_found = true;
            } else {
                // Some other unexpected cell
                debug!("3");
                return Err(BizError::InvalidClaimTransaction.into());
            }
        }
        if !reward_cell_found {
            return Err(BizError::MissingRewardCell.into());
        }
    } else {
        // --- FINAL CLAIM ---
        let dust_capacity = context.dist_capacity - reward_amount;

        let expected_output_count = if dust_capacity > 0 { 2 } else { 1 };
        if total_output_count != expected_output_count {
            debug!("4");
            return Err(BizError::InvalidClaimTransaction.into());
        }

        let mut reward_cell_found = false;
        let mut admin_refund_cell_found = false;

        for i in 0..total_output_count {
            let output_cell = load_cell(i, Source::Output)?;
            let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;
            let capacity: u64 = output_cell.capacity().unpack();

            if output_lock_hash == context.claim_witness.subscriber_lock_hash().as_slice() {
                if reward_cell_found {
                    debug!("5");
                    return Err(BizError::InvalidClaimTransaction.into());
                }
                if capacity != reward_amount {
                    return Err(BizError::InvalidRewardAmount.into());
                }
                reward_cell_found = true;
            } else if output_lock_hash == context.admin_lock_hash.as_ref() {
                if admin_refund_cell_found {
                    debug!("6");
                    return Err(BizError::InvalidClaimTransaction.into());
                }
                if capacity != dust_capacity {
                    return Err(BizError::InvalidAdminRefundAmount.into());
                }
                admin_refund_cell_found = true;
            } else {
                debug!("7");
                return Err(BizError::InvalidClaimTransaction.into());
            }
        }

        if !reward_cell_found {
            return Err(BizError::MissingRewardCell.into());
        }
        if dust_capacity > 0 && !admin_refund_cell_found {
            return Err(BizError::MissingAdminRefundCell.into());
        }
    }

    Ok(())
}
