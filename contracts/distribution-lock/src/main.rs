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
    high_level::{load_cell_data, load_header, load_witness_args},
};
use common::schema::distribution::{ClaimWitness, DistributionCellData};
use distribution_lock::error::{BizError, Error};

pub fn program_entry() -> i8 {
    match entry() {
        Ok(()) => 0,
        Err(err) => err.into(),
    }
}

fn entry() -> Result<(), Error> {
    debug!("distribution lock contract is executing");

    let dist_data_bytes = load_cell_data(0, Source::GroupInput)?;
    let dist_data = DistributionCellData::from_slice(&dist_data_bytes)
        .map_err(|_| BizError::DistributionDataInvalid)?;

    let header = load_header(0, Source::HeaderDep)?;
    let block_timestamp: u64 = header.raw().timestamp().unpack();
    let deadline_timestamp: u64 = dist_data.deadline().unpack();

    match load_witness_args(0, Source::GroupInput) {
        Ok(witness_args) => {
            if block_timestamp >= deadline_timestamp {
                Err(BizError::ClaimTimeInvalid)?;
            }

            let witness_args_bytes = witness_args
                .lock()
                .to_opt()
                .ok_or(BizError::WitnessDataInvalid)?
                .raw_data();

            let claim_witness = ClaimWitness::from_slice(&witness_args_bytes)
                .map_err(|_| BizError::WitnessDataInvalid)?;

            verify_merkle_proof(&dist_data, &claim_witness)?;
        }
        Err(_) => {
            if block_timestamp < deadline_timestamp {
                Err(BizError::ReclaimTimeInvalid)?;
            }

            verify_reclamation(&dist_data)?;
        }
    }
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
            parent_hasher.update(&sibling_hash.as_bytes());
        } else {
            parent_hasher.update(&sibling_hash.as_bytes());
            parent_hasher.update(&computed_hash);
        }
        parent_hasher.finalize(&mut computed_hash);
    }

    // Final computed hash must match the merkle root in the distribution cell
    if computed_hash != dist_data.merkle_root().as_slice() {
        Err(BizError::MerkleProofInvalid)?;
    }

    Ok(())
}

fn verify_reclamation(_: &DistributionCellData) -> Result<(), Error> {
    // If past the deadline, this lock script allows the spend.
    // The Type Script will enforce WHERE the funds can go.
    Ok(())
}
