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
        .campaign_id(campaign_id.clone())
        .proof(proof)
        .subscriber_lock_hash(subscriber_lock_hash.clone())
        .build()
}

pub fn populate_vault_data(
    campaign_id: &Byte32,
    proof_script_code_hash: &Byte32,
    fee_percentage: u16,
) -> VaultCellData {
    VaultCellData::new_builder()
        .campaign_id(campaign_id.clone())
        .proof_script_code_hash(proof_script_code_hash.clone())
        .fee_percentage(fee_percentage.pack())
        .build()
}

pub fn populate_distribution_data(
    campaign_id: &Byte32,
    admin_lock_hash: &Byte32,
    proof_script_code_hash: &Byte32,
    merkle_root: &[u8; 32],
    reward_amount: u64,
    deadline: u64,
) -> DistributionCellData {
    DistributionCellData::new_builder()
        .campaign_id(campaign_id.clone())
        .admin_lock_hash(admin_lock_hash.clone())
        .proof_script_code_hash(proof_script_code_hash.clone())
        .merkle_root(Byte32::from_slice(merkle_root).unwrap())
        .uniform_reward_amount(reward_amount.pack())
        .deadline(deadline.pack())
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
        .proof_cell_out_point(proof_cell_out_point.clone())
        .subscriber_lock_hash(subscriber_lock_hash.clone())
        .merkle_proof(Byte32Vec::new_builder().extend(proof_vec).build())
        .build()
}
