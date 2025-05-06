import path from "path";
import { config } from "dotenv";
config({ path: path.resolve(".env"), override: true });

import yargs from "yargs";
import { hideBin } from "yargs/helpers";
import { SignerCkbPrivateKey } from "@ckb-ccc/core";
import { cccClient } from "./ccc-client";
import { meltCell } from "./melt-cell";
import { claimProof } from "./claim-proof";

export const signer = new SignerCkbPrivateKey(
  cccClient,
  process.env.PRIV_KEY ?? ""
);

export const getAccount = async () => {
  const lock = await signer.getAddressObjSecp256k1();
  return {
    lockScript: lock.script,
    address: lock.toString(),
    pubKey: signer.publicKey,
  };
};

const scriptTxHash =
  "0xf327179e7dedfd4487de4e50eace49fa65b5dd6184aa8df2edc07c8b71a99daf";

(async function () {
  const argv = await yargs(hideBin(process.argv))
    .option("claim-proof", {
      describe: "claim a proof",
    })
    .option("melt-cell", {
      describe: "melt a cell",
    })
    .parse();

  const account = await getAccount();
  if (argv.claimProof) {
    const tx = await claimProof(account.lockScript, scriptTxHash);
    const result = await signer.sendTransaction(tx);
    console.log("Transaction sent:", result);
    console.log("Transaction hash:", tx.hash());
  } else if (argv.meltCell) {
    const tx = await meltCell(scriptTxHash);
    const result = await signer.sendTransaction(tx);
    console.log("Transaction sent:", result);
    console.log("Transaction hash:", tx.hash());
  }
})();
