/**
 * Creates an SSS-1 preset on DevNet and writes mint + authority keypair for backend E2E.
 * Run from repo root after: yarn test:devnet:deploy
 *   npx tsx tests/devnet/create-e2e-credentials.ts
 * Then run the Rust devnet E2E test with env from e2e-credentials.json.
 */

import { writeFileSync } from "node:fs";
import { resolve } from "node:path";

import { createSss1Preset } from "./helpers/presets";
import { workspaceRoot } from "./config";

async function main() {
  const ctx = await createSss1Preset();
  const fixtureDir = resolve(workspaceRoot(), "tests/devnet/fixtures");
  const authorityPath = resolve(fixtureDir, "e2e-authority.json");
  const credentialsPath = resolve(fixtureDir, "e2e-credentials.json");

  writeFileSync(
    authorityPath,
    JSON.stringify(Array.from(ctx.authority.secretKey)),
    "utf8",
  );
  writeFileSync(
    credentialsPath,
    JSON.stringify(
      {
        mint: ctx.mint.publicKey.toBase58(),
        targetAta: ctx.userAAta.toBase58(),
        targetWallet: ctx.userA.publicKey.toBase58(),
        authorityKeypairPath: authorityPath,
      },
      null,
      2,
    ),
    "utf8",
  );
  console.log("Wrote", credentialsPath);
  console.log("Export for Rust E2E:");
  console.log(`  export SSS_DEVNET_MINT=${ctx.mint.publicKey.toBase58()}`);
  console.log(`  export SSS_DEVNET_TARGET_ATA=${ctx.userAAta.toBase58()}`);
  console.log(`  export SSS_AUTHORITY_KEYPAIR=${authorityPath}`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
