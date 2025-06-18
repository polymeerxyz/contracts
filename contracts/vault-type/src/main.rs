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
    let outputs_count = QueryIter::new(load_cell, Source::GroupOutput).count();

    debug!(
        "inputs count :{}, outputs count: {}",
        inputs_count, outputs_count
    );

    match (inputs_count, outputs_count) {
        (0, 1) => verify_creation(),
        (1, 0) => {
            let context = load_context()?;

            // Determine the action being performed by inspecting the outputs.
            // The code hash of the Distribution contract must be passed in this Type Script's args.
            let args = context.script.args();
            if args.raw_data().len() != 64 {
                Err(BizError::ArgumentLengthInvalid)?;
            }

            let mut dist_lock_code_hash = [0u8; 32];
            dist_lock_code_hash.copy_from_slice(&args.raw_data()[0..32]);

            let is_distribution_action = QueryIter::new(load_cell, Source::Output)
                .any(|cell| cell.lock().code_hash().as_slice() == dist_lock_code_hash.as_ref());

            if is_distribution_action {
                let mut dist_type_code_hash = [0u8; 32];
                dist_type_code_hash.copy_from_slice(&args.raw_data()[32..64]);
                verify_distribution(&context, &dist_lock_code_hash, &dist_type_code_hash)
            } else {
                verify_full_refund(&context)
            }
        }
        (1, 1) => {
            let context = load_context()?;
            verify_partial_refund(&context)
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

    let fee_percentage_unpacked: u16 = vault_data.fee_percentage().unpack();
    if fee_percentage_unpacked > 10000 {
        Err(BizError::FeePercentageOutOfRange)?;
    }

    if vault_data.campaign_id().as_slice() == NULL_HASH {
        Err(BizError::VaultDataInvalid)?;
    }

    if vault_data.creator_lock_hash().as_slice() == NULL_HASH {
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
    let fee_percent: u128 = context.vault_data.fee_percentage().unpack().into();
    if fee_percent > 10000 {
        Err(BizError::FeePercentageOutOfRange)?;
    }

    let total_capacity = context.vault_capacity;
    let expected_fee_capacity = (total_capacity as u128 * fee_percent / 10000) as u64;

    if total_capacity < expected_fee_capacity {
        debug!("1");
        Err(BizError::VaultTransactionInvalid)?;
    }

    let mut total_dist_shards_capacity: u64 = 0;
    let mut total_fee_capacity: u64 = 0;
    let mut uniform_reward_amount: Option<u64> = None;

    let output_count = QueryIter::new(load_cell, Source::Output).count();
    if output_count < 2 {
        debug!("2");
        Err(BizError::VaultTransactionInvalid)?;
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
                    Err(BizError::DistributionDataInvalid)?;
                }
            } else {
                Err(BizError::DistributionDataInvalid)?;
            }

            total_dist_shards_capacity += output_capacity;

            let shard_data_bytes = load_cell_data(i, Source::Output)?;
            let shard_data = DistributionCellData::from_slice(&shard_data_bytes)
                .map_err(|_| BizError::DistributionDataInvalid)?;

            // Validate data consistency.
            if shard_data.campaign_id().as_bytes() != context.vault_data.campaign_id().as_bytes()
                || shard_data.proof_script_code_hash().as_bytes()
                    != context.vault_data.proof_script_code_hash().as_bytes()
                || shard_data.admin_lock_hash().as_slice() != context.admin_lock_hash.as_ref()
            {
                Err(BizError::DistributionDataInvalid)?;
            }

            let uniform_reward_amount_unpacked: u64 = shard_data.uniform_reward_amount().unpack();
            if uniform_reward_amount_unpacked == 0 {
                Err(BizError::ShardRewardInconsistent)?;
            }

            if let Some(first_amount) = uniform_reward_amount {
                if first_amount != uniform_reward_amount_unpacked {
                    Err(BizError::ShardRewardInconsistent)?;
                }
            } else {
                uniform_reward_amount = Some(uniform_reward_amount_unpacked);
            }
        } else {
            // fee cell
            let output_lock_hash = load_cell_lock_hash(i, Source::Output)?;
            if output_lock_hash == context.admin_lock_hash {
                if total_fee_capacity > 0 {
                    debug!("3");
                    Err(BizError::VaultTransactionInvalid)?;
                }
                total_fee_capacity += output_capacity;
            } else {
                Err(BizError::VaultActionUnknown)?;
            }
        }
    }

    if total_dist_shards_capacity + total_fee_capacity != total_capacity {
        Err(BizError::DistributionCapacityMismatch)?;
    }

    if total_fee_capacity != expected_fee_capacity {
        Err(BizError::FeeCapacityMismatch)?;
    }

    Ok(())
}

fn verify_partial_refund(context: &VmContext) -> Result<(), Error> {
    let output_vault_cell = load_cell(0, Source::GroupOutput)?;
    let output_vault_data_bytes = load_cell_data(0, Source::GroupOutput)?;

    if context.vault_data.as_slice() != output_vault_data_bytes {
        Err(BizError::VaultDataImmutable)?;
    }

    let vault_capaciy_unpacked: u64 = output_vault_cell.capacity().unpack();
    if vault_capaciy_unpacked >= context.vault_capacity {
        Err(BizError::PartialRefundInvalid)?;
    }

    let expected_refund_capacity = context.vault_capacity - vault_capaciy_unpacked;

    let creator_lock_hash: [u8; 32] = context.vault_data.creator_lock_hash().into();

    let refund_cells_found = QueryIter::new(load_cell, Source::Output)
        .filter(|cell| cell.lock().calc_script_hash().as_slice() == creator_lock_hash.as_ref())
        .collect::<alloc::vec::Vec<_>>();

    if refund_cells_found.len() != 1 {
        Err(BizError::PartialRefundInvalid)?;
    }

    let refund_cell = &refund_cells_found[0];
    let refund_capacity: u64 = refund_cell.capacity().unpack();

    if refund_capacity != expected_refund_capacity {
        Err(BizError::DistributionCapacityMismatch)?;
    }

    if refund_cell.type_().to_opt().is_some() {
        Err(BizError::PartialRefundInvalid)?;
    }

    Ok(())
}

fn verify_full_refund(context: &VmContext) -> Result<(), Error> {
    let creator_lock_hash: [u8; 32] = context.vault_data.creator_lock_hash().into();

    let refund_cells_found = QueryIter::new(load_cell, Source::Output)
        .filter(|cell| cell.lock().calc_script_hash().as_slice() == creator_lock_hash.as_ref())
        .collect::<alloc::vec::Vec<_>>();

    if refund_cells_found.len() != 1 {
        Err(BizError::RefundLockHashMismatch)?;
    }

    let refund_cell = &refund_cells_found[0];
    let refund_capacity: u64 = refund_cell.capacity().unpack();

    if refund_capacity != context.vault_capacity {
        Err(BizError::DistributionCapacityMismatch)?;
    }

    if refund_cell.type_().to_opt().is_some() {
        Err(BizError::VaultTransactionInvalid)?;
    }

    Ok(())
}
