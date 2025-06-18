import { Transaction } from "@ckb-ccc/core";
import { ProofData } from "./type";
import { logTx, generateTypeId, hashStringToByte32 } from "./utils";
import { getMyScript } from "./ccc-client";
import { subscriberSigner } from "./dependencies";
import { data } from "./info";

export async function createProof(): Promise<Transaction> {
  const lockScript = (await subscriberSigner.getRecommendedAddressObj()).script;

  const entityId = hashStringToByte32(data.entityId);
  const campaignId = hashStringToByte32(data.campaignId);
  const proof = hashStringToByte32(data.proof);

  const proofData = ProofData.encode({
    entity_id: entityId,
    campaign_id: campaignId,
    proof: proof,
    subscriber_lock_hash: lockScript.hash(),
  });

  const proofContract = getMyScript("proof-type");

  const tx = Transaction.from({
    version: "0x0",
    headerDeps: [],
    outputs: [
      // Proof cell
      {
        // no need to add capacity
        // because it is automatically calculated by @ckb-ccc/core
        lock: lockScript,
        type: {
          codeHash: proofContract.codeHash,
          hashType: "data1",
          // placeholder for type id will be replaced later
          args: "0x" + Buffer.from("".padEnd(32, "\0")).toString("hex"),
        },
      },
    ],
    outputsData: [proofData],
    cellDeps: [
      // no need to add secp256k1_blake160_sighash_all cell dep
      // because it is automatically added by @ckb-ccc/core
      {
        outPoint: proofContract.cellDeps[0]!.cellDep.outPoint,
        depType: proofContract.cellDeps[0]!.cellDep.depType,
      },
    ],
    witnesses: ["0x"],
  });

  await tx.completeFeeBy(subscriberSigner);

  const cellInput = tx.inputs[0];
  if (!cellInput) {
    throw new Error("No input found");
  }

  const typeId = generateTypeId(cellInput, 0);
  tx.outputs[0]!.type!.args = typeId;

  logTx(tx);

  return tx;
}
