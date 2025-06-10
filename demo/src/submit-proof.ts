import { mol, Script, Signer, Transaction, WitnessArgs } from "@ckb-ccc/core";
import { Checkpoint, ProofData, VerificationWitness } from "./type";
import { MerkleTree } from "./merkle";
import { logTx } from "./utils";
import { getMyScript } from "./ccc-client";
import { generateTypeId } from "./type-id";

export async function submitProof(
  signer: Signer,
  lockScript: Script
): Promise<Transaction> {
  const entityId = "C-IOK0IfMPLP-auHV1Yc9";
  const campaignId = "CFYILSrG-3-yMK1Hssg0Z";

  const checkpoints = [0, 1, 2, 3, 4, 5].map((i) => {
    return {
      timestamp: mol.Uint64.encode(1747302000000 + i * 1000000),
      nonce: mol.Byte32.encode(
        Buffer.from(`${i}`.padEnd(32, "\0")).toString("hex")
      ),
    };
  });

  const verificationWitness = VerificationWitness.encode({
    entity_id: mol.Byte32.encode(
      Buffer.from(entityId.padEnd(32, "\0")).toString("hex")
    ),
    campaign_id: mol.Byte32.encode(
      Buffer.from(campaignId.padEnd(32, "\0")).toString("hex")
    ),
    checkpoint_messages: checkpoints,
  });

  const leaves = checkpoints.map((checkpoint) => {
    return Checkpoint.encode({
      timestamp: checkpoint.timestamp,
      nonce: checkpoint.nonce,
    });
  });

  const merkleRoot = MerkleTree.calculateMerkleRoot(leaves);

  const proofData = ProofData.encode({
    entity_id: mol.Byte32.encode(
      Buffer.from(entityId.padEnd(32, "\0")).toString("hex")
    ),
    campaign_id: mol.Byte32.encode(
      Buffer.from(campaignId.padEnd(32, "\0")).toString("hex")
    ),
    proof: mol.Byte32.encode(merkleRoot),
  });

  const proof = getMyScript("proof");

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
          codeHash: proof.codeHash,
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
        outPoint: proof.cellDeps[0]!.cellDep.outPoint,
        depType: proof.cellDeps[0]!.cellDep.depType,
      },
    ],
    witnesses: [
      // First witness will contain both user signature and verification data
      WitnessArgs.encode({
        inputType: verificationWitness,
      }),
    ],
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
