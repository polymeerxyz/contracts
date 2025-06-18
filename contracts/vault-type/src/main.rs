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
    high_level::{load_cell, load_cell_data, load_cell_lock_hash, load_script, QueryIter},
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
    let outputs_count = QueryIter::new(load_cell, Source::Output).count();

    match (inputs_count, outputs_count) {
        (0, 1) => verify_creation(),
        (1, _) => {
            // full refund or distribute
            let context = load_context()?;
            let scrip_hash = context.script.calc_script_hash();

            // Determine the action being performed by inspecting the outputs.
            // The code hash of the Distribution contract must be passed in this Type Script's args.
            let args = context.script.args();
            if args.raw_data().len() != 64 {
                Err(BizError::InvalidArgumentLength)?;
            }

            let mut dist_lock_code_hash = [0u8; 32];
            dist_lock_code_hash.copy_from_slice(&args.raw_data()[0..32]);
            let mut dist_type_code_hash = [0u8; 32];
            dist_type_code_hash.copy_from_slice(&args.raw_data()[32..64]);

            let mut is_distribution_action = false;
            let mut is_partial_refund_action = false;

            for i in 0..outputs_count {
                let cell = load_cell(i, Source::Output)?;
                if let Some(type_script) = cell.type_().to_opt() {
                    if type_script.calc_script_hash() == scrip_hash {
                        is_partial_refund_action = true;
                        break;
                    }
                }

                if cell.lock().code_hash().as_slice() == dist_lock_code_hash.as_ref() {
                    is_distribution_action = true;
                    break;
                }
            }

            if is_distribution_action {
                verify_distribution(&context, &dist_lock_code_hash, &dist_type_code_hash)
            } else if is_partial_refund_action {
                verify_partial_refund(&context)
            } else {
                verify_full_refund(&context)
            }
        }
        _ => Err(BizError::InvalidVaultTransaction)?,
    }
}

fn verify_creation() -> Result<(), Error> {
    if load_script()?.args().raw_data().len() != 64 {
        Err(BizError::InvalidArgumentLength)?;
    }

    let vault_data_bytes = load_cell_data(0, Source::GroupOutput)?;
    let vault_data =
        VaultCellData::from_slice(&vault_data_bytes).map_err(|_| BizError::InvalidVaultData)?;

    let fee_percent = vault_data.fee_percentage().unpack();
    if fee_percent > 10000 {
        Err(BizError::InvalidFeePercentage)?;
    }

    if vault_data.campaign_id().as_slice() == NULL_HASH {
        Err(BizError::InvalidVaultData)?;
    }

    if vault_data.creator_lock_hash().as_slice() == NULL_HASH {
        Err(BizError::InvalidVaultData)?;
    }

    if vault_data.proof_script_code_hash().as_slice() == NULL_HASH {
        Err(BizError::InvalidVaultData)?;
    }

    Ok(())
}

fn verify_distribution(
    context: &VmContext,
    dist_lock_code_hash: &[u8; 32],
    dist_type_code_hash: &[u8; 32],
) -> Result<(), Error> {
    let fee_percent: u64 = context.vault_data.fee_percentage().unpack().into();
    if fee_percent > 10000 {
        Err(BizError::InvalidFeePercentage)?;
    }

    let total_capacity = context.vault_capacity;
    let expected_fee_capacity = total_capacity * fee_percent / 10000;

    if total_capacity <= expected_fee_capacity {
        Err(BizError::InvalidVaultTransaction)?;
    }

    let mut total_dist_shards_capacity: u64 = 0;
    let mut total_fee_capacity: u64 = 0;
    let mut uniform_reward_amount: Option<u64> = None;

    let output_count = QueryIter::new(load_cell, Source::Output).count();
    if output_count < 2 {
        Err(BizError::InvalidVaultTransaction)?;
    }

    for i in 0..output_count {
        let output_cell = load_cell(i, Source::Output)?;
        let output_lock = output_cell.lock();
        let output_capacity: u64 = output_cell.capacity().unpack();

        if output_lock.code_hash().as_slice() == dist_lock_code_hash {
            // distribution cell
            if let Some(type_script) = output_cell.type_().to_opt() {
                // distribution cell must have both lock and type binding to distribution contracts
                if type_script.code_hash().as_slice() != dist_type_code_hash {
                    Err(BizError::InvalidDistributionData)?;
                }
            } else {
                Err(BizError::InvalidDistributionData)?;
            }

            total_dist_shards_capacity += output_capacity;

            let shard_data_bytes = load_cell_data(i, Source::Output)?;
            let shard_data = DistributionCellData::from_slice(&shard_data_bytes)
                .map_err(|_| BizError::InvalidDistributionData)?;

            // Validate data consistency.
            if shard_data.campaign_id().as_bytes() != context.vault_data.campaign_id().as_bytes()
                || shard_data.proof_script_code_hash().as_bytes()
                    != context.vault_data.proof_script_code_hash().as_bytes()
                || shard_data.admin_lock_hash().as_slice() != context.admin_lock_hash.as_ref()
            {
                Err(BizError::InvalidDistributionData)?;
            }

            let current_shard_reward: u64 = shard_data.uniform_reward_amount().unpack();
            if let Some(first_amount) = uniform_reward_amount {
                if first_amount != current_shard_reward {
                    Err(BizError::InvalidShardRewardConsistency)?;
                }
            } else {
                uniform_reward_amount = Some(current_shard_reward);
            }
        } else {
            // refund cell
            let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;
            if output_lock_hash == context.admin_lock_hash {
                if total_fee_capacity > 0 {
                    Err(BizError::InvalidVaultTransaction)?;
                }
                total_fee_capacity += output_capacity;
            } else {
                Err(BizError::InvalidVaultAction)?;
            }
        }
    }

    if total_dist_shards_capacity + total_fee_capacity != total_capacity {
        Err(BizError::InvalidDistributionCapacity)?;
    }

    if total_fee_capacity < expected_fee_capacity {
        Err(BizError::InvalidFeeCapacity)?;
    }

    Ok(())
}

fn verify_partial_refund(context: &VmContext) -> Result<(), Error> {
    let output_count = QueryIter::new(load_cell, Source::Output).count();
    if output_count != 2 {
        Err(BizError::InvalidVaultTransaction)?;
    }

    let mut new_vault_cell_found = false;
    let mut refund_cell_found = false;
    let mut total_output_capacity: u64 = 0;

    // Get the script hash of the vault's type script to identify a new vault cell.
    let script_hash = context.script.calc_script_hash();
    let creator_lock_hash: [u8; 32] = context.vault_data.creator_lock_hash().into();
    let input_data = load_cell_data(0, Source::GroupInput)?;

    for i in 0..2 {
        let output_cell = load_cell(i, Source::Output)?;
        let output_cell_capacity: u64 = output_cell.capacity().unpack();
        total_output_capacity += output_cell_capacity;

        let mut is_new_vault_cell = false;
        if let Some(type_script) = output_cell.type_().to_opt() {
            if type_script.calc_script_hash() == script_hash {
                is_new_vault_cell = true;
            }
        }

        let mut is_refund_cell = false;
        if load_cell_lock_hash(i, Source::Output)? == creator_lock_hash {
            is_refund_cell = true;
        }

        if is_new_vault_cell {
            if new_vault_cell_found {
                Err(BizError::InvalidPartialRefund)?;
            }
            if input_data != load_cell_data(i, Source::Output)? {
                Err(BizError::InvalidPartialRefund)?;
            }
            new_vault_cell_found = true;
        } else if is_refund_cell {
            if refund_cell_found {
                Err(BizError::InvalidPartialRefund)?;
            }
            refund_cell_found = true;
        } else {
            // If the cell is neither, the transaction structure is invalid.
            Err(BizError::InvalidPartialRefund)?;
        }
    }

    if !new_vault_cell_found || !refund_cell_found {
        Err(BizError::InvalidPartialRefund)?;
    }

    if total_output_capacity != context.vault_capacity {
        Err(BizError::InvalidPartialRefund)?;
    }

    Ok(())
}

fn verify_full_refund(context: &VmContext) -> Result<(), Error> {
    if QueryIter::new(load_cell, Source::Output).count() != 1 {
        Err(BizError::InvalidVaultTransaction)?;
    }

    let output_lock_hash = load_cell_lock_hash(0, Source::Output)?;
    if output_lock_hash != context.vault_data.creator_lock_hash().as_slice() {
        Err(BizError::InvalidRefundLockHash)?;
    }

    Ok(())
}
