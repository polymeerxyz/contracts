import { OutPoint } from "@ckb-ccc/core";

export const data = {
  entityId: "5ed61d69-cf14-49af-aead-5f9552cf4e81",
  campaignId: "2bdec373-e4bd-4d65-9963-0ebc0c4b967d",
  proof: "0b07a03b-5c8f-4c06-ad66-96e715bc51be",
};

export const getClaimants = (outPoint: OutPoint, lockHash: string) => {
  return [
    {
      proofOutPoint: {
        txHash: outPoint.txHash,
        index: outPoint.index,
      },
      lockHash: lockHash,
    },
    {
      proofOutPoint: {
        txHash: "0x" + "c".repeat(64),
        index: "0x0",
      },
      lockHash:
        "0x2d10a2c5337463553c2953c929301379594c152140d63034c133066a439c007e",
    },
  ];
};
