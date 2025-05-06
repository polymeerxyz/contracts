use alloc::vec::Vec;
use common::{
    generated::data::{
        Byte32, CheckpointMessage, CheckpointMessageVec, ProofData, Uint64, VerificationWitness,
    },
    merkle::calculate_merkle_root,
};
use molecule::prelude::{Builder, Entity};

pub fn populate_checkpoint_messages() -> Vec<CheckpointMessage> {
    let checkpoint_messages = [0, 1, 2, 3, 4, 5].map(|i| {
        let timestamp = Uint64::from_slice(vec![i; 8].as_slice()).unwrap();
        let nonce = Byte32::from_slice(vec![i; 32].as_slice()).unwrap();
        CheckpointMessage::new_builder()
            .timestamp(timestamp)
            .nonce(nonce)
            .build()
    });
    Vec::from(checkpoint_messages)
}

pub fn populate_proof_data() -> ProofData {
    let entity_id = Byte32::from_slice(vec![1; 32].as_slice()).unwrap();
    let campaign_id = Byte32::from_slice(vec![2; 32].as_slice()).unwrap();
    let checkpoint_messages = populate_checkpoint_messages();
    let proof = &calculate_merkle_root(&checkpoint_messages).unwrap();

    ProofData::new_builder()
        .entity_id(entity_id)
        .campaign_id(campaign_id)
        .proof(Byte32::from_slice(proof).unwrap())
        .build()
}

pub fn populate_verification_witness() -> VerificationWitness {
    let entity_id = Byte32::from_slice(vec![1; 32].as_slice()).unwrap();
    let campaign_id = Byte32::from_slice(vec![2; 32].as_slice()).unwrap();
    let checkpoint_messages = populate_checkpoint_messages();
    let checkpoint_messages_vec = CheckpointMessageVec::new_builder()
        .extend(checkpoint_messages)
        .build();

    VerificationWitness::new_builder()
        .entity_id(entity_id)
        .campaign_id(campaign_id)
        .checkpoint_messages(checkpoint_messages_vec)
        .build()
}
