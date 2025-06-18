import path from "path";
import { config } from "dotenv";
config({ path: path.resolve(".env"), override: true });

import { SignerCkbPrivateKey } from "@ckb-ccc/core";
import { Network } from "../offckb.config";
import { buildCccClient } from "./ccc-client";

const cccClient = buildCccClient((process.env.NETWORK ?? "testnet") as Network);

// Used for managing vaults and distributions
export const adminSigner = new SignerCkbPrivateKey(
  cccClient,
  process.env.ADMIN_PRIV_KEY ?? ""
);

// Used for creating vaults and receiving refunds
export const creatorSigner = new SignerCkbPrivateKey(
  cccClient,
  process.env.CREATOR_PRIV_KEY ?? ""
);

// Used for claiming rewards
export const subscriberSigner = new SignerCkbPrivateKey(
  cccClient,
  process.env.SUBSCRIBER_PRIV_KEY ?? ""
);
