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
    high_level::{
        load_cell, load_cell_data, load_cell_lock, load_cell_lock_hash, load_cell_type_hash,
        load_script, QueryIter,
    },
};
use common::{
    schema::{DistributionCellData, VaultCellData},
    NULL_HASH,
};
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
                return Err(Error::InvalidArgumentLength);
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
        _ => Err(Error::InvalidVaultTransaction),
    }
}

fn verify_distribution(context: &VmContext, dist_lock_code_hash: &[u8; 32]) -> Result<(), Error> {
    let total_capacity = context.vault_capacity;
    let fee_percent: u64 = context.vault_data.fee_percentage().unpack().into();

    // Validate fee percentage (0-100%)
    if fee_percent > 10000 {
        return Err(Error::InvalidFeePercentage);
    }

    let expected_fee_capacity = total_capacity * fee_percent / 10000;

    // Ensure the vault has enough capacity to be useful
    if total_capacity <= expected_fee_capacity {
        return Err(Error::InvalidVaultTransaction);
    }

    let mut total_dist_capacity_sum: u64 = 0;
    let mut total_fee_capacity: u64 = 0;
    let mut uniform_reward_from_first_shard: Option<u64> = None;
    let mut shard_count: u32 = 0;

    let output_count = QueryIter::new(load_cell, Source::Output).count();
    if output_count < 2 {
        // A valid distribution must have at least one shard and one fee cell.
        return Err(Error::InvalidVaultTransaction);
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
                .map_err(|_| Error::InvalidDistributionData)?;

            let shard_capacity = output_capacity;
            let current_shard_reward = shard_data.uniform_reward_amount().unpack();

            // 1. The reward amount must be positive.
            if current_shard_reward == 0 {
                return Err(Error::InvalidDistributionData);
            }
            // 2. The reward amount cannot be greater than the shard's capacity.
            if shard_capacity < current_shard_reward {
                return Err(Error::InvalidDistributionData);
            }
            // 3. The shard's capacity must be perfectly divisible by the reward amount
            //    to prevent creating a shard with dust that can never be claimed.
            if shard_capacity % current_shard_reward != 0 {
                return Err(Error::InvalidDistributionData);
            }

            // Verify campaign_id and proof_script_code_hash match the vault
            if shard_data.campaign_id().as_bytes() != context.vault_data.campaign_id().as_bytes()
                || shard_data.proof_script_code_hash().as_bytes()
                    != context.vault_data.proof_script_code_hash().as_bytes()
            {
                return Err(Error::InvalidDistributionData);
            }

            // Verify shard_id is unique and sequential
            let shard_id = shard_data.shard_id().unpack();
            if shard_id != shard_count {
                return Err(Error::InvalidDistributionData);
            }
            shard_count += 1;

            // Verify the reward amount is consistent across all shards
            if let Some(first_amount) = uniform_reward_from_first_shard {
                if first_amount != current_shard_reward {
                    return Err(Error::InvalidShardRewardConsistency);
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
                    return Err(Error::InvalidVaultTransaction);
                }
                total_fee_capacity += output_capacity;
            } else {
                // Any other cell type (e.g., a refund cell) is forbidden in this action.
                return Err(Error::InvalidVaultAction);
            }
        }
    }

    // Ensure we have at least one shard
    if shard_count == 0 {
        return Err(Error::InvalidVaultTransaction);
    }

    // Verify total capacity distribution
    if total_dist_capacity_sum + total_fee_capacity != total_capacity {
        return Err(Error::InvalidDistributionCapacity);
    }

    // Verify fee amount is correct
    if total_fee_capacity != expected_fee_capacity {
        return Err(Error::InvalidFeeCapacity);
    }

    // Verify the uniform reward amount makes sense for the total capacity
    if let Some(reward_amount) = uniform_reward_from_first_shard {
        let available_for_rewards = total_capacity - expected_fee_capacity;
        if reward_amount == 0 || available_for_rewards % reward_amount != 0 {
            return Err(Error::InvalidDistributionData);
        }
    }

    Ok(())
}

fn verify_refund(context: &VmContext) -> Result<(), Error> {
    let output_count = QueryIter::new(load_cell, Source::Output).count();
    if output_count == 0 || output_count > 2 {
        return Err(Error::InvalidVaultTransaction);
    }

    let mut total_refund_capacity: u64 = 0;
    let mut new_vault_capacity: u64 = 0;
    let mut new_vault_found = false;

    // Get the script hash of the vault's type script to identify a new vault cell.
    let self_script_hash = load_script()?.calc_script_hash();

    for i in 0..output_count {
        let output_cell = load_cell(i, Source::Output)?;

        // Check if this output is a new Vault Cell by checking its type script hash.
        let output_type_hash = load_cell_type_hash(i, Source::Output)?;

        if output_type_hash.is_some() && output_type_hash.unwrap() == self_script_hash.unpack() {
            // --- THIS IS A NEW VAULT CELL (PARTIAL REFUND) ---
            if new_vault_found {
                // There can be at most one new Vault Cell.
                return Err(Error::InvalidVaultTransaction);
            }

            // The new vault must be a perfect clone, except for capacity.
            // 1. Verify Lock Script is unchanged.
            let input_lock_hash = load_cell_lock_hash(0, Source::GroupInput)?;
            let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;
            if output_lock_hash != input_lock_hash {
                return Err(Error::InvalidRefundLockHash);
            }

            // 2. Verify Data is unchanged.
            let input_data = load_cell_data(0, Source::GroupInput)?;
            if load_cell_data(i, Source::Output)? != input_data {
                return Err(Error::InvalidVaultDataUpdate);
            }

            new_vault_capacity += output_cell.capacity().unpack();
            new_vault_found = true;
        } else {
            // --- THIS IS NOT A NEW VAULT CELL, SO IT MUST BE THE REFUND CELL ---
            let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;
            if output_lock_hash != context.vault_data.creator_lock_hash().as_slice() {
                // If it's not a new vault and not locked to the creator, it's an invalid output.
                return Err(Error::InvalidRefundLockHash);
            }
            total_refund_capacity += output_cell.capacity().unpack();
        }
    }

    // --- FINAL VALIDATION ---
    // The sum of the new vault's capacity and the refund's capacity must equal
    // the original vault's capacity, ensuring no CKB is lost or created.
    if new_vault_capacity + total_refund_capacity != context.vault_capacity {
        return Err(Error::InvalidDistributionCapacity);
    }

    Ok(())
}

fn verify_creation() -> Result<(), Error> {
    // 1. Check data structure validity.
    let vault_data_bytes = load_cell_data(0, Source::GroupOutput)?;
    let vault_data =
        VaultCellData::from_slice(&vault_data_bytes).map_err(|_| Error::InvalidVaultData)?;

    // 2. Check the fee percentage is valid (0% to 100%).
    let fee_percent = vault_data.fee_percentage().unpack();
    if fee_percent > 10000 {
        return Err(Error::InvalidFeePercentage);
    }

    // 4. Ensure critical identifier hashes are not null/empty.
    if vault_data.campaign_id().as_slice() == NULL_HASH {
        return Err(Error::InvalidVaultData);
    }

    if vault_data.creator_lock_hash().as_slice() == NULL_HASH {
        return Err(Error::InvalidVaultData);
    }

    if vault_data.proof_script_code_hash().as_slice() == NULL_HASH {
        return Err(Error::InvalidVaultData);
    }

    // 3. Check that the script arguments are correctly formatted (must be a 32-byte hash).
    let args = load_script()?.args();
    if args.len() != 32 {
        return Err(Error::InvalidArgumentLength);
    }

    Ok(())
}
