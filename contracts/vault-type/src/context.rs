use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::Unpack,
    high_level::{load_cell, load_cell_data},
};
use common::schema::vault::VaultCellData;
use molecule::prelude::Entity;

use crate::error::{BizError, Error};

pub struct VmContext {
    pub admin_lock_hash: [u8; 32],
    pub creator_lock_hash: [u8; 32],
    pub vault_capacity: u64,
    pub vault_data: VaultCellData,
}

pub fn load_context() -> Result<VmContext, Error> {
    let input_vault_cell = load_cell(0, Source::GroupInput)?;
    let vault_data_bytes = load_cell_data(0, Source::GroupInput)?;
    let vault_data =
        VaultCellData::from_slice(&vault_data_bytes).map_err(|_| BizError::VaultDataInvalid)?;

    let vault_lock_args = input_vault_cell.lock().args().raw_data();
    if vault_lock_args.len() != 64 {
        Err(BizError::VaultTransactionInvalid)?;
    }

    let admin_lock_hash: [u8; 32] = vault_lock_args[32..64].try_into().unwrap();
    let creator_lock_hash: [u8; 32] = vault_lock_args[0..32].try_into().unwrap();

    Ok(VmContext {
        admin_lock_hash,
        creator_lock_hash,
        vault_data,
        vault_capacity: input_vault_cell.capacity().unpack(),
    })
}
