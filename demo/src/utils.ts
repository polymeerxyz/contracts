import { Transaction } from "@ckb-ccc/core";

export function logTx(tx: Transaction) {
  console.log(
    "tx",
    JSON.stringify(tx, (_, v) => (typeof v === "bigint" ? v.toString() : v), 2)
  );
}
