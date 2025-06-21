import { program } from "commander";
import { adminSigner, creatorSigner, subscriberSigner } from "./dependencies";

import { meltCell } from "./melt-cell";
import { createProof } from "./create-proof";
import { createVault } from "./create-vault";
import { CKB_UNIT, getOutpoint } from "./utils";
import { refundVault } from "./refund-vault";
import { createDistribution } from "./create-distribution";
import { claimDistribution } from "./claim-distribution";
import { reclaimDistribution } from "./reclaim-distribution";

(async function () {
  program
    .command("melt-cell")
    .description("Melt a cell to reclaim its capacity")
    .argument("<outpoint>", "transaction hash and index (e.g., 0x...:0)")
    .action(async (outpointStr) => {
      const tx = await meltCell(getOutpoint(outpointStr));
      const result = await adminSigner.sendTransaction(tx);
      console.log("Transaction sent:", result);
    });

  program
    .command("create-proof")
    .description("Submit a proof cell (as subscriber)")
    .action(async () => {
      const tx = await createProof();
      const result = await subscriberSigner.sendTransaction(tx);
      console.log("Transaction sent:", result);
    });

  program
    .command("create-vault")
    .description(
      "Create a new vault with a specified amount of CKB (as creator)"
    )
    .argument("<amount>", "amount of CKB")
    .action(async (amountStr) => {
      const amount = BigInt(amountStr) * CKB_UNIT;
      const tx = await createVault(amount);
      const result = await creatorSigner.sendTransaction(tx);
      console.log("Transaction sent:", result);
    });

  program
    .command("refund-vault")
    .description("Refund a vault partially or fully (as creator)")
    .argument("<outpoint>", "vault cell outpoint (e.g., 0x...:0)")
    .argument("<amount>", "amount of CKB to refund")
    .action(async (outpointStr, amountStr) => {
      const outpoint = getOutpoint(outpointStr);
      const amount = BigInt(amountStr) * CKB_UNIT;
      const tx = await refundVault(outpoint, amount);
      const result = await creatorSigner.sendTransaction(tx);
      console.log("Transaction sent:", result);
    });

  program
    .command("create-distribution")
    .description("Create distribution shards from a vault (as admin)")
    .argument("<vaultOutpoint>", "vault cell outpoint (e.g., 0x...:0)")
    .argument("<proofOutpoint>", "proof cell outpoint (e.g., 0x...:0)")
    .action(async (vaultOutpointStr, proofOutpointStr) => {
      const vaultOutpoint = getOutpoint(vaultOutpointStr);
      const proofOutpoint = getOutpoint(proofOutpointStr);
      const tx = await createDistribution(vaultOutpoint, proofOutpoint);
      const result = await adminSigner.sendTransaction(tx);
      console.log("Transaction sent:", result);
    });

  program
    .command("claim-distribution")
    .description("Claim rewards from a distribution shard (as subscriber)")
    .argument("<distOutpoint>", "distribution cell outpoint (e.g., 0x...:0)")
    .argument("<proofOutpoint>", "proof cell outpoint (e.g., 0x...:0)")
    .action(async (distOutpointStr, proofOutpointStr) => {
      const distOutpoint = getOutpoint(distOutpointStr);
      const proofOutpoint = getOutpoint(proofOutpointStr);
      const tx = await claimDistribution(distOutpoint, proofOutpoint);
      const result = await subscriberSigner.sendTransaction(tx);
      console.log("Transaction sent:", result);
    });

  program
    .command("reclaim-distribution")
    .description("Reclaim rewards from a distribution shard (as admin)")
    .argument("<outpoint>", "distribution cell outpoint (e.g., 0x...:0)")
    .action(async (outpointStr) => {
      const outPoint = getOutpoint(outpointStr);
      const tx = await reclaimDistribution(outPoint);
      const result = await adminSigner.sendTransaction(tx);
      console.log("Transaction sent:", result);
    });

  program.parse();
})();
