import { Script, Signer, Transaction } from "@ckb-ccc/core";
import { ProofData } from "./type";
import { logTx, generateTypeId, hashStringToByte32 } from "./utils";
import { getMyScript } from "./ccc-client";

export async function submitProof(
  signer: Signer,
  lockScript: Script
): Promise<Transaction> {
  const entityId = hashStringToByte32("5ed61d69-cf14-49af-aead-5f9552cf4e81");
  const campaignId = hashStringToByte32("2bdec373-e4bd-4d65-9963-0ebc0c4b967d");
  const proof = hashStringToByte32("0b07a03b-5c8f-4c06-ad66-96e715bc51be");

  const proofData = ProofData.encode({
    entity_id: entityId,
    campaign_id: campaignId,
    proof: proof,
    subscriber_lock_hash: lockScript.hash(),
  });

  const proofContract = getMyScript("proof");

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

  await tx.completeFeeBy(signer);

  const cellInput = tx.inputs[0];
  if (!cellInput) {
    throw new Error("No input found");
  }

  const typeId = generateTypeId(cellInput, 0);
  tx.outputs[0]!.type!.args = typeId;

  logTx(tx);

  return tx;
}
