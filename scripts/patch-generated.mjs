#!/usr/bin/env node
/**
 * Patches Codama-generated SDK code for known renderer issues.
 * Run after scripts/generate-sdks.mjs.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

// --- generated-kit: transfer-hook naming conflicts ---
const kitTransferHookInstructions = resolve(
  ROOT,
  "sdk/generated-kit/src/transfer-hook/instructions/transferHook.ts",
);
const kitTransferHookPrograms = resolve(
  ROOT,
  "sdk/generated-kit/src/transfer-hook/programs/transferHook.ts",
);

function patchKitTransferHook() {
  let content = readFileSync(kitTransferHookInstructions, "utf8");

  // Rename to avoid conflict with programs' union type and enum
  content = content.replace(
    /export type TransferHookInstruction</g,
    "export type TransferHookInstructionItem<",
  );
  content = content.replace(
    /: TransferHookInstruction</g,
    ": TransferHookInstructionItem<",
  );
  content = content.replace(
    /as TransferHookInstruction</g,
    "as TransferHookInstructionItem<",
  );
  content = content.replace(
    /export type ParsedTransferHookInstruction</g,
    "export type ParsedTransferHookInstructionItem<",
  );
  content = content.replace(
    /export function parseTransferHookInstruction</g,
    "export function parseTransferHookInstructionItem<",
  );
  content = content.replace(
    /: ParsedTransferHookInstruction</g,
    ": ParsedTransferHookInstructionItem<",
  );
  content = content.replace(
    /Promise<\s*TransferHookInstruction</g,
    "Promise<\n  TransferHookInstructionItem<",
  );

  writeFileSync(kitTransferHookInstructions, content);

  content = readFileSync(kitTransferHookPrograms, "utf8");

  // Use renamed imports (instructions exports *Item to avoid conflict)
  content = content.replace(
    `import {
  getInitializeExtraAccountMetaListInstructionAsync,
  getTransferHookInstructionAsync,
  parseInitializeExtraAccountMetaListInstruction,
  parseTransferHookInstruction,
  type InitializeExtraAccountMetaListAsyncInput,
  type ParsedInitializeExtraAccountMetaListInstruction,
  type ParsedTransferHookInstruction,
  type TransferHookAsyncInput,
} from "../instructions";`,
    `import {
  getInitializeExtraAccountMetaListInstructionAsync,
  getTransferHookInstructionAsync,
  parseInitializeExtraAccountMetaListInstruction,
  parseTransferHookInstructionItem,
  type InitializeExtraAccountMetaListAsyncInput,
  type ParsedInitializeExtraAccountMetaListInstruction,
  type ParsedTransferHookInstructionItem,
  type TransferHookAsyncInput,
} from "../instructions";`,
  );

  content = content.replace(
    `  | ({
      instructionType: TransferHookInstruction.TransferHook;
    } & ParsedTransferHookInstruction<TProgram>);`,
    `  | ({
      instructionType: TransferHookInstruction.TransferHook;
    } & ParsedTransferHookInstructionItem<TProgram>);`,
  );

  content = content.replace(
    `    case TransferHookInstruction.TransferHook: {
      assertIsInstructionWithAccounts(instruction);
      return {
        instructionType: TransferHookInstruction.TransferHook,
        ...parseTransferHookInstruction(instruction),
      };
    }`,
    `    case TransferHookInstruction.TransferHook: {
      assertIsInstructionWithAccounts(instruction);
      return {
        instructionType: TransferHookInstruction.TransferHook,
        ...parseTransferHookInstructionItem(instruction),
      };
    }`,
  );

  writeFileSync(kitTransferHookPrograms, content);
}

// --- generated-web3js: minterQuota PDA seeds (authority vs minter) ---
const web3jsMinterQuotaPda = resolve(
  ROOT,
  "sdk/generated-web3js/src/stablecoin/pdas/minterQuota.ts",
);
const web3jsUpdateMinter = resolve(
  ROOT,
  "sdk/generated-web3js/src/stablecoin/instructions/updateMinter.ts",
);
const web3jsMint = resolve(
  ROOT,
  "sdk/generated-web3js/src/stablecoin/instructions/mint.ts",
);

function patchWeb3jsMinterQuota() {
  // MinterQuotaPdaSeeds: use 'minter' (update_minter uses minter account)
  // mint instruction passes authority (which is the minter)
  let content = readFileSync(web3jsMinterQuotaPda, "utf8");
  content = content.replace(
    "export interface MinterQuotaPdaSeeds {\n  mint: PublicKey;\n  authority: PublicKey;\n}",
    "export interface MinterQuotaPdaSeeds {\n  mint: PublicKey;\n  minter: PublicKey;\n}",
  );
  content = content.replace("seeds.authority.toBuffer()", "seeds.minter.toBuffer()");
  writeFileSync(web3jsMinterQuotaPda, content);

  // updateMinter already passes minter - no change needed

  // mint passes authority as the minter
  content = readFileSync(web3jsMint, "utf8");
  content = content.replace(
    `findMinterQuotaPda(
      {
        mint: accounts.mint,
        authority: accounts.authority,
      },
      programId,
    )`,
    `findMinterQuotaPda(
      {
        mint: accounts.mint,
        minter: accounts.authority,
      },
      programId,
    )`,
  );
  writeFileSync(web3jsMint, content);
}

patchKitTransferHook();
patchWeb3jsMinterQuota();
console.log("Patches applied.");
