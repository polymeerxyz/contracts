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

use ckb_hash::new_blake2b;
use ckb_std::{
    ckb_constants::Source,
    high_level::{load_cell_data, load_cell_type, load_script, load_witness_args, QueryIter},
};
use common::{
    generated::data::{ProofData, VerificationWitness},
    validate_proof_against_witness,
};
use molecule::prelude::Entity;
use proof::error::Error;

pub fn program_entry() -> i8 {
    match entry() {
        Ok(()) => 0,
        Err(err) => err as i8,
    }
}

fn entry() -> Result<(), Error> {
    let script = load_script()?;
    let args = script.args().raw_data();

    // Parse the type ID from args
    if args.len() != 32 {
        return Err(Error::InvalidArgs);
    }

    let type_id = &args[0..32];
    let type_hash: &[u8; 32] = &script.calc_script_hash().as_slice().try_into().unwrap();

    // Count cells with our type script
    let inputs_count = count_cells_with_type_hash(type_hash, Source::Input);
    let outputs_count = count_cells_with_type_hash(type_hash, Source::Output);

    if inputs_count == 0 && outputs_count > 0 {
        // CREATION CASE
        let proof_data_output_index = find_output_with_type_hash(type_hash)?;

        let proof_data = load_cell_data(proof_data_output_index, Source::Output)?;
        let proof_data = ProofData::from_slice(&proof_data).map_err(|_| Error::Encoding)?;

        let calculated_type_id = calculate_type_id_from_first_input(&proof_data)?;
        if type_id != calculated_type_id {
            return Err(Error::InvalidTypeId);
        }

        validate_proof_creation(&proof_data)?;

        Ok(())
    } else if inputs_count == 1 && outputs_count == 0 {
        // CONSUMPTION CASE: Anyone can consume
        Ok(())
    } else {
        // TRANSFER CASE: Not allowed
        Err(Error::TransferNotAllowed)
    }
}

// Count cells with the given type script hash
fn count_cells_with_type_hash(type_hash: &[u8; 32], source: Source) -> usize {
    let len = QueryIter::new(load_cell_type, source).count();
    let mut count = 0;

    for i in 0..len {
        if let Ok(Some(cell_type)) = load_cell_type(i, source) {
            if cell_type.calc_script_hash().as_slice() == type_hash {
                count += 1;
            }
        }
    }

    count
}

// Find output cell with the given type script hash
fn find_output_with_type_hash(type_hash: &[u8; 32]) -> Result<usize, Error> {
    let len = QueryIter::new(load_cell_type, Source::Output).count();

    for i in 0..len {
        if let Ok(Some(cell_type)) = load_cell_type(i, Source::Output) {
            if cell_type.calc_script_hash().as_slice() == type_hash {
                return Ok(i);
            }
        }
    }

    Err(Error::CellNotFound)
}

// Validate proof creation against verification witness
fn validate_proof_creation(proof_data: &ProofData) -> Result<(), Error> {
    let len = QueryIter::new(load_witness_args, Source::Input).count();

    for i in 0..len {
        if let Ok(witness_args) = load_witness_args(i, Source::Input) {
            if let Some(input_type) = witness_args.input_type().to_opt() {
                if let Ok(verification_witness) =
                    &VerificationWitness::from_slice(&input_type.raw_data())
                {
                    validate_proof_against_witness(verification_witness, proof_data)?;
                    return Ok(());
                }
            }
        }
    }

    Err(Error::VerificationWitnessNotFound)
}

// Calculate type ID from first input
fn calculate_type_id_from_first_input(proof_data: &ProofData) -> Result<[u8; 32], Error> {
    // Calculate Type ID
    let mut hasher = new_blake2b();
    hasher.update(proof_data.as_slice());

    let mut result = [0u8; 32];
    hasher.finalize(&mut result);
    Ok(result)
}
