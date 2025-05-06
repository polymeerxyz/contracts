use crate::{
    error::Error,
    generated::data::{ProofData, VerificationWitness},
    merkle::verify_proof,
};
use alloc::vec::Vec;

// Validate proof data against verification witness
pub fn validate_proof_against_witness(
    verification_witness: &VerificationWitness,
    proof_data: &ProofData,
) -> Result<(), Error> {
    // Extract checkpoints and final verification
    let checkpoint_messages = verification_witness
        .checkpoint_messages()
        .into_iter()
        .collect::<Vec<_>>();

    for checkpoint_message in checkpoint_messages.clone() {
        if checkpoint_message.entity_id().raw_data() != proof_data.entity_id().raw_data() {
            return Err(Error::InvalidEntityId);
        }

        if checkpoint_message.campaign_id().raw_data() != proof_data.campaign_id().raw_data() {
            return Err(Error::InvalidCampaignId);
        }
    }

    // Verify the final hash is a valid Merkle root of checkpoints
    verify_proof(&checkpoint_messages, &proof_data.proof().raw_data())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ckb_std::ckb_types::packed::WitnessArgs;
    use faster_hex::hex_decode;
    use molecule::prelude::Entity;

    #[test]
    fn test_decode_verification_witness() {
        let raw_verification_witness = b"d40100000800000006000000512d494f4b3049664d504c502d61754856315963390000000000000000000000594659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d350a58000000000512d494f4b3049664d504c502d61754856315963390000000000000000000000594659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d35fe7c000000001512d494f4b3049664d504c502d61754856315963390000000000000000000000594659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d36f2a0000000002512d494f4b3049664d504c502d61754856315963390000000000000000000000594659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d37e6c4000000003512d494f4b3049664d504c502d61754856315963390000000000000000000000594659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d38dae8000000004512d494f4b3049664d504c502d61754856315963390000000000000000000000594659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d39cf0c000000005";
        let mut dst = [0u8; 468];
        assert!(hex_decode(raw_verification_witness.as_slice(), &mut dst).is_ok());
        let verification_witness = VerificationWitness::from_slice(&dst);
        assert!(verification_witness.is_ok());
    }

    #[test]
    fn test_decode_proof_data() {
        let raw_proof_data = b"512d494f4b3049664d504c502d61754856315963390000000000000000000000594659494c5372472d332d794d4b3148737367305a00000000000000000000001d116e6cbb951ce7724532a83ca6e5369249a1723f97be325e356eb3c86a700c";
        let mut dst = [0u8; 96];
        assert!(hex_decode(raw_proof_data.as_slice(), &mut dst).is_ok());
        let proof_data = ProofData::from_slice(&dst);
        assert!(proof_data.is_ok());
    }

    #[test]
    fn test_validate_proof_against_witness() {
        let raw_proof_data = b"432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a0000000000000000000000e0fec9c1999e9bcd5615deab922872f39116f35432c2417ccbf2de49f95d849a";
        let mut proof_data_dst = [0u8; 96];
        assert!(hex_decode(raw_proof_data.as_slice(), &mut proof_data_dst).is_ok());
        let proof_data = ProofData::from_slice(&proof_data_dst);
        assert!(proof_data.is_ok());

        let raw_witness = b"e80100001000000010000000e8010000d4010000d40100000800000006000000432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d350a58000000000432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d35fe7c000000001432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d36f2a0000000002432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d37e6c4000000003432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d38dae8000000004432d494f4b3049664d504c502d61754856315963390000000000000000000000434659494c5372472d332d794d4b3148737367305a000000000000000000000000000196d39cf0c000000005";
        let mut witness_dst: [u8; 488] = [0u8; 488];
        assert!(hex_decode(raw_witness.as_slice(), &mut witness_dst).is_ok());
        let witness = WitnessArgs::from_slice(&witness_dst);
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
