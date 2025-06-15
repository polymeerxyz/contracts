use ckb_testtool::ckb_types::prelude::*;
use common::{
    base::Byte32,
    schema::{
        distribution::{Byte32Vec, ClaimWitness, DistributionCellData, OutPoint},
        proof::ProofCellData,
        vault::VaultCellData,
    },
};
use molecule::prelude::{Builder, Entity};

pub fn populate_proof_data(subscriber_lock_hash: &Byte32, campaign_id: &Byte32) -> ProofCellData {
    let entity_id = Byte32::from_slice(&[1; 32]).unwrap();
    let proof = Byte32::from_slice(&[3; 32]).unwrap();

    ProofCellData::new_builder()
        .entity_id(entity_id)
        .campaign_id(Byte32::from_slice(campaign_id.as_slice()).unwrap())
        .proof(proof)
        .subscriber_lock_hash(subscriber_lock_hash.clone())
        .build()
}

pub fn populate_vault_data(
    campaign_id: &Byte32,
    creator_lock_hash: &Byte32,
    proof_script_code_hash: &Byte32,
    fee_percentage: u16,
) -> VaultCellData {
    VaultCellData::new_builder()
        .campaign_id(Byte32::from_slice(campaign_id.as_slice()).unwrap())
        .creator_lock_hash(Byte32::from_slice(creator_lock_hash.as_slice()).unwrap())
        .proof_script_code_hash(Byte32::from_slice(proof_script_code_hash.as_slice()).unwrap())
        .fee_percentage(fee_percentage.pack())
        .build()
}

pub fn populate_distribution_data(
    campaign_id: &Byte32,
    proof_script_code_hash: &Byte32,
    merkle_root: &[u8; 32],
    reward_amount: u64,
    shard_id: u32,
) -> DistributionCellData {
    DistributionCellData::new_builder()
        .campaign_id(Byte32::from_slice(campaign_id.as_slice()).unwrap())
        .proof_script_code_hash(Byte32::from_slice(proof_script_code_hash.as_slice()).unwrap())
        .merkle_root(Byte32::from_slice(merkle_root).unwrap())
        .uniform_reward_amount(reward_amount.pack())
        .shard_id(shard_id.pack())
        .build()
}

pub fn populate_claim_witness(
    proof_cell_out_point: &OutPoint,
    subscriber_lock_hash: &Byte32,
    merkle_proof: &[[u8; 32]],
) -> ClaimWitness {
    let proof_vec: Vec<Byte32> = merkle_proof
        .iter()
        .map(|item| Byte32::from_slice(item).unwrap())
        .collect();

    ClaimWitness::new_builder()
        .proof_cell_out_point(OutPoint::from_slice(proof_cell_out_point.as_slice()).unwrap())
        .subscriber_lock_hash(Byte32::from_slice(subscriber_lock_hash.as_slice()).unwrap())
        .merkle_proof(Byte32Vec::new_builder().extend(proof_vec).build())
        .build()
}
