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
    high_level::{load_cell, load_cell_data, load_cell_lock_hash, QueryIter},
    type_id::check_type_id,
};
use common::{schema::proof::ProofCellData, NULL_HASH};
use molecule::prelude::Entity;
use proof::error::{BizError, Error};

pub fn program_entry() -> i8 {
    match entry() {
        Ok(()) => 0,
        Err(err) => err.into(),
    }
}

fn entry() -> Result<(), Error> {
    check_type_id(0)?;

    let inputs_count = QueryIter::new(load_cell, Source::GroupInput).count();
    let outputs_count = QueryIter::new(load_cell, Source::GroupOutput).count();

    match (inputs_count, outputs_count) {
        (0, 1) => verify_creation(),
        (1, 0) => verify_consumption(),
        (1, 1) => Err(BizError::InvalidProofCellUpdate)?,
        _ => Err(BizError::InvalidProofTransaction)?,
    }
}

fn verify_creation() -> Result<(), Error> {
    // 1. Check data structure validity.
    let proof_data_bytes = load_cell_data(0, Source::GroupOutput)?;
    let proof_data =
        ProofCellData::from_slice(&proof_data_bytes).map_err(|_| BizError::InvalidProofData)?;

    // 2. Ensure critical identifier hashes are not null/empty.
    if proof_data.entity_id().as_slice() == NULL_HASH {
        Err(BizError::InvalidProofEntityId)?;
    }

    if proof_data.campaign_id().as_slice() == NULL_HASH {
        Err(BizError::InvalidProofCampaignId)?;
    }

    if proof_data.proof().as_slice() == NULL_HASH {
        Err(BizError::InvalidProofProof)?;
    }

    // 3. Check lock hash is correct.
    let actual_lock_hash = load_cell_lock_hash(0, Source::GroupOutput)?;
    if proof_data.subscriber_lock_hash().as_slice() != actual_lock_hash {
        // We can reuse this error code as it indicates a mismatch related to the lock.
        Err(BizError::InvalidSubscriberLockHash)?;
    }

    Ok(())
}

fn verify_consumption() -> Result<(), Error> {
    // When a Proof Cell is consumed, we don't need additional validation
    // beyond what's already enforced by the transaction structure checks.
    // The cell is being destroyed, which is allowed.
    Ok(())
}
