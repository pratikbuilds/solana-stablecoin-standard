# SSS-1

`SSS-1` is the minimal stablecoin standard profile in this repository.

## Purpose

Use `SSS-1` when an issuer needs:

- controlled minting with minter quotas
- controlled burning
- emergency pause
- token-account freeze and thaw

Use a different profile if transfer-time compliance enforcement is required.

## Required Behavior

An `SSS-1` mint must initialize with:

- `enable_permanent_delegate = false`
- `enable_transfer_hook = false`
- `default_account_frozen = false`

The stablecoin program must create:

- a `StablecoinConfig` PDA
- a `RoleConfig` PDA
- a `MinterQuota` PDA for each approved minter

Default role behavior:

- `master_authority`, `pauser`, and `burner` are set to the initializing authority
- `blacklister` and `seizer` remain unset

## Allowed Operations

- initialize
- mint
- burn
- pause and unpause
- freeze and thaw
- update minter
- update roles for pauser and burner
- transfer authority

## Disallowed Compliance Features

`SSS-1` does not support:

- blacklist add and remove
- seizure
- transfer-hook transfer rejection

Any attempt to enable compliance-only roles or flows must be rejected by the program.

## Operational Model

`SSS-1` assumes compliance is handled offchain or at account onboarding time rather than on every transfer. It is intentionally simpler than `SSS-2` and avoids transfer-hook dependencies.
