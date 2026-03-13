# Architecture

This repository is organized as a layered stablecoin platform: onchain control plane, client SDKs, and an optional backend data plane.

## Layer Model

### Layer 1: Onchain programs

- `programs/stablecoin` is the source of truth for issuer state.
- `programs/transfer-hook` is only active for `SSS-2` mints.
- Token-2022 extensions are configured during initialization and encoded into the mint itself.

Core stablecoin instructions:

- `initialize`
- `mint`
- `burn`
- `pause` and `unpause`
- `freeze_account` and `thaw_account`
- `update_minter`
- `update_roles`
- `transfer_authority`
- `add_to_blacklist`
- `remove_from_blacklist`
- `seize`

Primary program accounts:

- `StablecoinConfig`: mint metadata, flags, counters, pause state, and authority.
- `RoleConfig`: master authority plus pauser, burner, blacklister, and seizer roles.
- `MinterQuota`: per-minter quota and consumption tracking.
- `BlacklistEntry`: per-wallet blacklist record for `SSS-2`.

### Layer 2: SDK and generated clients

- `sdk/generated-kit` and `sdk/generated-web3js` expose generated instruction builders.
- `sdk/client` adds preset logic, PDA derivation, transaction assembly, read helpers, and compliance helpers.
- `sdk/cli` is the entry point for workspace CLI packaging.

### Layer 3: Backend services

- `backend/crates/indexer` decodes chain activity and projects it into Postgres.
- `backend/crates/api` exposes indexed data and operation workflows over HTTP.
- `backend/crates/db` owns SQL schema and persistence.
- `backend/crates/domain` defines transport-safe shared types.

## Data Flows

### Onchain issuance flow

1. A client creates a mint through the SDK.
2. The stablecoin program initializes the mint, config PDA, and role PDA.
3. The program emits `StablecoinInitialized`.
4. The master authority assigns or updates minter quotas.
5. Active minters call `mint`, which checks quota and updates counters.

### `SSS-2` compliance transfer flow

1. The mint is initialized with permanent delegate and transfer-hook enabled.
2. The transfer-hook program derives blacklist PDAs for source and destination owners.
3. If either side is blacklisted, transfer execution fails.
4. If both are clear, transfer proceeds through Token-2022.

### Backend indexing flow

1. The indexer reads blocks from RPC and optional block-subscribe streams.
2. It filters for the stablecoin and transfer-hook program IDs.
3. Stablecoin Anchor events are decoded from logs.
4. Transfer-hook outcomes are synthesized from instruction context and logs.
5. Raw events are written to `chain_events`.
6. Derived tables such as `mints`, `mint_roles`, `minter_quotas`, `blacklist_entries`, and `compliance_actions` are updated.

### Backend operations flow

1. External systems create `operation_requests` through the API.
2. An approver moves the request from `requested` to `approved`.
3. Optional workers pick up approved requests.
4. Signer backends, webhook delivery, and audit export are all driven from Postgres state transitions.

## Security Mode

The repository has two security modes, implemented as presets rather than separate programs.

### `SSS-1`

- No transfer-hook program is attached.
- No blacklist or seizure path is available.
- Permanent delegate is disabled.
- Freeze and pause remain available through the config PDA as mint authority.

### `SSS-2`

- Transfer hook is required and permanent delegate must also be enabled.
- Blacklist and seizure roles are active.
- `seize` requires:
  - a positive amount
  - the mint not being paused
  - permanent delegate enabled
  - caller authorized as seizer
  - a matching blacklist entry
  - a frozen source token account
  - a treasury token account owned by the current authority
- The transfer hook blocks transfers involving blacklisted source or destination owners.

## Trust Boundaries

- Onchain state is authoritative for mint policy and balances.
- The SDK is a convenience layer, not an authority boundary.
- The backend is an observability and orchestration layer. It does not replace onchain authorization checks.
- Operation approval is an offchain governance step; onchain execution must still satisfy program rules.
