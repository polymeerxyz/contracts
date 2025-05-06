import { mol } from "@ckb-ccc/core";

export const Checkpoint = mol.struct({
  timestamp: mol.Uint64,
  nonce: mol.Byte32,
});

export const CheckpointMessageVec = mol.vector(Checkpoint);

export const VerificationWitness = mol.table({
  entity_id: mol.Byte32,
  campaign_id: mol.Byte32,
  checkpoint_messages: CheckpointMessageVec,
});

export const ProofData = mol.struct({
  entity_id: mol.Byte32,
  campaign_id: mol.Byte32,
  proof: mol.Byte32,
});
