# CLI Admin Flows Design

**Date:** 2026-03-14

**Goal**

Close the current gaps between the advertised `sss-token` CLI surface and the actual operator workflows needed to manage an SSS mint entirely from the CLI. The required path is:

- initialize `sss-1` and `sss-2` mints
- assign non-admin roles to other wallets
- create mint and burn requests from the CLI
- list and inspect those requests from the CLI
- approve and execute those requests from the CLI
- manage freeze/thaw, blacklist add/remove, seize, status, and supply from the CLI

**Current Gaps**

- `freeze`, `thaw`, `blacklist`, and `seize` are parsed by Clap but fail at runtime because the CLI exits before using the chain client.
- there is no CLI command to update on-chain roles even though the program and SDK support it.
- there is no CLI command to list lifecycle requests, so an operator cannot discover pending approvals from the CLI.
- `status` and `supply` currently print the same two-line output, which does not make `status` useful.

**Chosen Approach**

Extend the existing Rust CLI with the smallest surface that satisfies the required operator flow and keep the split between:

- backend-backed lifecycle requests for `mint`, `burn`, and `operation ...`
- direct on-chain execution for `init`, `roles`, `freeze`, `thaw`, `pause`, `unpause`, `blacklist`, `seize`, `status`, `supply`, `minters`, and `holders`

This keeps request approval and execution aligned with the backend queue model while using direct chain execution for compliance and governance actions that already exist on-chain.

**CLI Surface**

- `sss-token init --preset sss-1|sss-2`
- `sss-token init --config <path>` and keep `--custom` as a compatibility alias if needed
- `sss-token mint <recipient> <amount>`
- `sss-token burn <amount> [--account <token-account>]`
- `sss-token freeze <token-account>`
- `sss-token thaw <token-account>`
- `sss-token pause`
- `sss-token unpause`
- `sss-token status`
- `sss-token supply`
- `sss-token blacklist add <wallet> --reason "..."`
- `sss-token blacklist remove <wallet>`
- `sss-token seize <token-account> --to <treasury-token-account> [--amount <amount>]`
- `sss-token minters list|add|remove`
- `sss-token holders [--min-balance <amount>]`
- `sss-token audit-log [--action <type>]`
- `sss-token roles get`
- `sss-token roles set [--pauser <wallet>] [--burner <wallet>] [--blacklister <wallet>] [--seizer <wallet>]`
- `sss-token operation list [--status <status>] [--type <type>] [--limit <n>]`
- `sss-token operation get <id>`
- `sss-token operation approve <id>`
- `sss-token operation execute <id>`

**Data Flow**

1. `mint` and `burn` submit lifecycle requests to the API.
2. `operation list/get` read lifecycle requests from the API.
3. `operation approve` transitions requests to `approved`.
4. `operation execute` keeps the current API contract and marks requests ready for workers; local workers or explicit execute remain CLI-driven.
5. Compliance and role commands submit direct program instructions through the chain client using the configured authority keypair.

**Status Output**

`status` should become the human-readable mint summary. It should print:

- mint
- preset inference: `sss-1` or `sss-2`
- name, symbol, decimals, uri
- paused
- authority
- feature flags
- total minted
- total burned
- current supply
- master authority, pauser, burner, blacklister, seizer

`supply` should remain concise and print only:

- mint
- supply

**Testing Strategy**

- CLI parser tests for new commands and flags.
- chain client unit coverage for new instruction builders and account readers.
- backend API integration coverage for lifecycle request listing filters.
- CLI integration test using an in-process mock API for request flows.
- devnet end-to-end check that runs the approved CLI flow against real programs:
  - create mint
  - assign role to another wallet
  - create mint request
  - list and approve request
  - verify supply and status
  - create burn request
  - approve and execute burn
  - for `sss-2`, freeze, thaw, blacklist add/remove, and seize

**Risk Controls**

- role updates that touch `blacklister` or `seizer` must surface the existing `sss-2` restriction clearly.
- `freeze` and `thaw` must target token accounts, not wallet addresses.
- `seize --to` must take a token account, not an owner wallet, to avoid ambiguous ATA derivation.
- `operation list` should be additive at the API layer and must not change existing request transitions.
