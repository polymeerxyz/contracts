use crate::{
    error::{Error, ValidationError},
    generated::data::{ProofData, VerificationWitness},
    merkle::verify_proof,
};
use alloc::vec::Vec;

// Validate proof data against verification witness
pub fn validate_proof_against_witness(
    verification_witness: &VerificationWitness,
    proof_data: &ProofData,
) -> Result<(), Error> {
    if verification_witness.entity_id().raw_data() != proof_data.entity_id().raw_data() {
        return Err(Error::Validation(ValidationError::EntityIDNotMatch));
    }

    if verification_witness.campaign_id().raw_data() != proof_data.campaign_id().raw_data() {
        return Err(Error::Validation(ValidationError::CampaignIDNotMatch));
    }

    // Extract checkpoints and final verification
    let checkpoint_messages = verification_witness
        .checkpoint_messages()
        .into_iter()
        .collect::<Vec<_>>();

    // Verify the final hash is a valid Merkle root of checkpoints
    verify_proof(&checkpoint_messages, proof_data.proof().into())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::decode_hex;
    use ckb_std::ckb_types::packed::WitnessArgs;
    use molecule::prelude::Entity;

    #[test]
    fn test_validate_proof_against_witness() {
        let proof_data = decode_hex::<ProofData>("432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a0000000000000000000000d1537708fc1908fdc451592df768c32d08b1f0ee81be875288008e765f992672");

        let witness = decode_hex::<WitnessArgs>("580100001000000010000000580100004401000044010000100000003000000050000000432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a00000000000000000000000600000000000196d350a580300000000000000000000000000000000000000000000000000000000000000000000196d35fe7c0310000000000000000000000000000000000000000000000000000000000000000000196d36f2a00320000000000000000000000000000000000000000000000000000000000000000000196d37e6c40330000000000000000000000000000000000000000000000000000000000000000000196d38dae80340000000000000000000000000000000000000000000000000000000000000000000196d39cf0c03500000000000000000000000000000000000000000000000000000000000000");
        assert!(witness.is_ok());

        let verification_witness = VerificationWitness::from_slice(
            &witness.unwrap().input_type().to_opt().unwrap().raw_data(),
        );
        assert!(verification_witness.is_ok());

        let result =
            validate_proof_against_witness(&verification_witness.unwrap(), &proof_data.unwrap());
        assert!(result.is_ok());
    }
}
