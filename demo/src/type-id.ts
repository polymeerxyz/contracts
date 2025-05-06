import { CellInput, HasherCkb, Hex, mol } from "@ckb-ccc/core";

export function generateTypeId(cellInput: CellInput, index: number): Hex {
  const hash = new HasherCkb();
  hash.update(cellInput.toBytes());
  hash.update(mol.Uint64.encode(index).buffer);
  let result = hash.digest();
  return result;
}
