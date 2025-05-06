import path from "path";
import { config } from "dotenv";
config({ path: path.resolve(".env"), override: true });

import yargs from "yargs";
import { hideBin } from "yargs/helpers";
import { SignerCkbPrivateKey } from "@ckb-ccc/core";
import { buildCccClient } from "./ccc-client";
import { meltCell } from "./melt-cell";
import { submitProof } from "./submit-proof";
import { Network } from "../offckb.config";

const cccClient = buildCccClient((process.env.NETWORK ?? "testnet") as Network);

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

(async function () {
  const argv = await yargs(hideBin(process.argv))
    .option("submit-proof", {
      describe: "submit a proof",
    })
    .option("melt-cell", {
      describe: "melt a cell",
    })
    .parse();

  const account = await getAccount();
  if (argv.submitProof) {
    const tx = await submitProof(signer, account.lockScript);
    const result = await signer.sendTransaction(tx);
    console.log("Transaction sent:", result);
  } else if (argv.meltCell) {
    const tx = await meltCell(signer);
    const result = await signer.sendTransaction(tx);
    console.log("Transaction sent:", result);
  }
})();
