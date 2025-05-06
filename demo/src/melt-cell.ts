import { Signer, Transaction } from "@ckb-ccc/core";
import { logTx } from "./utils";
import { getMyScript } from "./ccc-client";

export async function meltCell(signer: Signer) {
  const proof = getMyScript("proof");

  const tx = Transaction.from({
    version: 0,
    cellDeps: [],
    headerDeps: [],
    inputs: [
      {
        previousOutput: proof.cellDeps[0]!.cellDep.outPoint,
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
