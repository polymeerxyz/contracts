use std::vec::Vec;

use ckb_testtool::{ckb_hash::new_blake2b, ckb_types::packed::CellInput};
use molecule::prelude::Entity;

pub fn calculate_type_id(cell_input: &CellInput, index: usize) -> [u8; 32] {
    let mut blake2b = new_blake2b();
    blake2b.update(cell_input.as_slice());
    blake2b.update(&index.to_le_bytes());

    let mut type_id = [0; 32];
    blake2b.finalize(&mut type_id);
    type_id
}

pub fn blake2b_256<T: AsRef<[u8]>>(s: T) -> [u8; 32] {
    let mut blake2b = new_blake2b();
    blake2b.update(s.as_ref());
    let mut h = [0; 32];
    blake2b.finalize(&mut h);
    h
}

pub fn build_merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0; 32];
    }
    if leaves.len() == 1 {
        return leaves[0];
    }

    let mut current_level = leaves.to_vec();
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        for chunk in current_level.chunks(2) {
            let node1 = chunk[0];
            let node2 = if chunk.len() > 1 { chunk[1] } else { node1 }; // Duplicate if odd

            let mut combined = Vec::new();
            if node1 < node2 {
                combined.extend_from_slice(&node1);
                combined.extend_from_slice(&node2);
            } else {
                combined.extend_from_slice(&node2);
                combined.extend_from_slice(&node1);
            }
            next_level.push(blake2b_256(&combined));
        }
        current_level = next_level;
    }
    current_level[0]
}

pub fn build_merkle_proof(leaves: &[[u8; 32]], leaf_index: usize) -> Vec<[u8; 32]> {
    if leaves.len() <= 1 {
        return vec![];
    }

    let mut proof = Vec::new();
    let mut current_level = leaves.to_vec();
    let mut current_index = leaf_index;

    while current_level.len() > 1 {
        let sibling_index = if current_index % 2 == 0 {
            current_index + 1
        } else {
            current_index - 1
        };

        if sibling_index < current_level.len() {
            proof.push(current_level[sibling_index]);
        } else {
            proof.push(current_level[current_index]);
        }

        let mut next_level = Vec::new();
        for chunk in current_level.chunks(2) {
            let node1 = chunk[0];
            let node2 = if chunk.len() > 1 { chunk[1] } else { node1 };

            let mut combined = Vec::new();
            if node1 < node2 {
                combined.extend_from_slice(&node1);
                combined.extend_from_slice(&node2);
            } else {
                combined.extend_from_slice(&node2);
                combined.extend_from_slice(&node1);
            }
            next_level.push(blake2b_256(&combined));
        }
        current_level = next_level;
        current_index /= 2;
    }
    proof
}
