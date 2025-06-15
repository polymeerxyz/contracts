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
    let context = load_context()?;

    verify_merkle_proof(&context.dist_data, &context.witness)?;

    verify_proof_cell(&context.dist_data, &context.witness)?;

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
        // Validate sibling hash is not all zeros (invalid hash)
        let is_valid_hash = sibling_hash.as_slice().iter().any(|&b| b != 0);
        if !is_valid_hash {
            Err(BizError::InvalidMerkleProof)?;
        }

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
    // Find the input Proof Cell by its Type ID.
    let mut proof_cell_input_index = None;
    let mut proof_cell_count = 0;

    for (i, cell) in QueryIter::new(load_cell, Source::Input).enumerate() {
        let opt_script = cell.type_().to_opt();
        if opt_script.is_some()
            && opt_script.unwrap().code_hash().as_bytes()
                == dist_data.proof_script_code_hash().as_bytes()
        {
            proof_cell_count += 1;
            proof_cell_input_index = Some(i);
        }
    }

    // Ensure exactly one proof cell is being spent
    if proof_cell_count != 1 {
        Err(BizError::InvalidProofCellCount)?;
    }

    let index = proof_cell_input_index.ok_or(BizError::MissingProofCell)?;

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
    let input_dist_cell = load_cell(0, Source::GroupInput)?;
    let input_capacity: u64 = input_dist_cell.capacity().unpack();
    let reward_amount = context.dist_data.uniform_reward_amount().unpack();

    // Check if this is the final claim (no capacity left for another reward)
    let is_final_claim = input_capacity == reward_amount;

    // For final claim: only reward cell
    // For normal claim: reward cell + new distribution cell
    let expected_output_count = if is_final_claim { 1 } else { 2 };
    let total_output_count = QueryIter::new(load_cell, Source::Output).count();

    if total_output_count != expected_output_count {
        Err(BizError::InvalidClaimTransaction)?;
    }

    let mut reward_cell_found = false;
    let mut new_dist_cell_found = false;

    let script_hash = context.script.calc_script_hash();

    for i in 0..total_output_count {
        let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;

        if output_lock_hash == context.witness.subscriber_lock_hash().as_slice() {
            if reward_cell_found {
                Err(BizError::InvalidClaimTransaction)?;
            }
            let reward_cell = load_cell(i, Source::Output)?;
            let reward_cell_capacity: u64 = reward_cell.capacity().unpack();
            if reward_cell_capacity != reward_amount {
                Err(BizError::InvalidRewardAmount)?;
            }
            reward_cell_found = true;
        } else if output_lock_hash == script_hash.as_slice() {
            if is_final_claim {
                Err(BizError::InvalidFinalClaim)?;
            }
            if new_dist_cell_found {
                Err(BizError::InvalidClaimTransaction)?;
            }

            // Verify the new shard cell is a perfect, capacity-reduced clone.
            let input_type_hash = load_cell_type_hash(0, Source::GroupInput)?;
            let output_type_hash = load_cell_type_hash(i, Source::Output)?;
            if input_type_hash != output_type_hash {
                Err(BizError::InvalidTypeScriptUpdate)?;
            }

            let input_data = load_cell_data(0, Source::GroupInput)?;
            let output_data = load_cell_data(i, Source::Output)?;
            if input_data != output_data {
                Err(BizError::InvalidShardDataUpdate)?;
            }

            let output_cell = load_cell(i, Source::Output)?;
            let output_cell_capacity: u64 = output_cell.capacity().unpack();
            if output_cell_capacity != input_capacity - reward_amount {
                Err(BizError::InvalidShardCapacity)?;
            }
            new_dist_cell_found = true;
        } else {
            Err(BizError::InvalidClaimTransaction)?;
        }
    }

    if !reward_cell_found {
        Err(BizError::MissingRewardCell)?;
    }
    if !is_final_claim && !new_dist_cell_found {
        Err(BizError::InvalidClaimTransaction)?;
    }

    Ok(())
}
