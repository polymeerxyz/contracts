import { OutPoint, Script, Transaction } from "@ckb-ccc/core";
import { VaultData, DistributionData } from "./type";
import { getMyScript } from "./ccc-client";
import { logTx } from "./utils";
import { buildMerkleRoot, hashLeaf } from "./merkle";
import { adminSigner, subscriberSigner } from "./dependencies";
import { getClaimants } from "./info";

export async function createDistribution(
  vaultOutPoint: OutPoint,
  proofOutPoint: OutPoint
) {
  const adminLock = (await adminSigner.getRecommendedAddressObj()).script;
  const subscriberLock = (await subscriberSigner.getRecommendedAddressObj())
    .script;

  const vaultContract = getMyScript("vault-type");
  const distLockContract = getMyScript("distribution-lock");
  const distTypeContract = getMyScript("distribution-type");
  const proofContract = getMyScript("proof-type");

  const vaultCell = await adminSigner.client.getCellLive(vaultOutPoint, true);
  if (!vaultCell) {
    throw new Error("Vault cell not found");
  }

  const proofCell = await subscriberSigner.client.getCellLive(
    proofOutPoint,
    true
  );
  if (!proofCell) {
    throw new Error("Proof cell not found");
  }

  const claimants = getClaimants(proofOutPoint, subscriberLock.hash());

  const vaultData = VaultData.decode(vaultCell.outputData);
  const vaultCapacity = BigInt(vaultCell.cellOutput.capacity);

  const feePercentage = BigInt(vaultData.fee_percentage);
  const feeCapacity = (vaultCapacity * feePercentage) / 10000n;
  const totalRewardCapacity = vaultCapacity - feeCapacity;
  const uniformRewardAmount = totalRewardCapacity / BigInt(claimants.length);

  const leaves = claimants.map((c) =>
    hashLeaf(
      OutPoint.encode(c.proofOutPoint),
      Buffer.from(c.lockHash.slice(2), "hex")
    )
  );
  const merkleRoot = buildMerkleRoot(leaves);

  const distData = DistributionData.encode({
    campaign_id: vaultData.campaign_id,
    admin_lock_hash: adminLock.hash(),
    merkle_root: "0x" + Buffer.from(merkleRoot).toString("hex"),
    proof_script_code_hash: proofContract.codeHash,
    uniform_reward_amount: uniformRewardAmount,
    deadline: BigInt(Math.floor(Date.now() / 1000) + 900), // 15 minutes from now, in seconds
  });

  const distShardOutput = {
    capacity: totalRewardCapacity,
    lock: Script.from({ ...distLockContract, args: "" }),
    type: Script.from({ ...distTypeContract, args: "" }),
  };

  const feeOutput = {
    capacity: feeCapacity,
    lock: adminLock,
  };

  const tx = Transaction.from({
    cellDeps: [
      {
        outPoint: vaultContract.cellDeps[0]!.cellDep.outPoint,
        depType: vaultContract.cellDeps[0]!.cellDep.depType,
      },
      {
        outPoint: distLockContract.cellDeps[0]!.cellDep.outPoint,
        depType: distLockContract.cellDeps[0]!.cellDep.depType,
      },
      {
        outPoint: distTypeContract.cellDeps[0]!.cellDep.outPoint,
        depType: distTypeContract.cellDeps[0]!.cellDep.depType,
      },
    ],
    inputs: [
      {
        previousOutput: vaultOutPoint,
        since: "0x0",
      },
    ],
    outputs: [distShardOutput, feeOutput],
    outputsData: [distData, "0x"],
  });

  await tx.completeFeeBy(adminSigner);
  logTx(tx);

  return tx;
}
