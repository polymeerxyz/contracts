import { HasherCkb } from "@ckb-ccc/core";

function hash(data: Uint8Array): Uint8Array {
  const hasher = new HasherCkb();
  hasher.update(data);
  return Buffer.from(hasher.digest().slice(2), "hex");
}

export function buildMerkleRoot(leaves: Uint8Array[]): Uint8Array {
  if (leaves.length === 0) {
    return new Uint8Array(32).fill(0);
  }
  if (leaves.length === 1) {
    return leaves[0]!;
  }

  let currentLevel = [...leaves];
  while (currentLevel.length > 1) {
    const nextLevel: Uint8Array[] = [];
    for (let i = 0; i < currentLevel.length; i += 2) {
      const node1 = currentLevel[i]!;
      const node2 = i + 1 < currentLevel.length ? currentLevel[i + 1]! : node1; // Duplicate if odd

      const combined = new Uint8Array(node1.length + node2.length);
      if (Buffer.from(node1).compare(Buffer.from(node2)) < 0) {
        combined.set(node1, 0);
        combined.set(node2, node1.length);
      } else {
        combined.set(node2, 0);
        combined.set(node1, node2.length);
      }
      nextLevel.push(hash(combined));
    }
    currentLevel = nextLevel;
  }
  return currentLevel[0]!;
}

export function buildMerkleProof(
  leaves: Uint8Array[],
  leafIndex: number
): Uint8Array[] {
  if (leaves.length <= 1) {
    return [];
  }

  const proof: Uint8Array[] = [];
  let currentLevel = [...leaves];
  let currentIndex = leafIndex;

  while (currentLevel.length > 1) {
    const siblingIndex =
      currentIndex % 2 === 0 ? currentIndex + 1 : currentIndex - 1;

    if (siblingIndex < currentLevel.length) {
      proof.push(currentLevel[siblingIndex]!);
    } else {
      // If there's no sibling, the node itself is used as the sibling
      proof.push(currentLevel[currentIndex]!);
    }

    const nextLevel: Uint8Array[] = [];
    for (let i = 0; i < currentLevel.length; i += 2) {
      const node1 = currentLevel[i]!;
      const node2 = i + 1 < currentLevel.length ? currentLevel[i + 1]! : node1;

      const combined = new Uint8Array(node1.length + node2.length);
      if (Buffer.from(node1).compare(Buffer.from(node2)) < 0) {
        combined.set(node1, 0);
        combined.set(node2, node1.length);
      } else {
        combined.set(node2, 0);
        combined.set(node1, node2.length);
      }
      nextLevel.push(hash(combined));
    }
    currentLevel = nextLevel;
    currentIndex = Math.floor(currentIndex / 2);
  }
  return proof;
}

export function hashLeaf(
  outPoint: Uint8Array,
  lockHash: Uint8Array
): Uint8Array {
  const combined = new Uint8Array(outPoint.length + lockHash.length);
  combined.set(outPoint, 0);
  combined.set(lockHash, outPoint.length);
  return hash(combined);
}
