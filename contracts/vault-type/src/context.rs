use ckb_std::{
    ckb_constants::Source,
    ckb_types::{packed::Script, prelude::Unpack},
    high_level::{load_cell, load_cell_data, load_cell_lock_hash, load_script},
};
use common::schema::vault::VaultCellData;
use molecule::prelude::Entity;

use crate::error::{BizError, Error};

pub struct VmContext {
    pub admin_lock_hash: [u8; 32],
    pub script: Script,
    pub vault_capacity: u64,
    pub vault_data: VaultCellData,
}

pub fn load_context() -> Result<VmContext, Error> {
    let script = load_script()?;

    let vault_input = load_cell(0, Source::GroupInput)?;
    let vault_data_bytes = load_cell_data(0, Source::GroupInput)?;
    let vault_data =
        VaultCellData::from_slice(&vault_data_bytes).map_err(|_| BizError::DataStructureInvalid)?;

    let admin_lock_hash = load_cell_lock_hash(0, Source::GroupInput)?;

    Ok(VmContext {
        admin_lock_hash,
        script,
        vault_data,
        vault_capacity: vault_input.capacity().unpack(),
    })
}
