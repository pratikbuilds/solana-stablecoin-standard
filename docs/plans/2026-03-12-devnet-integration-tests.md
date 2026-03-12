# Devnet Integration Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add deploy-first Devnet TypeScript integration tests for SSS-1 and SSS-2, plus preset-config coverage, a bounded Devnet stress suite, and Trident fuzz scaffolding.

**Architecture:** The implementation uses a shared TypeScript Devnet harness under `tests/devnet/` and runs scenario tests through Mocha with `tsx` as the runtime loader. Program deployment becomes a first-class part of the workflow through a dedicated Devnet deploy script that writes machine-readable program IDs consumed by the test harness.

**Tech Stack:** Anchor, Solana Web3.js, SPL Token 2022, Mocha, tsx, TypeScript, Trident

---

### Task 1: Add Devnet Deploy Command Surface

**Files:**
- Create: `scripts/deploy-devnet.sh`
- Modify: `package.json`
- Modify: `README.md`
- Modify: `tests/README.md`

**Step 1: Write the failing smoke check**

Add a package script that points at a not-yet-created shell script:

```json
{
  "scripts": {
    "test:devnet:deploy": "./scripts/deploy-devnet.sh"
  }
}
```

**Step 2: Run it to verify it fails**

Run: `yarn test:devnet:deploy`
Expected: shell error that `scripts/deploy-devnet.sh` does not exist

**Step 3: Write the minimal deploy script**

Create `scripts/deploy-devnet.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_FILE="$ROOT_DIR/tests/devnet/fixtures/program-ids.json"

cd "$ROOT_DIR"
anchor build
solana config set --url devnet >/dev/null

STABLECOIN_DEPLOY_OUTPUT="$(solana program deploy target/deploy/stablecoin.so --output json)"
TRANSFER_HOOK_DEPLOY_OUTPUT="$(solana program deploy target/deploy/transfer_hook.so --output json)"

mkdir -p "$(dirname "$OUT_FILE")"
cat >"$OUT_FILE" <<EOF
{
  "cluster": "devnet",
  "stablecoinProgramId": $(printf '%s' "$STABLECOIN_DEPLOY_OUTPUT" | jq '.programId'),
  "transferHookProgramId": $(printf '%s' "$TRANSFER_HOOK_DEPLOY_OUTPUT" | jq '.programId')
}
EOF
```

**Step 4: Run it to verify it reaches deployment logic**

Run: `yarn test:devnet:deploy`
Expected: `anchor build` runs, Devnet deploy commands execute, and `tests/devnet/fixtures/program-ids.json` is written

**Step 5: Commit**

```bash
git add package.json README.md tests/README.md scripts/deploy-devnet.sh
git commit -m "test: add devnet deploy workflow"
```

### Task 2: Add Mocha + TS Runtime Dependencies

**Files:**
- Modify: `package.json`
- Create: `tests/devnet/tsconfig.json`
- Create: `tests/devnet/mocha.setup.ts`

**Step 1: Write the failing test runner script**

Add package scripts:

```json
{
  "scripts": {
    "test:devnet": "mocha --require tsx/cjs --file tests/devnet/mocha.setup.ts \"tests/devnet/**/*.spec.ts\"",
    "test:devnet:stress": "mocha --require tsx/cjs --file tests/devnet/mocha.setup.ts \"tests/devnet/stress.spec.ts\""
  }
}
```

**Step 2: Run it to verify it fails**

Run: `yarn test:devnet`
Expected: missing `mocha` or missing setup file error

**Step 3: Add the dependencies and setup**

Update `package.json` dev dependencies:

```json
{
  "devDependencies": {
    "@types/mocha": "^10.0.10",
    "@types/node": "^24.0.0",
    "mocha": "^11.2.2",
    "tsx": "^4.20.3",
    "typescript": "^5.8.3"
  }
}
```

Create `tests/devnet/tsconfig.json`:

```json
{
  "extends": "../../tsconfig.json",
  "compilerOptions": {
    "noEmit": true,
    "types": ["node", "mocha"]
  },
  "include": ["./**/*.ts"]
}
```

Create `tests/devnet/mocha.setup.ts`:

```ts
process.env.SOLANA_CLUSTER ??= "devnet";
process.env.MOCHA_COLORS ??= "1";
```

**Step 4: Run it to verify the runner starts**

Run: `yarn test:devnet`
Expected: Mocha starts and reports `0 passing` or missing spec files, not a loader error

**Step 5: Commit**

```bash
git add package.json tests/devnet/tsconfig.json tests/devnet/mocha.setup.ts
git commit -m "test: add mocha devnet runner"
```

### Task 3: Add Shared Devnet Config And Wallet Helpers

**Files:**
- Create: `tests/devnet/config.ts`
- Create: `tests/devnet/helpers/cluster.ts`
- Create: `tests/devnet/helpers/wallet.ts`
- Create: `tests/devnet/fixtures/.gitkeep`

**Step 1: Write the failing config test**

Create a small spec `tests/devnet/preset-config.spec.ts` with a bootstrap assertion:

```ts
import assert from "node:assert/strict";
import { loadProgramIds } from "./config";

describe("devnet config", () => {
  it("loads deployed program ids", () => {
    const ids = loadProgramIds();
    assert.ok(ids.stablecoinProgramId);
    assert.ok(ids.transferHookProgramId);
  });
});
```

**Step 2: Run it to verify it fails**

Run: `yarn test:devnet --grep "loads deployed program ids"`
Expected: module-not-found or missing file error

**Step 3: Write the minimal helper layer**

Create `tests/devnet/config.ts`:

```ts
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

export interface ProgramIds {
  cluster: "devnet";
  stablecoinProgramId: string;
  transferHookProgramId: string;
}

export function loadProgramIds(): ProgramIds {
  const path = resolve(process.cwd(), "tests/devnet/fixtures/program-ids.json");
  return JSON.parse(readFileSync(path, "utf8")) as ProgramIds;
}
```

Create `tests/devnet/helpers/cluster.ts`:

```ts
import { Connection, clusterApiUrl } from "@solana/web3.js";

export function devnetConnection(): Connection {
  return new Connection(process.env.SOLANA_RPC_URL ?? clusterApiUrl("devnet"), "confirmed");
}
```

Create `tests/devnet/helpers/wallet.ts`:

```ts
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { Keypair } from "@solana/web3.js";

export function loadPayer(): Keypair {
  const keypairPath = process.env.SOLANA_KEYPAIR ?? resolve(process.env.HOME!, ".config/solana/id.json");
  return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(keypairPath, "utf8"))));
}
```

**Step 4: Run it to verify it passes**

Run: `yarn test:devnet --grep "loads deployed program ids"`
Expected: PASS after `tests/devnet/fixtures/program-ids.json` exists

**Step 5: Commit**

```bash
git add tests/devnet/config.ts tests/devnet/helpers/cluster.ts tests/devnet/helpers/wallet.ts tests/devnet/fixtures/.gitkeep tests/devnet/preset-config.spec.ts
git commit -m "test: add devnet config and wallet helpers"
```

### Task 4: Add Transaction And Token-2022 Helpers

**Files:**
- Create: `tests/devnet/helpers/transactions.ts`
- Create: `tests/devnet/helpers/token2022.ts`
- Modify: `tests/devnet/preset-config.spec.ts`

**Step 1: Write the failing helper usage test**

Extend `preset-config.spec.ts`:

```ts
import assert from "node:assert/strict";
import { generateSignerSet } from "./helpers/token2022";

it("creates fresh signer sets per test", () => {
  const { authority, userA, userB } = generateSignerSet();
  assert.notEqual(authority.publicKey.toBase58(), userA.publicKey.toBase58());
  assert.notEqual(userA.publicKey.toBase58(), userB.publicKey.toBase58());
});
```

**Step 2: Run it to verify it fails**

Run: `yarn test:devnet --grep "creates fresh signer sets per test"`
Expected: module-not-found error

**Step 3: Implement the helper layer**

Create `tests/devnet/helpers/transactions.ts`:

```ts
import { Connection, SendOptions, TransactionConfirmationStrategy, VersionedTransaction } from "@solana/web3.js";

export async function sendAndConfirm(
  connection: Connection,
  transaction: VersionedTransaction,
  strategy: TransactionConfirmationStrategy,
  options: SendOptions = {}
): Promise<string> {
  const signature = await connection.sendTransaction(transaction, options);
  await connection.confirmTransaction({ ...strategy, signature }, "confirmed");
  return signature;
}
```

Create `tests/devnet/helpers/token2022.ts`:

```ts
import { Keypair } from "@solana/web3.js";

export function generateSignerSet() {
  return {
    authority: Keypair.generate(),
    treasury: Keypair.generate(),
    userA: Keypair.generate(),
    userB: Keypair.generate(),
  };
}
```

**Step 4: Run it to verify it passes**

Run: `yarn test:devnet --grep "creates fresh signer sets per test"`
Expected: PASS

**Step 5: Commit**

```bash
git add tests/devnet/helpers/transactions.ts tests/devnet/helpers/token2022.ts tests/devnet/preset-config.spec.ts
git commit -m "test: add devnet transaction helpers"
```

### Task 5: Build Fresh Preset Initialization Helpers

**Files:**
- Create: `tests/devnet/helpers/presets.ts`
- Create: `tests/devnet/helpers/assertions.ts`
- Modify: `tests/devnet/preset-config.spec.ts`

**Step 1: Write the failing preset-config test**

Replace the bootstrap-only test with a real preset init assertion:

```ts
it("initializes an SSS-1 preset with expected flags", async function () {
  const ctx = await createSss1Preset();
  assert.equal(ctx.config.paused, false);
  assert.equal(ctx.config.enableTransferHook, false);
});
```

**Step 2: Run it to verify it fails**

Run: `yarn test:devnet --grep "initializes an SSS-1 preset with expected flags"`
Expected: missing helper error

**Step 3: Implement the preset helper skeleton**

Create `tests/devnet/helpers/presets.ts` with typed factories:

```ts
export interface InitializedPreset {
  mint: PublicKey;
  configPda: PublicKey;
  roleConfigPda: PublicKey;
  authority: Keypair;
  treasury: Keypair;
  users: Keypair[];
  config: {
    paused: boolean;
    enableTransferHook: boolean;
    enablePermanentDelegate: boolean;
  };
}

export async function createSss1Preset(): Promise<InitializedPreset> {
  throw new Error("not implemented");
}

export async function createSss2Preset(): Promise<InitializedPreset> {
  throw new Error("not implemented");
}
```

Create `tests/devnet/helpers/assertions.ts`:

```ts
import assert from "node:assert/strict";

export function assertSss1Flags(config: {
  paused: boolean;
  enableTransferHook: boolean;
  enablePermanentDelegate: boolean;
}) {
  assert.equal(config.paused, false);
  assert.equal(config.enableTransferHook, false);
  assert.equal(config.enablePermanentDelegate, false);
}
```

Then implement the minimal happy path for `createSss1Preset()` using the current program instruction surface and Devnet program IDs.

**Step 4: Run it to verify it passes**

Run: `yarn test:devnet --grep "initializes an SSS-1 preset with expected flags"`
Expected: PASS against a fresh Devnet init

**Step 5: Commit**

```bash
git add tests/devnet/helpers/presets.ts tests/devnet/helpers/assertions.ts tests/devnet/preset-config.spec.ts
git commit -m "test: add preset initialization helpers"
```

### Task 6: Finish Preset Config Tests For SSS-1 And SSS-2

**Files:**
- Modify: `tests/devnet/preset-config.spec.ts`
- Modify: `tests/devnet/helpers/assertions.ts`

**Step 1: Write the failing SSS-2 config test**

Add:

```ts
it("initializes an SSS-2 preset with permanent delegate and transfer hook enabled", async function () {
  const ctx = await createSss2Preset();
  assertSss2Flags(ctx.config);
});
```

**Step 2: Run it to verify it fails**

Run: `yarn test:devnet --grep "initializes an SSS-2 preset"`
Expected: not implemented or assertion failure

**Step 3: Implement the remaining assertions**

Extend `tests/devnet/helpers/assertions.ts`:

```ts
export function assertSss2Flags(config: {
  paused: boolean;
  enableTransferHook: boolean;
  enablePermanentDelegate: boolean;
}) {
  assert.equal(config.paused, false);
  assert.equal(config.enableTransferHook, true);
  assert.equal(config.enablePermanentDelegate, true);
}
```

Also add direct on-chain extension assertions for:

- mint authority
- freeze authority
- permanent delegate extension
- transfer hook extension

**Step 4: Run it to verify it passes**

Run: `yarn test:devnet --grep "preset"`
Expected: all preset-config tests PASS

**Step 5: Commit**

```bash
git add tests/devnet/preset-config.spec.ts tests/devnet/helpers/assertions.ts
git commit -m "test: cover preset config invariants on devnet"
```

### Task 7: Implement SSS-1 Integration Scenario

**Files:**
- Create: `tests/devnet/sss1.spec.ts`
- Modify: `tests/devnet/helpers/presets.ts`
- Modify: `tests/devnet/helpers/token2022.ts`

**Step 1: Write the failing SSS-1 scenario**

Create:

```ts
describe("SSS-1 devnet flow", () => {
  it("runs mint -> transfer -> freeze", async function () {
    const ctx = await createSss1Preset();
    await mintToUser(ctx, 1_000_000n);
    await transferBetweenUsers(ctx, 250_000n);
    await freezeUserAccount(ctx);
    await assertSss1FlowResult(ctx);
  });
});
```

**Step 2: Run it to verify it fails**

Run: `yarn test:devnet --grep "SSS-1 devnet flow"`
Expected: missing helper or assertion failure

**Step 3: Implement the minimal scenario helpers**

Add helper functions for:

- ATA creation
- minter role setup
- mint execution
- token transfer
- freeze account
- post-flow balance/state fetches

Use current program semantics, not mocked SDK wrappers.

**Step 4: Run it to verify it passes**

Run: `yarn test:devnet --grep "SSS-1 devnet flow"`
Expected: PASS with fresh on-chain state

**Step 5: Commit**

```bash
git add tests/devnet/sss1.spec.ts tests/devnet/helpers/presets.ts tests/devnet/helpers/token2022.ts
git commit -m "test: add sss1 devnet integration flow"
```

### Task 8: Implement SSS-2 Integration Scenario

**Files:**
- Create: `tests/devnet/sss2.spec.ts`
- Modify: `tests/devnet/helpers/presets.ts`
- Modify: `tests/devnet/helpers/assertions.ts`

**Step 1: Write the failing SSS-2 scenario**

Create:

```ts
describe("SSS-2 devnet flow", () => {
  it("runs mint -> transfer -> blacklist -> seize", async function () {
    const ctx = await createSss2Preset();
    await mintToUser(ctx, 1_000_000n);
    await transferBetweenUsers(ctx, 250_000n);
    await blacklistUser(ctx);
    await freezeBlacklistedAccount(ctx);
    await seizeFromBlacklistedAccount(ctx, 250_000n);
    await assertSss2FlowResult(ctx);
  });
});
```

**Step 2: Run it to verify it fails**

Run: `yarn test:devnet --grep "SSS-2 devnet flow"`
Expected: missing helper or failed instruction

**Step 3: Implement the compliance flow helpers**

Add helper functions for:

- add-to-blacklist
- blacklist PDA fetch/assert
- freeze target account
- seize into treasury ATA
- post-seize balance and account-state assertions

**Step 4: Run it to verify it passes**

Run: `yarn test:devnet --grep "SSS-2 devnet flow"`
Expected: PASS with Devnet signatures confirmed

**Step 5: Commit**

```bash
git add tests/devnet/sss2.spec.ts tests/devnet/helpers/presets.ts tests/devnet/helpers/assertions.ts
git commit -m "test: add sss2 devnet integration flow"
```

### Task 9: Add Bounded Devnet Stress Coverage

**Files:**
- Create: `tests/devnet/stress.spec.ts`
- Modify: `tests/devnet/mocha.setup.ts`
- Modify: `package.json`

**Step 1: Write the failing stress skeleton**

Create:

```ts
describe("Devnet stress", function () {
  this.timeout(10 * 60 * 1000);

  it("runs repeated example operations with explicit bounds", async function () {
    for (let i = 0; i < 3; i += 1) {
      const ctx = await createSss2Preset();
      await mintToUser(ctx, 100_000n);
      await transferBetweenUsers(ctx, 10_000n);
    }
  });
});
```

**Step 2: Run it to verify it fails**

Run: `yarn test:devnet:stress`
Expected: helper failures or timeout tuning needed

**Step 3: Implement explicit stress helpers**

Add:

- signature logging on failure
- retry/backoff for confirmation polling
- optional env bound such as `DEVNET_STRESS_ITERATIONS`

**Step 4: Run it to verify it passes**

Run: `DEVNET_STRESS_ITERATIONS=2 yarn test:devnet:stress`
Expected: PASS within bounded runtime

**Step 5: Commit**

```bash
git add tests/devnet/stress.spec.ts tests/devnet/mocha.setup.ts package.json
git commit -m "test: add bounded devnet stress coverage"
```

### Task 10: Scaffold Trident Fuzz Coverage

**Files:**
- Modify: `trident-tests/README.md`
- Create: `trident-tests/Cargo.toml`
- Create: `trident-tests/fuzz_tests.rs`
- Create: `trident-tests/test_fuzz.rs`

**Step 1: Write the failing Trident command note**

Document the intended command in `trident-tests/README.md`:

```md
cargo trident test
```

**Step 2: Run it to verify the workspace is not ready**

Run: `cargo trident test`
Expected: missing Trident setup or missing targets

**Step 3: Add the minimal Trident scaffold**

Create a basic harness with target buckets for:

- preset initialization inputs
- role updates
- pause gates on mint/burn/seize
- blacklist/seize account invariants

Keep the initial fuzz targets small and compile-first.

**Step 4: Run it to verify the scaffold is wired**

Run: `cargo trident test`
Expected: harness builds and starts fuzz/integration execution

**Step 5: Commit**

```bash
git add trident-tests/README.md trident-tests/Cargo.toml trident-tests/fuzz_tests.rs trident-tests/test_fuzz.rs
git commit -m "test: scaffold trident fuzz coverage"
```

### Task 11: Document The Workflow

**Files:**
- Modify: `README.md`
- Modify: `tests/README.md`
- Modify: `trident-tests/README.md`

**Step 1: Write the failing documentation checklist**

List the missing operator instructions:

- how to deploy to Devnet
- how to run Mocha tests
- required env vars
- expected SOL funding
- how to run stress tests
- how to run Trident fuzzing

**Step 2: Run a manual doc review**

Read: `README.md`, `tests/README.md`, `trident-tests/README.md`
Expected: current docs do not explain the new workflow

**Step 3: Update the docs**

Add exact sections such as:

```md
## Devnet Integration Tests

1. Fund `~/.config/solana/id.json` on Devnet
2. Run `yarn test:devnet:deploy`
3. Run `yarn test:devnet`
4. Run `DEVNET_STRESS_ITERATIONS=2 yarn test:devnet:stress`
```

**Step 4: Verify the docs match reality**

Run:

```bash
yarn test:devnet:deploy
yarn test:devnet --grep "preset"
```

Expected: commands in docs are executable as written

**Step 5: Commit**

```bash
git add README.md tests/README.md trident-tests/README.md
git commit -m "docs: document devnet integration and fuzz workflows"
```
