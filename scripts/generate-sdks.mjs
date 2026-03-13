#!/usr/bin/env node
/**
 * Generates Codama SDKs for Kit and web3.js from Anchor IDLs.
 * Run after `anchor build` to ensure IDLs exist.
 */
import { createFromRoot } from "codama";
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor as renderKitVisitor } from "@codama/renderers-js";
import { renderVisitor as renderRustVisitor } from "@codama/renderers-rust";
import { renderVisitor as renderWeb3jsVisitor } from "@pratikbuilds/web3js-legacy";
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const TARGET_IDL = resolve(ROOT, "target/idl");
const SDK_KIT = resolve(ROOT, "sdk/generated-kit");
const SDK_WEB3JS = resolve(ROOT, "sdk/generated-web3js");
const STABLECOIN_CLIENT_CRATE = resolve(ROOT, "backend/crates/stablecoin_client");

const PROGRAMS = [
  { idl: "stablecoin.json", outDir: "src/stablecoin" },
  { idl: "transfer_hook.json", outDir: "src/transfer-hook" },
];

for (const { idl, outDir } of PROGRAMS) {
  const idlPath = resolve(TARGET_IDL, idl);
  const idlJson = JSON.parse(readFileSync(idlPath, "utf8"));

  const codama = createFromRoot(rootNodeFromAnchor(idlJson));

  codama.accept(
    renderKitVisitor(SDK_KIT, {
      generatedFolder: outDir,
      deleteFolderBeforeRendering: false,
      formatCode: true,
    }),
  );
  console.log(`Generated Kit SDK: ${idl} -> ${outDir}`);

  codama.accept(
    renderWeb3jsVisitor(resolve(SDK_WEB3JS, outDir), {
      packageFolder: SDK_WEB3JS,
      deleteFolderBeforeRendering: false,
      formatCode: true,
    }),
  );
  console.log(`Generated Web3.js SDK: ${idl} -> ${outDir}`);
}

// Generate Rust client for stablecoin only (used by backend API signer).
const stablecoinIdlPath = resolve(TARGET_IDL, "stablecoin.json");
const stablecoinIdl = JSON.parse(readFileSync(stablecoinIdlPath, "utf8"));
const stablecoinCodama = createFromRoot(rootNodeFromAnchor(stablecoinIdl));
stablecoinCodama.accept(
  renderRustVisitor(STABLECOIN_CLIENT_CRATE, {
    generatedFolder: "src/generated",
    deleteFolderBeforeRendering: true,
    formatCode: true,
    syncCargoToml: true,
  }),
);
console.log("Generated Rust client: stablecoin.json -> backend/crates/stablecoin_client");

console.log("SDK generation complete.");
