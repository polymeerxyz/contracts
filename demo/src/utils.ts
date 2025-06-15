import { mol, CellInput, HasherCkb, Hex, Transaction } from "@ckb-ccc/core";

export function logTx(tx: Transaction) {
  console.log(
    "tx",
    JSON.stringify(tx, (_, v) => (typeof v === "bigint" ? v.toString() : v), 2)
  );
}

export function hashStringToByte32(input: string): Hex {
  const hasher = new HasherCkb();
  hasher.update(Buffer.from(input, "utf-8"));
  const hash = hasher.digest();
  return hash;
}

export function generateTypeId(cellInput: CellInput, index: number): Hex {
  const hash = new HasherCkb();
  hash.update(cellInput.toBytes());
  hash.update(mol.Uint64.encode(index).buffer);
  const result = hash.digest();
  return result;
}
