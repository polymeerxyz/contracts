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
use common::schema::{ClaimWitness, DistributionCellData, ProofCellData};
use distribution::{
    context::{load_context, VmContext},
    error::Error,
};

pub fn program_entry() -> i8 {
    match entry() {
        Ok(()) => 0,
        Err(err) => err.into(),
    }
}

fn entry() -> Result<(), Error> {
    // 1. Load all necessary data from the transaction into a context struct.
    let context = load_context()?;

    // 2. Verify the claimant is on the list using the Merkle proof.
    verify_merkle_proof(&context.dist_data, &context.witness)?;

    // 3. Verify the input Proof Cell is valid and consistent.
    verify_proof_cell(&context.dist_data, &context.witness)?;

    // 4. Verify the outputs are correctly structured and funded.
    verify_outputs(&context)?;

    Ok(())
}

fn verify_merkle_proof(
    dist_data: &DistributionCellData,
    witness: &ClaimWitness,
) -> Result<(), Error> {
    let mut leaf_hasher = new_blake2b();
    leaf_hasher.update(&witness.proof_cell_out_point().as_bytes());
    leaf_hasher.update(&witness.subscriber_lock_hash().as_bytes());
    let mut leaf_hash = [0u8; 32];
    leaf_hasher.finalize(&mut leaf_hash);

    let mut computed_hash = leaf_hash;
    for sibling_hash in witness.merkle_proof().into_iter() {
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

    if computed_hash != dist_data.merkle_root().as_slice() {
        return Err(Error::InvalidMerkleProof);
    }

    Ok(())
}

fn verify_proof_cell(
    dist_data: &DistributionCellData,
    witness: &ClaimWitness,
) -> Result<(), Error> {
    // Find the input Proof Cell by its Type ID.
    let mut proof_cell_input_index = None;
    for (i, cell) in QueryIter::new(load_cell, Source::Input).enumerate() {
        let opt_script = cell.type_().to_opt();
        if opt_script.is_some()
            && opt_script.unwrap().code_hash().as_bytes()
                == dist_data.proof_script_code_hash().as_bytes()
        {
            proof_cell_input_index = Some(i);
            break;
        }
    }
    let index = proof_cell_input_index.ok_or(Error::ProofCellNotFound)?;

    // Check 1: The OutPoint in the witness must match the actual OutPoint being spent.
    let actual_proof_outpoint = load_input_out_point(index, Source::Input)?;
    if actual_proof_outpoint.as_bytes() != witness.proof_cell_out_point().as_bytes() {
        return Err(Error::InvalidMerkleProof);
    }

    // Check 2: Inspect the Proof Cell's internal data for consistency.
    let proof_cell_data_bytes = load_cell_data(index, Source::Input)?;
    let proof_data = ProofCellData::from_slice(&proof_cell_data_bytes)
        .map_err(|_| Error::InvalidDataStructure)?;

    if proof_data.campaign_id().as_bytes() != dist_data.campaign_id().as_bytes() {
        return Err(Error::CampaignIdMismatch);
    }
    if proof_data.subscriber_lock_hash().as_bytes() != witness.subscriber_lock_hash().as_bytes() {
        return Err(Error::RewardLockMismatch);
    }

    Ok(())
}

/// Verifies the transaction's outputs (Reward Cell and new Distribution Shard Cell).
fn verify_outputs(context: &VmContext) -> Result<(), Error> {
    let input_dist_cell = load_cell(0, Source::GroupInput)?;
    let input_capacity = input_dist_cell.capacity().unpack();
    let reward_amount = context.dist_data.uniform_reward_amount().unpack();

    let is_final_claim = input_capacity == reward_amount;
    let expected_output_count = if is_final_claim { 1 } else { 2 };
    let total_output_count = QueryIter::new(load_cell, Source::Output).count();
    if total_output_count != expected_output_count {
        return Err(Error::InvalidClaimTxStructure);
    }

    let mut reward_cell_found = false;
    let mut new_dist_cell_found = false;

    for i in 0..total_output_count {
        let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;

        if output_lock_hash == context.witness.subscriber_lock_hash().as_slice() {
            if reward_cell_found {
                return Err(Error::InvalidClaimTxStructure);
            }
            let reward_cell = load_cell(i, Source::Output)?;
            if reward_cell.capacity().unpack() != reward_amount {
                return Err(Error::RewardAmountMismatch);
            }
            reward_cell_found = true;
        } else if output_lock_hash == context.script.calc_script_hash().as_slice() {
            if is_final_claim {
                return Err(Error::InvalidFinalClaim);
            }
            if new_dist_cell_found {
                return Err(Error::InvalidClaimTxStructure);
            }

            // Verify the new shard cell is a perfect, capacity-reduced clone.
            let input_type_hash = load_cell_type_hash(0, Source::GroupInput)?;
            let output_type_hash = load_cell_type_hash(i, Source::Output)?;
            if input_type_hash != output_type_hash {
                return Err(Error::TypeScriptImmutable);
            }

            let input_data = load_cell_data(0, Source::GroupInput)?;
            let output_data = load_cell_data(i, Source::Output)?;
            if input_data != output_data {
                return Err(Error::ShardDataImmutable);
            }

            let output_cell = load_cell(i, Source::Output)?;
            if output_cell.capacity().unpack() != input_capacity - reward_amount {
                return Err(Error::ShardCapacityMismatch);
            }
            new_dist_cell_found = true;
        } else {
            return Err(Error::InvalidClaimTxStructure);
        }
    }

    if !reward_cell_found {
        return Err(Error::RewardCellNotFound);
    }
    if !is_final_claim && !new_dist_cell_found {
        return Err(Error::InvalidClaimTxStructure);
    }

    Ok(())
}
