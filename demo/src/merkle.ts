import { HasherCkb } from "@ckb-ccc/core";

export class MerkleTree {
  constructor() {}

  static calculateMerkleRoot(checkpoints: Uint8Array[]): Uint8Array {
    if (checkpoints.length === 0) {
      throw new Error("No checkpoints provided");
    }

    // Hash each checkpoint
    let hashes: Uint8Array[] = checkpoints.map((cp) => {
      return this.hash(cp);
    });

    // Build Merkle tree
    while (hashes.length > 1) {
      const newHashes: Uint8Array[] = [];

      for (let i = 0; i < Math.ceil(hashes.length / 2); i++) {
        if (i * 2 + 1 < hashes.length) {
          const combined = new Uint8Array([
            ...hashes[i * 2]!,
            ...hashes[i * 2 + 1]!,
          ]);
          newHashes.push(this.hash(combined));
        } else {
          // Odd number of nodes: duplicate the last
          newHashes.push(hashes[i * 2]!);
        }
      }

      hashes = newHashes;
    }

    return hashes[0]!;
  }

  private static hash(data: Uint8Array): Uint8Array {
    const hash = new HasherCkb();
    hash.update(data);
    return Buffer.from(hash.digest().slice(2), "hex");
  }
}
