import { mol } from "@ckb-ccc/core";

export const ProofData = mol.struct({
  entity_id: mol.Byte32,
  campaign_id: mol.Byte32,
  proof: mol.Byte32,
  subscriber_lock_hash: mol.Byte32,
});
