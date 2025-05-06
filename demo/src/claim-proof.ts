import {
  Bytes,
  fixedPointFrom,
  HasherCkb,
  Hex,
  mol,
  Script,
  Transaction,
  WitnessArgs,
} from "@ckb-ccc/core";
import { Checkpoint, ProofData, VerificationWitness } from "./type";
import { MerkleTree } from "./merkle";
import { signer } from ".";
import { logTx } from "./utils";

export async function claimProof(
  lockScript: Script,
  txHash: string
): Promise<Transaction> {
  const entityId = "C-IOK0IfMPLP-auHV1Yc9";
  const campaignId = "CFYILSrG-3-yMK1Hssg0Z";

  const checkpoints = [0, 1, 2, 3, 4, 5].map((i) => {
    return {
      entity_id: mol.Byte32.encode(
        Buffer.from(entityId.padEnd(32, "\0")).toString("hex")
      ),
      campaign_id: mol.Byte32.encode(
        Buffer.from(campaignId.padEnd(32, "\0")).toString("hex")
      ),
      timestamp: mol.Uint64.encode(1747302000000 + i * 1000000),
      nonce: mol.Uint32.encode(i),
    };
  });

  const verificationWitness = VerificationWitness.encode({
    checkpoint_messages: checkpoints,
  });

  const leaves = checkpoints.map((checkpoint) => {
    return Checkpoint.encode({
      entity_id: checkpoint.entity_id,
      campaign_id: checkpoint.campaign_id,
      timestamp: checkpoint.timestamp,
      nonce: checkpoint.nonce,
    });
  });

  const proof = MerkleTree.calculateMerkleRoot(leaves);

  const proofData = ProofData.encode({
    entity_id: mol.Byte32.encode(
      Buffer.from(entityId.padEnd(32, "\0")).toString("hex")
    ),
    campaign_id: mol.Byte32.encode(
      Buffer.from(campaignId.padEnd(32, "\0")).toString("hex")
    ),
    proof: mol.Byte32.encode(proof),
  });

  const proofCellCapacity = calculateProofCellCapacity();

  const typeId = generateTypeId(proofData);
  console.log("typeId", typeId);

  let tx = Transaction.from({
    version: "0x0",
    headerDeps: [],
    outputs: [
      // Proof cell
      {
        capacity: `0x${proofCellCapacity.toString(16)}`,
        lock: lockScript,
        type: {
          codeHash:
            "0x1601f32dade3b0de07b7d19661479b008a34b2ff8cdda6da8e291130ca6579ef",
          hashType: "data1",
          args: typeId,
        },
      },
    ],
    outputsData: [proofData],
    cellDeps: [
      {
        outPoint: {
          txHash:
            "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37",
          index: "0x0",
        },
        depType: "depGroup",
      },
      {
        outPoint: {
          txHash: txHash,
          index: "0x0",
        },
        depType: "code",
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
  logTx(tx);

  return tx;
}

function generateTypeId(proof: Bytes): Hex {
  const hash = new HasherCkb();
  hash.update(proof);
  return hash.digest();
}

function calculateProofCellCapacity(): bigint {
  // Simple capacity calculation:
  // - 8 bytes for cell capacity field
  // - ~100 bytes for lock script
  // - ~60 bytes for type script
  // - Data size (variable)
  const dataSize = 32 + 32 + 32; // ProofData fields
  return fixedPointFrom(8 + 100 + 60 + dataSize); // ~264 bytes
}
