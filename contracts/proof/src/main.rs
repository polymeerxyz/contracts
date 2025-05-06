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
    error::SysError,
    high_level::{
        load_cell_data, load_cell_type_hash, load_script_hash, load_witness_args, QueryIter,
    },
};
use common::{
    generated::data::{ProofData, VerificationWitness},
    type_id::{load_type_id_from_script_args, locate_type_id_output_index, validate_type_id},
    validate_proof_against_witness,
};
use molecule::prelude::Entity;
use proof::error::{Error, ProofError};

pub fn program_entry() -> i8 {
    match entry() {
        Ok(()) => 0,
        Err(err) => err.into(),
    }
}

fn entry() -> Result<(), Error> {
    let type_id = load_type_id_from_script_args(0)?;
    let current_script_hash = load_script_hash()?;
    let output_index = locate_type_id_output_index(current_script_hash)?;

    validate_type_id(type_id, output_index)?;

    // Count cells with our type script
    let inputs_count = count_cells_with_type_script(current_script_hash, Source::Input);
    let outputs_count = count_cells_with_type_script(current_script_hash, Source::Output);

    if inputs_count == 0 && outputs_count == 1 {
        // CREATION CASE: Anyone can create one cell with proper data

        validate_proof_creation(output_index)?;
        Ok(())
    } else if inputs_count == 1 && outputs_count == 0 {
        // CONSUMPTION CASE: Anyone can consume

        Ok(())
    } else {
        // TRANSFER CASE: Not allowed

        Err(Error::Proof(ProofError::OperationNotAllowed))
    }
}

fn count_cells_with_type_script(current_script_hash: [u8; 32], source: Source) -> usize {
    QueryIter::new(load_cell_type_hash, source)
        .flatten()
        .filter(|script_hash| script_hash == &current_script_hash)
        .count()
}

fn validate_proof_creation(index: usize) -> Result<(), Error> {
    let cell_ouput_data = load_cell_data(index, Source::Output)?;
    let proof_data =
        &ProofData::from_slice(&cell_ouput_data).map_err(|_| Error::Sys(SysError::Encoding))?;

    let verification_witness = QueryIter::new(load_witness_args, Source::Input)
        .find_map(|witness_args| {
            witness_args
                .input_type()
                .to_opt()
                .and_then(|input_type| VerificationWitness::from_slice(&input_type.raw_data()).ok())
        })
        .ok_or(Error::Proof(ProofError::VerificationWitnessNotFound))?;
    validate_proof_against_witness(&verification_witness, proof_data)?;
    Ok(())
}
