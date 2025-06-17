use ckb_std::{
    ckb_constants::Source,
    ckb_types::{packed::Script, prelude::Unpack},
    high_level::{load_cell, load_cell_data, load_script, load_witness_args},
};
use common::schema::distribution::{ClaimWitness, DistributionCellData};
use molecule::prelude::Entity;

use crate::error::{BizError, Error};

pub struct VmContext {
    pub claim_witness: ClaimWitness,
    pub dist_capacity: u64,
    pub dist_data: DistributionCellData,
    pub script: Script,
}

pub fn load_context() -> Result<VmContext, Error> {
    let script = load_script()?;

    let dist_input = load_cell(0, Source::GroupInput)?;
    let dist_data_bytes = load_cell_data(0, Source::GroupInput)?;
    let dist_data = DistributionCellData::from_slice(&dist_data_bytes)
        .map_err(|_| BizError::InvalidDistributionData)?;

    let witness_args = load_witness_args(0, Source::GroupInput)?;
    let witness_args_bytes = witness_args
        .lock()
        .to_opt()
        .ok_or(BizError::InvalidWitnessData)?
        .raw_data();
    let claim_witness =
        ClaimWitness::from_slice(&witness_args_bytes).map_err(|_| BizError::InvalidWitnessData)?;

    Ok(VmContext {
        claim_witness,
        dist_capacity: dist_input.capacity().unpack(),
        dist_data,
        script,
    })
}
