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
  const vaultLockContract = getMyScript("vault-lock");
  const vaultTypeContract = getMyScript("vault-type");

  const campaignId = hashStringToByte32(data.campaignId);

  const feePercentage = 500; // 5.00%

  const vaultData = VaultData.encode({
    campaign_id: campaignId,
    fee_percentage: feePercentage,
    proof_script_code_hash: proofContract.codeHash,
  });

  const vaultLockArgs = creatorLock.hash() + adminLock.hash().slice(2);
  const vaultTypeArgs =
    distLockContract.codeHash + distTypeContract.codeHash.slice(2);

  const tx = Transaction.from({
    cellDeps: [
      {
        outPoint: vaultLockContract.cellDeps[0]!.cellDep.outPoint,
        depType: vaultLockContract.cellDeps[0]!.cellDep.depType,
      },
      {
        outPoint: vaultTypeContract.cellDeps[0]!.cellDep.outPoint,
        depType: vaultTypeContract.cellDeps[0]!.cellDep.depType,
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
        lock: {
          ...vaultLockContract,
          args: vaultLockArgs,
        },
        type: {
          ...vaultTypeContract,
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
