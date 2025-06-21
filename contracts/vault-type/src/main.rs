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
    high_level::{load_cell, load_cell_data, load_script, QueryIter},
};
use common::{
    schema::{distribution::DistributionCellData, vault::VaultCellData},
    NULL_HASH,
};
use molecule::prelude::Entity;
use vault_type::{
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
    debug!("vault type contract is executing");

    let inputs_count = QueryIter::new(load_cell, Source::GroupInput).count();
    let outputs_count = QueryIter::new(load_cell, Source::GroupOutput).count();

    debug!(
        "inputs count :{}, outputs count: {}",
        inputs_count, outputs_count
    );

    match (inputs_count, outputs_count) {
        (0, 1) => {
            debug!("vault creation transaction");
            // Creation of a new vault.
            verify_creation()
        }
        (1, 1) => {
            debug!("vault capacity adjustment transaction");
            // Update of the vault, must be a capacity adjustment (increase/decrease).
            // vault-lock ensures this is signed by the creator.
            let context = load_context()?;
            verify_capacity_adjustment(&context)
        }
        (1, 0) => {
            // Destruction of the vault. This can be either a distribution or a full refund.
            let context = load_context()?;
            let args = load_script()?.args();
            let args_bytes = args.raw_data();

            if args_bytes.len() != 64 {
                Err(BizError::ArgumentLengthInvalid)?;
            }
            // Args are valid, so this *could* be a distribution.
            let mut dist_lock_code_hash = [0u8; 32];
            dist_lock_code_hash.copy_from_slice(&args_bytes[0..32]);

            let has_dist_shard = QueryIter::new(load_cell, Source::Output)
                .any(|cell| cell.lock().code_hash().as_slice() == dist_lock_code_hash);

            if has_dist_shard {
                // This is a distribution.
                // vault-lock ensures this is signed by the admin.
                debug!("vault distribution transaction");
                let mut dist_type_code_hash = [0u8; 32];
                dist_type_code_hash.copy_from_slice(&args_bytes[32..64]);
                verify_distribution(&context, &dist_lock_code_hash, &dist_type_code_hash)
            } else {
                // No distribution shards found, so it's a full refund.
                debug!("vault destruction transaction (full refund)");
                verify_full_refund(&context)
            }
        }
        _ => Err(BizError::VaultTransactionInvalid)?,
    }
}

fn verify_creation() -> Result<(), Error> {
    if load_script()?.args().raw_data().len() != 64 {
        Err(BizError::ArgumentLengthInvalid)?;
    }

    let vault_data_bytes = load_cell_data(0, Source::GroupOutput)?;
    let vault_data =
        VaultCellData::from_slice(&vault_data_bytes).map_err(|_| BizError::VaultDataInvalid)?;

    let fee_percentage: u16 = vault_data.fee_percentage().unpack();
    if fee_percentage > 10000 {
        Err(BizError::FeePercentageOutOfRange)?;
    }

    if vault_data.campaign_id().as_slice() == NULL_HASH {
        Err(BizError::VaultDataInvalid)?;
    }

    if vault_data.proof_script_code_hash().as_slice() == NULL_HASH {
        Err(BizError::VaultDataInvalid)?;
    }

    Ok(())
}

fn verify_distribution(
    context: &VmContext,
    dist_lock_code_hash: &[u8; 32],
    dist_type_code_hash: &[u8; 32],
) -> Result<(), Error> {
    // 1. Find all distribution shards and validate their data.
    let mut total_dist_shards_capacity: u64 = 0;
    let mut uniform_reward_amount: Option<u64> = None;

    let dist_shards = QueryIter::new(load_cell, Source::Output)
        .enumerate()
        .filter(|(_i, cell)| cell.lock().code_hash().as_slice() == dist_lock_code_hash)
        .collect::<alloc::vec::Vec<_>>();

    if dist_shards.is_empty() {
        // Must create at least one shard for a distribution.
        Err(BizError::DistributionDataInvalid)?;
    }

    for (i, shard_cell) in dist_shards {
        // Check for distribution type script
        let type_script = shard_cell
            .type_()
            .to_opt()
            .ok_or(BizError::DistributionDataInvalid)?;
        if type_script.code_hash().as_slice() != dist_type_code_hash {
            Err(BizError::DistributionDataInvalid)?;
        }

        let shard_capacity: u64 = shard_cell.capacity().unpack();
        total_dist_shards_capacity += shard_capacity;

        let shard_data_bytes = load_cell_data(i, Source::Output)?;
        let shard_data = DistributionCellData::from_slice(&shard_data_bytes)
            .map_err(|_| BizError::DistributionDataInvalid)?;

        // On the first shard, capture the reward amount for consistency checks.
        if uniform_reward_amount.is_none() {
            let reward: u64 = shard_data.uniform_reward_amount().unpack();
            if reward == 0 {
                Err(BizError::ShardRewardInconsistent)?;
            }
            uniform_reward_amount = Some(reward);
        }

        // Validate data consistency against the vault and the first shard.
        if shard_data.campaign_id().as_bytes() != context.vault_data.campaign_id().as_bytes()
            || shard_data.proof_script_code_hash().as_bytes()
                != context.vault_data.proof_script_code_hash().as_bytes()
            // The admin lock hash in the shard must match the one from the vault's lock.
            || shard_data.admin_lock_hash().as_slice() != context.admin_lock_hash
            || shard_data.uniform_reward_amount().unpack() != uniform_reward_amount.unwrap()
        {
            Err(BizError::DistributionDataInvalid)?;
        }
    }

    // 2. Calculate expected fee
    let fee_percentage: u128 = context.vault_data.fee_percentage().unpack().into();
    if fee_percentage > 10000 {
        Err(BizError::FeePercentageOutOfRange)?;
    }
    let total_capacity = context.vault_capacity;
    let expected_fee_capacity = (total_capacity as u128 * fee_percentage / 10000) as u64;

    if total_capacity < expected_fee_capacity {
        Err(BizError::VaultTransactionInvalid)?;
    }

    // 3. Verify capacity partitioning
    if total_dist_shards_capacity + expected_fee_capacity != total_capacity {
        Err(BizError::CapacityMismatch)?;
    }

    // 4. Find and validate the fee cell.
    let fee_cells_count = QueryIter::new(load_cell, Source::Output)
        .filter(|cell| {
            let lock_hash = cell.lock().calc_script_hash();
            let capacity: u64 = cell.capacity().unpack();
            lock_hash.as_slice() == context.admin_lock_hash
                && capacity == expected_fee_capacity
                && cell.type_().to_opt().is_none()
        })
        .count();

    if fee_cells_count != 1 {
        // Must be exactly one cell matching the fee criteria.
        Err(BizError::FeeCapacityMismatch)?;
    }

    Ok(())
}

fn verify_capacity_adjustment(context: &VmContext) -> Result<(), Error> {
    let input_vault_cell = load_cell(0, Source::GroupInput)?;
    let output_vault_cell = load_cell(0, Source::GroupOutput)?;
    let output_vault_data_bytes = load_cell_data(0, Source::GroupOutput)?;

    if context.vault_data.as_slice() != output_vault_data_bytes {
        Err(BizError::VaultDataImmutable)?;
    }

    if input_vault_cell.lock().as_slice() != output_vault_cell.lock().as_slice() {
        Err(BizError::VaultLockScriptImmutable)?;
    }

    let output_vault_capacity: u64 = output_vault_cell.capacity().unpack();

    if output_vault_capacity >= context.vault_capacity {
        return Ok(());
    }

    // Sum capacity of all outputs going to the creator.
    // Note: The output vault cell itself is NOT locked by the creator, so it won't be counted here.
    let creator_output_capacity: u64 = QueryIter::new(load_cell, Source::Output)
        .filter_map(|cell| {
            if cell.lock().calc_script_hash().as_slice() == context.creator_lock_hash {
                let cell_capacity: u64 = cell.capacity().unpack();
                Some(cell_capacity)
            } else {
                None
            }
        })
        .sum();

    if output_vault_capacity + creator_output_capacity < context.vault_capacity {
        Err(BizError::CapacityAdjustmentInvalid)?;
    }

    Ok(())
}

fn verify_full_refund(context: &VmContext) -> Result<(), Error> {
    // In a full refund, the vault's capacity must be returned to the creator.
    // We verify this by checking that the sum of capacities of output cells
    // locked with the creator's lock hash is at least the vault's capacity.
    // The vault-lock script ensures the creator signed the transaction, so they
    // are in control of any other inputs/outputs. The CKB VM's balance check
    // handles the rest.

    let creator_output_capacity: u64 = QueryIter::new(load_cell, Source::Output)
        .filter_map(|cell| {
            if cell.lock().calc_script_hash().as_slice() == context.creator_lock_hash {
                let cell_capacity: u64 = cell.capacity().unpack();
                Some(cell_capacity)
            } else {
                None
            }
        })
        .sum();

    if creator_output_capacity < context.vault_capacity {
        Err(BizError::CapacityAdjustmentInvalid)?;
    }

    Ok(())
}
