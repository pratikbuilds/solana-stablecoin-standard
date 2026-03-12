# Devnet Integration Tests Design

## Goal

Add a TypeScript integration test system that exercises the deployed Devnet programs end to end for both presets:

- `SSS-1`: mint -> transfer -> freeze
- `SSS-2`: mint -> transfer -> blacklist -> seize

The same workstream also adds preset configuration assertions, a bounded Devnet stress suite, and a separate Trident fuzzing surface for on-chain invariants.

## Constraints

- Tests run only on Devnet.
- Every integration test creates a fresh mint and fresh stablecoin config on-chain.
- The deployed `stablecoin` and `transfer-hook` programs must exist before the TypeScript suite runs.
- The repository does not currently contain a TypeScript integration harness, only Rust LiteSVM tests and placeholder SDK/client packages.
- The current handwritten TS client is too thin to support these flows directly, so the test harness must include its own operational helpers first.

## Recommended Architecture

Use Mocha for test structure and reporting, with `tsx` as the TypeScript runtime loader.

Keep the core test logic in a reusable Devnet harness rather than in the test runner itself. The harness will own:

- RPC connection to Devnet
- wallet loading
- fresh authority/user keypair generation
- transaction send/confirm/retry helpers
- program ID loading from a deploy output file or env vars
- fresh preset initialization helpers
- ATA creation and Token-2022 account fetch helpers
- stablecoin config, role, blacklist, and mint state assertions

Mocha then stays thin and scenario-focused.

## Deployment Model

Deployment is part of the test system, not a manual prerequisite hidden in README text.

Add a Devnet deployment command that:

1. builds both programs
2. deploys `stablecoin`
3. deploys `transfer-hook`
4. writes the resulting program IDs to a machine-readable config file used by the TS tests

The TypeScript suite should fail fast if the program ID config is missing or incomplete.

## Test Surface

### Preset Config Coverage

`preset-config.spec.ts` verifies:

- SSS-1 config flags
- SSS-2 config flags
- role defaults per preset
- Token-2022 mint authority / freeze authority
- permanent delegate and transfer-hook extensions for SSS-2
- default frozen account state when required by preset
- config pause/audit fields after initialization

### SSS-1 Integration Flow

`sss1.spec.ts` provisions a fresh SSS-1 mint and verifies:

1. initialize preset
2. create token accounts
3. enable minter and mint tokens
4. transfer tokens between users
5. freeze a token account
6. assert balances, frozen state, and config counters

### SSS-2 Integration Flow

`sss2.spec.ts` provisions a fresh SSS-2 mint and verifies:

1. initialize preset
2. create token accounts
3. enable minter and mint tokens
4. perform a normal transfer before compliance actions
5. blacklist the target wallet
6. freeze the blacklisted token account
7. seize from the frozen blacklisted account into treasury
8. assert blacklist PDA, balances, frozen state transitions, and treasury receipt

### Devnet Stress Coverage

`stress.spec.ts` performs bounded repeated example operations on Devnet:

- repeated preset initialization with fresh mints
- repeated minting and transfers
- repeated blacklist + seize cycles for SSS-2

This suite is intended as a soak/regression check for Devnet confirmation and instruction sequencing, not as an unbounded performance benchmark.

## Trident Fuzzing Scope

Keep Trident separate from the Devnet Mocha suite.

Trident should cover:

- preset initialization parameter invariants
- role mutation invariants
- pause + mint/burn/freeze/seize gate checks
- blacklist and seize account-shape invariants
- malformed account ordering and duplicate account paths where applicable

The Devnet suite proves deployed integration. Trident proves on-chain robustness under adversarial input.

## File Layout

- `scripts/deploy-devnet.sh`
- `tests/devnet/tsconfig.json`
- `tests/devnet/mocha.setup.ts`
- `tests/devnet/config.ts`
- `tests/devnet/helpers/cluster.ts`
- `tests/devnet/helpers/wallet.ts`
- `tests/devnet/helpers/transactions.ts`
- `tests/devnet/helpers/presets.ts`
- `tests/devnet/helpers/token2022.ts`
- `tests/devnet/helpers/assertions.ts`
- `tests/devnet/fixtures/program-ids.json`
- `tests/devnet/sss1.spec.ts`
- `tests/devnet/sss2.spec.ts`
- `tests/devnet/preset-config.spec.ts`
- `tests/devnet/stress.spec.ts`
- `trident-tests/` fuzz scaffold and README updates

## Command Surface

The intended top-level commands are:

- `yarn test:devnet:deploy`
- `yarn test:devnet`
- `yarn test:devnet --grep "SSS-1"`
- `yarn test:devnet:stress`
- `cargo trident test`

## Operational Risks

- Devnet confirmation latency can make flaky tests if timeouts and retry logic are naive.
- Fresh state per test increases SOL usage and runtime; the suite needs shared funding checks and bounded test counts.
- Program IDs must stay synchronized with the latest deployed binaries or the tests will give misleading failures.
- Because the generated/client TS SDK is still thin, the test helpers may temporarily become the de facto operational client unless a shared abstraction is extracted later.

## Acceptance Criteria

- Fresh Devnet preset instances are created per test run.
- Mocha tests cover the SSS-1 and SSS-2 flows described above.
- Preset config assertions verify mint extensions and account state directly on-chain.
- A repeatable Devnet deploy command exists and feeds the TS suite program IDs.
- Trident fuzz coverage exists for the main preset and compliance invariants.
- Stress coverage runs example operations against the deployed Devnet programs with explicit bounds and timeouts.
