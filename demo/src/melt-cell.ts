import { Transaction } from "@ckb-ccc/core";
import { signer } from ".";
import { logTx } from "./utils";

export async function meltCell(txHash: string) {
  const tx = Transaction.from({
    version: 0,
    cellDeps: [],
    headerDeps: [],
    inputs: [
      {
        previousOutput: {
          txHash: txHash,
          index: "0x0",
        },
        since: "0x0",
      },
    ],
    outputs: [],
    outputsData: [],
    witnesses: [],
  });

  await tx.completeFeeBy(signer);
  logTx(tx);

  return tx;
}
