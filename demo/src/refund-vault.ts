import { CellOutput, OutPoint, Transaction } from "@ckb-ccc/core";
import { VaultData } from "./type";
import { getMyScript } from "./ccc-client";
import { logTx } from "./utils";
import { adminSigner, creatorSigner } from "./dependencies";

export async function refundVault(vaultOutPoint: OutPoint, amount: bigint) {
  const creatorLock = (await creatorSigner.getRecommendedAddressObj()).script;

  const vaultContract = getMyScript("vault-type");

  const vaultCell = await adminSigner.client.getCellLive(vaultOutPoint, true);
  if (!vaultCell) {
    throw new Error("Vault cell not found");
  }

  const vaultData = VaultData.decode(vaultCell.outputData);

  if (creatorLock.hash() !== vaultData.creator_lock_hash) {
    throw new Error(
      "Creator lock hash mismatch. The provided creator private key does not match the one in the vault."
    );
  }

  const vaultCapacity = BigInt(vaultCell.cellOutput.capacity);
  if (amount > vaultCapacity) {
    throw new Error("Refund amount cannot be greater than vault capacity.");
  }

  const outputs: Transaction["outputs"] = [];
  const outputsData: Transaction["outputsData"] = [];

  if (amount < vaultCapacity) {
    // Partial refund: create a new vault cell with the remaining capacity.
    outputs.push(
      CellOutput.from({
        capacity: vaultCapacity - amount,
        lock: vaultCell.cellOutput.lock,
        type: vaultCell.cellOutput.type,
      })
    );
    outputsData.push(vaultCell.outputData);
  }
  // If amount === vaultCapacity, this is a full refund, and no new vault cell is created.

  // Add the refund cell for the creator.
  outputs.push(
    CellOutput.from({
      capacity: amount,
      lock: creatorLock,
    })
  );
  outputsData.push("0x");

  const tx = Transaction.from({
    cellDeps: [
      {
        outPoint: vaultContract.cellDeps[0]!.cellDep.outPoint,
        depType: vaultContract.cellDeps[0]!.cellDep.depType,
      },
    ],
    inputs: [
      {
        previousOutput: vaultOutPoint,
        since: "0x0",
      },
    ],
    outputs,
    outputsData,
  });

  await tx.completeFeeBy(adminSigner);
  logTx(tx);

  return tx;
}
