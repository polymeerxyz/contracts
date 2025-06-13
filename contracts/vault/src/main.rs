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
    ckb_types::{bytes::Bytes, prelude::*},
    high_level::{
        load_cell, load_cell_data, load_cell_lock, load_cell_lock_hash, load_script, QueryIter,
    },
};
use common::schema::{DistributionCellData, VaultCellData};
use molecule::prelude::Entity;
use vault::{
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
    let inputs_count = QueryIter::new(load_cell, Source::GroupInput).count();
    let outputs_count = QueryIter::new(load_cell, Source::GroupOutput).count();

    match (inputs_count, outputs_count) {
        // Case 1: Consumption (Distribution or Refund)
        // One vault cell is being spent, and no new one is created.
        (1, 0) => {
            let context = load_context()?;

            // Determine the action being performed by inspecting the outputs.
            // The code hash of the Distribution contract must be passed in this Type Script's args.
            let script = load_script()?;
            let args = script.args();
            if args.len() != 32 {
                return Err(Error::InvalidArgsLength);
            }
            let mut dist_lock_code_hash = [0u8; 32];
            dist_lock_code_hash.copy_from_slice(args.as_slice());

            let mut is_distribution_action = false;
            let output_locks = QueryIter::new(load_cell_lock, Source::Output);
            for lock in output_locks {
                if lock.code_hash().as_slice() == dist_lock_code_hash.as_ref() {
                    is_distribution_action = true;
                    break;
                }
            }

            if is_distribution_action {
                verify_distribution(&context, &dist_lock_code_hash)
            } else {
                verify_refund(&context)
            }
        }
        // Case 2: Creation
        // One vault cell is being created.
        (0, 1) => verify_creation(),
        // All other transaction structures are invalid for a Vault Cell.
        _ => Err(Error::InvalidVaultTxStructure),
    }
}

fn verify_distribution(context: &VmContext, dist_lock_code_hash: &[u8; 32]) -> Result<(), Error> {
    let total_capacity = context.vault_capacity;
    let fee_percent: u64 = context.vault_data.fee_percentage().unpack().into();

    if fee_percent > 10000 {
        return Err(Error::InvalidPercentage);
    }

    let expected_fee_capacity = total_capacity * fee_percent / 10000;

    let mut total_dist_capacity_sum: u64 = 0;
    let mut total_fee_capacity: u64 = 0;
    let mut uniform_reward_from_first_shard: Option<u64> = None;

    let output_count = QueryIter::new(load_cell, Source::Output).count();
    if output_count < 2 {
        // A valid distribution must have at least one shard and one fee cell.
        return Err(Error::InvalidVaultTxStructure);
    }

    for i in 0..output_count {
        let output_cell = load_cell(i, Source::Output)?;
        let output_lock = output_cell.lock();
        let output_capacity = output_cell.capacity().unpack();

        if output_lock.code_hash().as_slice() == dist_lock_code_hash {
            // This is a Distribution Shard Cell.
            total_dist_capacity_sum += output_capacity;

            // Validate its data is consistent with the vault.
            let shard_data_bytes = load_cell_data(i, Source::Output)?;
            let shard_data = DistributionCellData::from_slice(&shard_data_bytes)
                .map_err(|_| Error::InvalidDistributionCellData)?;

            if shard_data.campaign_id().as_bytes() != context.vault_data.campaign_id().as_bytes()
                || shard_data.proof_script_code_hash().as_bytes()
                    != context.vault_data.proof_script_code_hash().as_bytes()
            {
                return Err(Error::InvalidDistributionCellData);
            }

            let current_shard_reward = shard_data.uniform_reward_amount().unpack();
            if let Some(first_amount) = uniform_reward_from_first_shard {
                if first_amount != current_shard_reward {
                    return Err(Error::InconsistentShardRewardAmount);
                }
            } else {
                // This is the first shard we've seen. Store its reward amount.
                uniform_reward_from_first_shard = Some(current_shard_reward);
            }
        } else {
            // For other cells, we check the lock hash.
            let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;
            if output_lock_hash == context.admin_lock_hash {
                if total_fee_capacity > 0 {
                    // There can be only one fee cell.
                    return Err(Error::InvalidVaultTxStructure);
                }
                total_fee_capacity += output_capacity;
            } else {
                // Any other cell type (e.g., a refund cell) is forbidden in this action.
                return Err(Error::UnknownVaultAction);
            }
        }
    }

    if total_dist_capacity_sum + total_fee_capacity != total_capacity {
        return Err(Error::DistributionCapacityMismatch);
    }

    if total_fee_capacity != expected_fee_capacity {
        return Err(Error::FeeCapacityMismatch);
    }

    Ok(())
}

fn verify_refund(context: &VmContext) -> Result<(), Error> {
    let output_count = QueryIter::new(load_cell, Source::Output).count();
    if output_count != 1 {
        return Err(Error::UnknownVaultAction);
    }

    let output_lock_hash = load_cell_lock_hash(0, Source::Output)?;
    if output_lock_hash != context.vault_data.creator_lock_hash().as_slice() {
        return Err(Error::RefundLockMismatch);
    }

    Ok(())
}

fn verify_creation() -> Result<(), Error> {
    let vault_data_bytes = load_cell_data(0, Source::GroupOutput)?;
    let vault_data =
        VaultCellData::from_slice(&vault_data_bytes).map_err(|_| Error::InvalidDataStructure)?;

    let fee_percent: u16 = vault_data.fee_percentage().unpack();
    if fee_percent > 10000 {
        return Err(Error::InvalidPercentage);
    }

    // The arguments to this Type Script must contain the 32-byte code_hash
    // of the Distribution contract.
    let self_args: Bytes = load_script()?.args().unpack();
    if self_args.len() != 32 {
        return Err(Error::InvalidArgsLength);
    }

    Ok(())
}
