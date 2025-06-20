import { OutPoint, Since, Transaction } from "@ckb-ccc/core";
import { getMyScript } from "./ccc-client";
import { adminSigner } from "./dependencies";
import { logTx } from "./utils";
import { DistributionData } from "./type";

export async function reclaimDistribution(outPoint: OutPoint) {
  const adminLock = (await adminSigner.getRecommendedAddressObj()).script;

  const distLockContract = getMyScript("distribution-lock");
  const distTypeContract = getMyScript("distribution-type");

  const distCell = await adminSigner.client.getCellLive(outPoint, true);
  if (!distCell) {
    throw new Error("Distribution cell not found");
  }

  const distData = DistributionData.decode(distCell.outputData);
  const deadline = BigInt(distData.deadline);

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
    ],
    headerDeps: [],
    inputs: [
      {
        previousOutput: distCell.outPoint,
        since: Since.from({
          relative: "absolute",
          metric: "timestamp",
          value: deadline,
        }),
      },
    ],
    outputs: [
      {
        capacity: distCell.cellOutput.capacity,
        lock: adminLock,
      },
    ],
    outputsData: ["0x"],
  });

  await tx.completeFeeBy(adminSigner);
  logTx(tx);

  return tx;
}
