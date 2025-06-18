import { OutPoint, Transaction } from "@ckb-ccc/core";
import { logTx } from "./utils";
import { adminSigner } from "./dependencies";

export async function meltCell(outPoint: OutPoint) {
  const tx = Transaction.from({
    version: 0,
    cellDeps: [],
    headerDeps: [],
    inputs: [
      {
        previousOutput: outPoint,
        since: "0x0",
      },
    ],
    outputs: [],
    outputsData: [],
    witnesses: [],
  });

  await tx.completeFeeBy(adminSigner);
  logTx(tx);

  return tx;
}
