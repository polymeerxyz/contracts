use ckb_hash::new_blake2b;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::packed::CellInput,
    debug,
    error::SysError,
    high_level::{
        load_cell, load_cell_type_hash, load_input, load_script, load_script_hash, QueryIter,
    },
};
use molecule::prelude::Entity;

use crate::error::Error;

fn has_type_id_cell(index: usize, source: Source) -> bool {
    match load_cell(index, source) {
        Ok(_) => true,
        Err(e) => {
            // just confirm cell presence, no data needed
            if let SysError::LengthNotEnough(_) = e {
                return true;
            }
            debug!("load cell err: {:?}", e);
            false
        }
    }
}

fn locate_first_type_id_output_index() -> Result<usize, Error> {
    let current_script_hash = load_script_hash()?;
    QueryIter::new(load_cell_type_hash, Source::Output)
        .flatten()
        .position(|type_hash| type_hash == current_script_hash)
        .ok_or(Error::MissingScriptHash)
}

/// Given a 32-byte type id, this function validates if
/// current transaction confronts to the type ID rules.
pub fn validate_type_id(type_id: [u8; 32]) -> Result<(), Error> {
    if has_type_id_cell(1, Source::GroupInput) || has_type_id_cell(1, Source::GroupOutput) {
        debug!("There can only be at most one input and at most one output type ID cell!");
        return Err(Error::InvalidArgumentCount);
    }

    if !has_type_id_cell(0, Source::GroupInput) {
        // We are creating a new type ID cell here. Additional checkings are needed to ensure the type ID is legit.
        let index = locate_first_type_id_output_index()?;

        // The type ID is calculated as the blake2b (with CKB's personalization) of
        // the first CellInput in current transaction, and the created output cell
        // index(in 64-bit little endian unsigned integer).
        let cell_input = load_input(0, Source::Input)?;
        let calculated_type_id = calculate_type_id(&cell_input, index);

        if calculated_type_id != type_id {
            debug!("Invalid Type ID!");
            return Err(Error::InvalidScriptHash);
        }
    }
    Ok(())
}

/// Loading type ID from current script args, type_id must be at least 32 byte
/// long.
pub fn load_type_id_from_script_args(offset: usize) -> Result<[u8; 32], Error> {
    let script = load_script()?;
    let args = script.args();
    if offset + 32 > args.raw_data().len() {
        debug!("Length of type id is incorrect!");
        return Err(Error::InvalidArgumentLength);
    }
    let mut ret = [0; 32];
    ret.copy_from_slice(&args.raw_data()[offset..offset + 32]);
    Ok(ret)
}

// Calculate the type id of a cell input
pub fn calculate_type_id(cell_input: &CellInput, index: usize) -> [u8; 32] {
    let mut blake2b = new_blake2b();
    blake2b.update(cell_input.as_slice());
    blake2b.update(&index.to_le_bytes());

    let mut type_id = [0; 32];
    blake2b.finalize(&mut type_id);
    type_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use ckb_std::ckb_types::{packed::*, prelude::Builder};
    use molecule::pack_number;

    use crate::utils::{decode_hex, hex_to_vec};

    #[test]
    fn test_generate_type_id() {
        let cell_input = CellInput::new_builder()
            .previous_output(
                OutPoint::new_builder()
                    .tx_hash(
                        decode_hex(
                            "0922239874bc02e3271bd91e36a7d010339b12e9d0624c8f4703219f3baf0e92",
                        )
                        .unwrap(),
                    )
                    .index(Uint32::from_slice(&pack_number(1)).unwrap())
                    .build(),
            )
            .since(Uint64::from_slice(vec![0; 8].as_slice()).unwrap())
            .build();

        let index: usize = 0;
        let type_id = calculate_type_id(&cell_input, index);
        let expected_type_id =
            hex_to_vec("e1348ad4a1a9b38c29ef70e8eb3a723a66f531fccb5a1d3d5489bb68f15d581a");

        assert_eq!(type_id, expected_type_id.as_slice());
    }
}
