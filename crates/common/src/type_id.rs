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
        .ok_or(Error::InvalidScriptHashInArgs)
}

/// Given a 32-byte type id, this function validates if
/// current transaction confronts to the type ID rules.
pub fn validate_type_id(type_id: [u8; 32]) -> Result<(), Error> {
    if has_type_id_cell(1, Source::GroupInput) || has_type_id_cell(1, Source::GroupOutput) {
        debug!("There can only be at most one input and at most one output type ID cell!");
        return Err(Error::InvalidArgsNumber);
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
            return Err(Error::InvalidScriptHashInArgs);
        }
    }
    Ok(())
}

/// Loading type ID from current script args, type_id must be at least 32 byte
/// long.
pub fn load_type_id_from_script_args(offset: usize) -> Result<[u8; 32], Error> {
    let script = load_script()?;
    let args = script.args();
    if offset + 32 > args.len() {
        debug!(
            "Type script args length is not 32 bytes, found: {}",
            args.len() - offset
        );
        return Err(Error::InvalidArgsLength);
    }
    let mut ret = [0; 32];
    ret.copy_from_slice(&args.as_slice()[offset..offset + 32]);
    Ok(ret)
}

// Calculate the type id of a cell input
fn calculate_type_id(cell_input: &CellInput, index: usize) -> [u8; 32] {
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
        let expected_type_id =
            hex_to_vec("3faa70b96d101b802b6f8720fb24ff38612f769ac51a50653815c83073fc8b5d");

        let index: usize = 0;
        let cell_input = CellInput::new_builder()
            .previous_output(
                OutPoint::new_builder()
                    .tx_hash(
                        decode_hex(
                            "bd0780bad3363818d9f227aaf5c71c33ed436b03f2f8aade5ff4dfdf5249da65",
                        )
                        .unwrap(),
                    )
                    .index(Uint32::from_slice(&pack_number(1)).unwrap())
                    .build(),
            )
            .since(Uint64::from_slice(vec![0; 8].as_slice()).unwrap())
            .build();

        let type_id = calculate_type_id(&cell_input, index);
        assert_eq!(type_id, expected_type_id.as_slice());
    }
}
