import { Transaction } from "@ckb-ccc/core";
import { VaultData } from "./type";
import { getMyScript } from "./ccc-client";
import { hashStringToByte32, logTx } from "./utils";
import { adminSigner, creatorSigner } from "./dependencies";
import { data } from "./info";

export async function createVault(amount: bigint) {
  const adminLock = (await adminSigner.getRecommendedAddressObj()).script;
  const creatorLock = (await creatorSigner.getRecommendedAddressObj()).script;

  const proofContract = getMyScript("proof-type");
  const distLockContract = getMyScript("distribution-lock");
  const distTypeContract = getMyScript("distribution-type");
  const vaultContract = getMyScript("vault-type");

  const campaignId = hashStringToByte32(data.campaignId);

  const feePercentage = 500; // 5.00%

  const vaultData = VaultData.encode({
    campaign_id: campaignId,
    creator_lock_hash: creatorLock.hash(),
    fee_percentage: feePercentage,
    proof_script_code_hash: proofContract.codeHash,
  });

  const vaultTypeArgs =
    distLockContract.codeHash + distTypeContract.codeHash.slice(2);

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
    outputs: [
      {
        capacity: amount,
        lock: adminLock,
        type: {
          ...vaultContract,
          args: vaultTypeArgs,
        },
      },
    ],
    outputsData: [vaultData],
  });

  await tx.completeFeeBy(creatorSigner);
  logTx(tx);

  return tx;
}
