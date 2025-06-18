import {
  mol,
  CellInput,
  HasherCkb,
  Hex,
  Transaction,
  OutPoint,
} from "@ckb-ccc/core";

export const CKB_UNIT = 100_000_000n;

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

export function getOutpoint(str: string) {
  const [txHash, index] = str.split(":");
  if (!txHash || !index) {
    throw new Error("Invalid outpoint");
  }

  return OutPoint.from({
    txHash,
    index: `0x${parseInt(index, 10).toString(16)}`,
  });
}
