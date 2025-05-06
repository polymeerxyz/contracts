import { mol } from "@ckb-ccc/core";

export const Checkpoint = mol.struct({
  entity_id: mol.Byte32,
  campaign_id: mol.Byte32,
  timestamp: mol.Uint64,
  nonce: mol.Uint32,
});

export const CheckpointMessageVec = mol.vector(Checkpoint);

export const VerificationWitness = mol.table({
  checkpoint_messages: CheckpointMessageVec,
});

export const ProofData = mol.struct({
  entity_id: mol.Byte32,
  campaign_id: mol.Byte32,
  proof: mol.Byte32,
});
