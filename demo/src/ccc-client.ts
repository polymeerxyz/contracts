import {
  ccc,
  CellDepInfoLike,
  KnownScript,
  Script,
  ScriptInfo,
} from "@ckb-ccc/core";
import offCKB, { Network, SystemScriptName } from "../offckb.config";

const DEVNET_SCRIPTS: Record<
  string,
  Pick<Script, "codeHash" | "hashType"> & { cellDeps: CellDepInfoLike[] }
> = {
  [KnownScript.NervosDao]: offCKB.systemScripts.dao!.script,
  [KnownScript.Secp256k1Blake160]:
    offCKB.systemScripts.secp256k1_blake160_sighash_all!.script,
  [KnownScript.Secp256k1Multisig]:
    offCKB.systemScripts.secp256k1_blake160_multisig_all!.script,
  [KnownScript.AnyoneCanPay]: offCKB.systemScripts.anyone_can_pay!.script,
  [KnownScript.XUdt]: offCKB.systemScripts.xudt!.script,
  [KnownScript.OmniLock]: offCKB.systemScripts.omnilock!.script,
};

export function buildCccClient(network: Network) {
  const client =
    network === "mainnet"
      ? new ccc.ClientPublicMainnet()
      : network === "testnet"
        ? new ccc.ClientPublicTestnet()
        : new ccc.ClientPublicTestnet({
            url: offCKB.rpcUrl,
            scripts: DEVNET_SCRIPTS,
          });

  return client;
}

enum MyScriptName {
  proof = "proof",
}

export function getMyScript(name: keyof typeof MyScriptName): ScriptInfo {
  let script = offCKB.myScripts[name];
  if (!script) {
    throw new Error(`Script ${name} not found`);
  }
  return ScriptInfo.from(script);
}

export function getSystemScript(
  name: keyof typeof SystemScriptName
): ScriptInfo {
  let script = offCKB.systemScripts[name];
  if (!script) {
    throw new Error(`Script ${name} not found`);
  }
  return ScriptInfo.from(script.script);
}
