# Rust Admin CLI Design

**Date:** 2026-03-13

## Goal

Build a Rust-based `sss-token` admin CLI that lets operators initialize and manage stablecoin mints quickly from a single command surface, without exposing transport details like direct on-chain execution versus backend workflow routing.

## Product Direction

`sss-token` is the admin console for the system. Operators provide intent, not transport. The CLI resolves the active profile and routes commands according to policy.

Key rules:

- No user-facing `--mode chain|request` parameter.
- Mutating commands show a confirmation prompt by default.
- `--yes` bypasses confirmation for automation.
- Commands print a clear final result, including operation id or transaction signature.
- `--json` is supported for all commands.

## Command Surface

```bash
sss-token init --preset sss-1 --name "Acme USD" --symbol AUSD --decimals 6 --uri https://example.com/ausd.json
sss-token init --preset sss-2 --name "Acme USD" --symbol AUSD --decimals 6 --uri https://example.com/ausd.json
sss-token init --config config.toml
sss-token init wizard

sss-token mint <recipient> <amount>
sss-token burn <amount>
sss-token freeze <address>
sss-token thaw <address>
sss-token pause
sss-token unpause

sss-token blacklist add <address> --reason "OFAC match"
sss-token blacklist remove <address>
sss-token seize <address> --to <treasury>

sss-token minters list
sss-token minters add <address> --quota <amount>
sss-token minters remove <address>

sss-token holders [--min-balance <amount>] [--limit <n>] [--cursor <cursor>]
sss-token audit-log [--action <type>] [--wallet <address>] [--from <date>] [--to <date>]
sss-token status
sss-token supply
```

## Presets

### SSS-1

Standard operational preset.

- mint / burn / pause / freeze / thaw
- no transfer-hook enforcement
- no permanent delegate requirement
- accounts are not frozen by default

Use when the issuer wants standard admin controls without the stricter compliance posture.

### SSS-2

Compliance-oriented preset.

- blacklist and seize support
- transfer-hook enforcement enabled
- permanent delegate enabled
- accounts frozen by default

Use when the issuer needs stricter compliance controls and transfer-level enforcement.

The CLI must always render the preset description and effective flags during `init --dry-run` and before final confirmation.

## Init Command

`init` is the center of the CLI.

Supported entry paths:

- `sss-token init --preset sss-1 ...`
- `sss-token init --preset sss-2 ...`
- `sss-token init --config config.toml`
- `sss-token init --config config.json`
- `sss-token init wizard`

Required metadata:

- `name`
- `symbol`
- `decimals`
- `uri`

If `--config <path>` is provided and the file does not exist, the CLI should create it after initialization completes using the fully resolved configuration.

The CLI should support `--dry-run` to print the resolved configuration and stop before execution.

Suggested config format:

```toml
name = "Acme USD"
symbol = "AUSD"
decimals = 6
uri = "https://example.com/ausd.json"
preset = "sss-2"

[features]
enable_permanent_delegate = true
enable_transfer_hook = true
default_account_frozen = true

[roles]
mint_authority = "..."
pauser = "..."
burner = "..."
blacklister = "..."
seizer = "..."

[[minters]]
authority = "..."
quota = "1000000000"
active = true
```

## Execution Policy

Execution policy is profile-driven and internal.

Examples:

```toml
[profile.devnet]
cluster = "devnet"
mint = "..."
rpc_url = "https://api.devnet.solana.com"
execution_policy = "direct"

[profile.production]
cluster = "mainnet"
mint = "..."
rpc_url = "https://..."
api_url = "https://..."
execution_policy = "governed"
```

The operator does not type this policy on each command. The CLI decides how to route the request and reports the result clearly.

## Confirmation UX

Mutating commands must show:

- active profile
- cluster
- mint
- action
- target address or account
- amount, if applicable
- reason, if applicable

Then prompt for confirmation. `--yes` skips the prompt.

## Read Commands

`status` should include:

- mint metadata
- preset and effective features
- paused state
- authorities
- transfer-hook status
- total minted
- total burned
- circulating supply
- backend/indexer freshness, when available

`holders` and `audit-log` should be backend-first with pagination and filters.

## Technical Direction

Reuse existing backend routes and the generated Rust stablecoin client where possible. Do not duplicate program logic in the CLI. The CLI should parse arguments, validate shape, resolve defaults, route execution, and present results.
