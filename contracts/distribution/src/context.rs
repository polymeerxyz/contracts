use ckb_std::{
    ckb_constants::Source,
    ckb_types::{packed::Script, prelude::Unpack},
    high_level::{load_cell, load_cell_data, load_script, load_witness_args},
};
use common::schema::distribution::{ClaimWitness, DistributionCellData};
use molecule::prelude::Entity;

use crate::error::{BizError, Error};

pub struct VmContext {
    pub dist_capacity: u64,
    pub dist_data: DistributionCellData,
    pub script: Script,
    pub witness: ClaimWitness,
}

pub fn load_context() -> Result<VmContext, Error> {
    let script = load_script()?;

    let dist_input = load_cell(0, Source::GroupInput)?;
    let dist_data_bytes = load_cell_data(0, Source::GroupInput)?;
    let dist_data = DistributionCellData::from_slice(&dist_data_bytes)
        .map_err(|_| BizError::InvalidDistributionData)?;

    let witness_bytes = load_witness_args(0, Source::GroupInput)?;
    let witness = ClaimWitness::from_slice(witness_bytes.as_slice())
        .map_err(|_| BizError::InvalidWitnessData)?;

    Ok(VmContext {
        dist_capacity: dist_input.capacity().unpack(),
        dist_data,
        script,
        witness,
    })
}
