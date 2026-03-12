# Solana Stablecoin Standard

Workspace bootstrap for the Superteam Brazil `solana-stablecoin-standard` bounty.

## Structure

This repository mirrors the `solana-vault-standard` monorepo shape so the stablecoin programs, SDK, CLI, backend, and tests can evolve without reorganization:

- `programs/` Anchor programs
- `sdk/` generated bindings, client helpers, and CLI
- `backend/` Rust backend and Carbon indexer crates
- `docs/` design, bootstrap, and submission documentation
- `tests/` TypeScript/integration tests
- `trident-tests/` trident-specific coverage
- `modules/` shared specs and future reusable modules
- `scripts/` build and bootstrap scripts

## Bootstrap

```bash
yarn install
yarn build
yarn verify
```

`yarn build` runs the TypeScript build, Rust workspace check, and Anchor build in one pipeline.

## Devnet Integration Tests

The Devnet suite uses Mocha + TypeScript and targets the currently deployed workspace programs.

Before running the tests:

1. Fund `~/.config/solana/id.json` on Devnet.
2. Deploy the latest programs and refresh the fixture file:

```bash
yarn test:devnet:deploy
```

Then run the integration suite:

```bash
yarn test:devnet
```

Run a focused preset flow:

```bash
yarn test:devnet --grep "SSS-1 devnet flow"
yarn test:devnet --grep "SSS-2 devnet flow"
```

Run the bounded stress suite:

```bash
DEVNET_STRESS_ITERATIONS=2 yarn test:devnet:stress
```

## Current status

Phase 1 is a functional workspace:

- buildable placeholder Anchor programs
- buildable Rust backend crates
- buildable TypeScript SDK and CLI packages
- root scripts that verify the whole workspace

Phase 2 will fill in the real architecture:

- `programs/stablecoin` as the configurable core program
- `programs/transfer-hook` as the SSS-2 enforcement program
- `SSS-1` and `SSS-2` implemented as presets, not separate main programs
