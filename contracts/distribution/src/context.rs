use ckb_std::{
    ckb_constants::Source,
    ckb_types::packed::Script,
    high_level::{load_cell_data, load_script, load_witness_args},
};
use common::schema::distribution::{ClaimWitness, DistributionCellData};
use molecule::prelude::Entity;

use crate::error::{BizError, Error};

pub struct VmContext {
    pub dist_data: DistributionCellData,
    pub witness: ClaimWitness,
    pub script: Script,
}

pub fn load_context() -> Result<VmContext, Error> {
    // This is a Lock Script, so the cell it's protecting is at index 0 of its group.
    let script = load_script()?;
    let dist_cell_data_bytes = load_cell_data(0, Source::GroupInput)?;
    let witness_bytes = load_witness_args(0, Source::GroupInput)?;

    let dist_data = DistributionCellData::from_slice(&dist_cell_data_bytes)
        .map_err(|_| BizError::InvalidDistributionData)?;
    let witness = ClaimWitness::from_slice(witness_bytes.as_slice())
        .map_err(|_| BizError::InvalidWitnessData)?;

    Ok(VmContext {
        dist_data,
        witness,
        script,
    })
}
