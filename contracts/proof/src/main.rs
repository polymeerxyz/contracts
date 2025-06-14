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
    high_level::{load_cell_data, load_cell_lock_hash, QueryIter},
};
use common::{
    schema::ProofCellData,
    type_id::{load_type_id_from_script_args, validate_type_id},
    NULL_HASH,
};
use molecule::prelude::Entity;
use proof::error::Error;

pub fn program_entry() -> i8 {
    match entry() {
        Ok(()) => 0,
        Err(err) => err.into(),
    }
}

fn entry() -> Result<(), Error> {
    let type_id = load_type_id_from_script_args(0)?;
    validate_type_id(type_id)?;

    let inputs_count = QueryIter::new(load_cell_data, Source::GroupInput).count();
    let outputs_count = QueryIter::new(load_cell_data, Source::GroupOutput).count();

    match (inputs_count, outputs_count) {
        (0, 1) => verify_creation(),                  // creation
        (1, 0) => verify_consumption(),               // consumption
        (1, 1) => Err(Error::InvalidProofCellUpdate), // updation
        _ => Err(Error::InvalidProofTransaction),
    }
}

fn verify_creation() -> Result<(), Error> {
    // 1. Check data structure validity.
    let proof_data_bytes = load_cell_data(0, Source::GroupOutput)?;
    let proof_data =
        ProofCellData::from_slice(&proof_data_bytes).map_err(|_| Error::InvalidProofData)?;

    // 2. Ensure critical identifier hashes are not null/empty.
    if proof_data.entity_id().as_slice() == NULL_HASH {
        return Err(Error::InvalidProofData);
    }

    if proof_data.campaign_id().as_slice() == NULL_HASH {
        return Err(Error::InvalidProofData);
    }

    if proof_data.proof().as_slice() == NULL_HASH {
        return Err(Error::InvalidProofData);
    }

    // 3. Check lock hash is correct.
    let actual_lock_hash = load_cell_lock_hash(0, Source::GroupOutput)?;
    if proof_data.subscriber_lock_hash().as_slice() != actual_lock_hash {
        // We can reuse this error code as it indicates a mismatch related to the lock.
        return Err(Error::InvalidSubscriberLockHash);
    }

    Ok(())
}

fn verify_consumption() -> Result<(), Error> {
    // When a Proof Cell is consumed, we don't need additional validation
    // beyond what's already enforced by the transaction structure checks.
    // The cell is being destroyed, which is allowed.
    Ok(())
}
