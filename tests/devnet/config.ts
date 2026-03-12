import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import type { Stablecoin } from "../../target/types/stablecoin";
import type { TransferHook } from "../../target/types/transfer_hook";

export interface ProgramIds {
  cluster: "devnet";
  stablecoinProgramId: string;
  transferHookProgramId: string;
}

export function workspaceRoot(): string {
  return resolve(__dirname, "../..");
}

function readJsonFile<T>(path: string): T {
  return JSON.parse(readFileSync(path, "utf8")) as T;
}

export function loadProgramIds(): ProgramIds {
  return readJsonFile<ProgramIds>(
    resolve(workspaceRoot(), "tests/devnet/fixtures/program-ids.json"),
  );
}

export function loadStablecoinIdl(): Stablecoin {
  return readJsonFile<Stablecoin>(
    resolve(workspaceRoot(), "target/idl/stablecoin.json"),
  );
}

export function loadTransferHookIdl(): TransferHook {
  return readJsonFile<TransferHook>(
    resolve(workspaceRoot(), "target/idl/transfer_hook.json"),
  );
}
