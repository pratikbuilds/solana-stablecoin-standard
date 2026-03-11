# Workspace Bootstrap

This repository starts with the same workspace layers as `solana-vault-standard`.

## Goals

- Keep Anchor, Rust, and TypeScript workspaces at the repo root
- Ensure every workspace member builds from day one
- Prepare the repo for Codama-generated SDKs without locking in program APIs too early

## Layout

- `programs/stablecoin`: core Anchor program placeholder
- `programs/transfer-hook`: transfer hook Anchor program placeholder
- `sdk/generated`: future Codama output package
- `sdk/client`: handwritten TypeScript helpers and preset orchestration
- `sdk/cli`: `sss-token` CLI entry point
- `backend/crates/api`: Rust API placeholder
- `backend/crates/indexer`: Rust Carbon indexer placeholder
- `backend/crates/domain`: shared backend domain types
- `backend/crates/db`: shared database helpers

## Root commands

- `yarn build`: TypeScript build + Rust workspace check + Anchor build
- `yarn verify`: minimal smoke pipeline for the full workspace

## Next implementation milestone

The next phase will keep a two-program topology:

1. `stablecoin` for config, roles, mint lifecycle, pause/freeze, burn, and preset flags
2. `transfer-hook` for SSS-2 compliance enforcement
