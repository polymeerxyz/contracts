use crate::{
    error::{Error, ValidationError},
    generated::data::CheckpointMessage,
};
use alloc::vec::Vec;
use ckb_hash::new_blake2b;
use molecule::prelude::Entity;

// Verify the final hash matches the Merkle root of timestamps and nonces
pub fn verify_proof(
    checkpoint_messages: &[CheckpointMessage],
    proof: [u8; 32],
) -> Result<(), Error> {
    let merkle_root = calculate_merkle_root(checkpoint_messages)?;

    // Compare with the final hash
    if merkle_root != proof {
        return Err(Error::Validation(ValidationError::ProofNotMatch));
    }

    Ok(())
}

pub fn calculate_merkle_root(checkpoints: &[CheckpointMessage]) -> Result<[u8; 32], Error> {
    // Same implementation as in the verifier contract
    if checkpoints.is_empty() {
        return Err(Error::Validation(ValidationError::CheckpointsNotFound));
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
    use crate::generated::data::{Byte32, CheckpointMessage, CheckpointMessageVec, Uint64};
    use alloc::vec;
    use molecule::prelude::{Builder, Entity};

    #[test]
    fn test_verify_proof() {
        let checkpoint_messages = CheckpointMessageVec::new_builder()
            .push(
                CheckpointMessage::new_builder()
                    .timestamp(Uint64::from_slice(vec![0; 8].as_slice()).unwrap())
                    .nonce(Byte32::from_slice(vec![0; 32].as_slice()).unwrap())
                    .build(),
            )
            .push(
                CheckpointMessage::new_builder()
                    .timestamp(Uint64::from_slice(vec![0; 8].as_slice()).unwrap())
                    .nonce(Byte32::from_slice(vec![0; 32].as_slice()).unwrap())
                    .build(),
            )
            .build()
            .into_iter()
            .collect::<Vec<_>>();
        let final_hash = [
            63, 108, 60, 89, 122, 252, 174, 167, 198, 247, 124, 216, 141, 76, 205, 221, 2, 114, 1,
            37, 224, 84, 100, 241, 228, 69, 223, 36, 76, 248, 4, 163,
        ];

        // Assuming the checkpoint messages are empty
        let result = verify_proof(checkpoint_messages.as_slice(), final_hash);
        assert!(result.is_ok(), "Expected verification to succeed");
    }
}
