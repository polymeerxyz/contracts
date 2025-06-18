import { CellOutput, OutPoint, Transaction, WitnessArgs } from "@ckb-ccc/core";
import { ClaimWitness, DistributionData } from "./type";
import { getMyScript } from "./ccc-client";
import { logTx } from "./utils";
import { buildMerkleProof, hashLeaf } from "./merkle";
import { subscriberSigner } from "./dependencies";
import { getClaimants } from "./info";

export async function claimDistribution(
  distOutPoint: OutPoint,
  proofOutPoint: OutPoint
) {
  const subscriberLock = (await subscriberSigner.getRecommendedAddressObj())
    .script;

  const distLockContract = getMyScript("distribution-lock");
  const distTypeContract = getMyScript("distribution-type");
  const proofContract = getMyScript("proof-type");

  const distCell = await subscriberSigner.client.getCellLive(
    distOutPoint,
    true
  );
  if (!distCell) {
    throw new Error("Distribution cell not found");
  }

  const proofCell = await subscriberSigner.client.getCellLive(
    proofOutPoint,
    true
  );
  if (!proofCell) {
    throw new Error("Proof cell not found");
  }

  const distData = DistributionData.decode(distCell.outputData);
  const distCapacity = BigInt(distCell.cellOutput.capacity);
  const rewardAmount = BigInt(distData.uniform_reward_amount);
  const proofCapacity = BigInt(proofCell.cellOutput.capacity);

  const claimants = getClaimants(proofOutPoint, subscriberLock.hash());

  // Find the claimant's index to generate the correct proof
  const claimantIndex = claimants.findIndex(
    (c) => c.lockHash === subscriberLock.hash()
  );
  if (claimantIndex === -1) {
    throw new Error(
      `Claimant with lock hash ${subscriberLock.hash()} not found in claimants data`
    );
  }

  const leaves = claimants.map((c) =>
    hashLeaf(
      OutPoint.encode(c.proofOutPoint),
      Buffer.from(c.lockHash.slice(2), "hex")
    )
  );
  const merkleProof = buildMerkleProof(leaves, claimantIndex);

  const claimWitness = ClaimWitness.encode({
    merkle_proof: merkleProof.map((p) => "0x" + Buffer.from(p).toString("hex")),
    subscriber_lock_hash: subscriberLock.hash(),
    proof_cell_out_point: {
      tx_hash: proofOutPoint.txHash,
      index: proofOutPoint.index,
    },
  });

  const outputs: Transaction["outputs"] = [];
  const outputsData: Transaction["outputsData"] = [];

  // Check if it's the final claim
  if (distCapacity === rewardAmount) {
    // Final claim, no new distribution cell
  } else {
    // Normal claim, create a new distribution cell with reduced capacity
    outputs.push(
      CellOutput.from({
        ...distCell.cellOutput,
        capacity: distCapacity - rewardAmount,
      })
    );
    outputsData.push(distCell.outputData);
  }

  // Add reward cell
  outputs.push(
    CellOutput.from({
      capacity: rewardAmount + proofCapacity,
      lock: subscriberLock,
    })
  );
  outputsData.push("0x");

  const tx = Transaction.from({
    cellDeps: [
      {
        outPoint: distLockContract.cellDeps[0]!.cellDep.outPoint,
        depType: distLockContract.cellDeps[0]!.cellDep.depType,
      },
      {
        outPoint: distTypeContract.cellDeps[0]!.cellDep.outPoint,
        depType: distTypeContract.cellDeps[0]!.cellDep.depType,
      },
      {
        outPoint: proofContract.cellDeps[0]!.cellDep.outPoint,
        depType: proofContract.cellDeps[0]!.cellDep.depType,
      },
    ],
    inputs: [
      {
        previousOutput: distOutPoint,
        since: "0x0",
      },
      {
        previousOutput: proofOutPoint,
        since: "0x0",
      },
    ],
    outputs,
    outputsData,
    witnesses: [
      WitnessArgs.encode({
        lock: claimWitness,
      }),
    ],
  });

  await tx.completeFeeBy(subscriberSigner);
  logTx(tx);

  return tx;
}
