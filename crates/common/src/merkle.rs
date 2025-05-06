use crate::{error::Error, generated::data::CheckpointMessage};
use alloc::vec::Vec;
use ckb_hash::new_blake2b;
use molecule::prelude::Entity;

// Verify the final hash matches the Merkle root of timestamps and nonces
pub fn verify_proof(checkpoint_messages: &[CheckpointMessage], proof: &[u8]) -> Result<(), Error> {
    let merkle_root = calculate_merkle_root(checkpoint_messages)?;

    // Compare with the final hash
    if merkle_root != proof {
        return Err(Error::InvalidProof);
    }

    Ok(())
}

pub fn calculate_merkle_root(checkpoints: &[CheckpointMessage]) -> Result<[u8; 32], Error> {
    // Same implementation as in the verifier contract
    if checkpoints.is_empty() {
        return Err(Error::InvalidProof);
    }

    // First, hash each checkpoint
    let mut hashes: Vec<[u8; 32]> = Vec::new();
    for checkpoint in checkpoints {
        let mut hasher = new_blake2b();
        hasher.update(&checkpoint.as_bytes());

        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        hashes.push(hash);
    }

    // Then, build the Merkle tree
    while hashes.len() > 1 {
        let mut new_hashes: Vec<[u8; 32]> = Vec::new();

        for i in 0..(hashes.len() + 1) / 2 {
            if i * 2 + 1 < hashes.len() {
                // Hash pair of nodes
                let mut hasher = new_blake2b();
                hasher.update(&hashes[i * 2]);
                hasher.update(&hashes[i * 2 + 1]);

                let mut hash = [0u8; 32];
                hasher.finalize(&mut hash);
                new_hashes.push(hash);
            } else if i * 2 < hashes.len() {
                // Odd number of nodes, duplicate the last one
                new_hashes.push(hashes[i * 2]);
            }
        }

        hashes = new_hashes;
    }

    Ok(hashes[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::data::{Byte32, CheckpointMessage, CheckpointMessageVec, Uint32, Uint64};
    use alloc::vec;
    use molecule::prelude::{Builder, Entity};

    #[test]
    fn test_verify_proof() {
        let checkpoint_messages = CheckpointMessageVec::new_builder()
            .push(
                CheckpointMessage::new_builder()
                    .entity_id(Byte32::from_slice(vec![0; 32].as_slice()).unwrap())
                    .campaign_id(Byte32::from_slice(vec![0; 32].as_slice()).unwrap())
                    .timestamp(Uint64::from_slice(vec![0; 8].as_slice()).unwrap())
                    .nonce(Uint32::from_slice(vec![0; 4].as_slice()).unwrap())
                    .build(),
            )
            .push(
                CheckpointMessage::new_builder()
                    .entity_id(Byte32::from_slice(vec![0; 32].as_slice()).unwrap())
                    .campaign_id(Byte32::from_slice(vec![0; 32].as_slice()).unwrap())
                    .timestamp(Uint64::from_slice(vec![0; 8].as_slice()).unwrap())
                    .nonce(Uint32::from_slice(vec![0; 4].as_slice()).unwrap())
                    .build(),
            )
            .build()
            .into_iter()
            .collect::<Vec<_>>();
        let final_hash = [
            176, 146, 166, 82, 164, 202, 146, 111, 138, 55, 208, 165, 122, 114, 2, 189, 78, 49,
            165, 143, 26, 133, 112, 99, 82, 45, 218, 135, 201, 22, 84, 101,
        ];

        // Assuming the checkpoint messages are empty
        let result = verify_proof(&checkpoint_messages, &final_hash);
        assert!(result.is_ok(), "Expected verification to succeed");
    }
}
