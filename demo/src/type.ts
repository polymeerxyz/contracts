import { mol } from "@ckb-ccc/core";

export const DistributionData = mol.struct({
  campaign_id: mol.Byte32,
  admin_lock_hash: mol.Byte32,
  merkle_root: mol.Byte32,
  proof_script_code_hash: mol.Byte32,
  uniform_reward_amount: mol.Uint64,
  deadline: mol.Uint64,
});

export const OutPoint = mol.struct({
  tx_hash: mol.Byte32,
  index: mol.Uint32,
});

export const ClaimWitness = mol.table({
  merkle_proof: mol.Byte32Vec,
  subscriber_lock_hash: mol.Byte32,
  proof_cell_out_point: OutPoint,
});

export const ProofData = mol.struct({
  entity_id: mol.Byte32,
  campaign_id: mol.Byte32,
  proof: mol.Byte32,
  subscriber_lock_hash: mol.Byte32,
});

export const VaultData = mol.struct({
  campaign_id: mol.Byte32,
  creator_lock_hash: mol.Byte32,
  fee_percentage: mol.Uint16,
  proof_script_code_hash: mol.Byte32,
});
