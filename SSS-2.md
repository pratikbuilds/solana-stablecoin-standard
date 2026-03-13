# SSS-2

`SSS-2` is the compliance-oriented stablecoin standard profile in this repository.

## Purpose

Use `SSS-2` when an issuer needs:

- all `SSS-1` controls
- onchain blacklist records
- transfer-time blacklist enforcement
- forced treasury seizure from blacklisted frozen accounts

## Required Behavior

An `SSS-2` mint must initialize with:

- `enable_permanent_delegate = true`
- `enable_transfer_hook = true`
- `default_account_frozen = true` by preset default

The program must reject transfer-hook enablement unless permanent delegate is also enabled.

Default role behavior:

- `master_authority`, `pauser`, `burner`, `blacklister`, and `seizer` are all set to the initializing authority

## Compliance Primitives

### Blacklist

- A blacklist entry is keyed by mint and wallet owner.
- Adding a blacklist entry records authority, reason, and timestamp.
- Removing a blacklist entry closes the PDA and emits an unblacklist event.

### Transfer hook

- Source and destination token-account owners are checked through derived blacklist PDAs.
- If either side is blacklisted, the transfer hook rejects the transfer.
- If neither side is blacklisted, the Token-2022 transfer may continue.

### Seizure

Seizure requires all of the following:

- the mint is not paused
- the source owner has an active blacklist entry
- the source token account is frozen
- the caller is authorized as seizer
- the treasury token account belongs to the current authority

The program thaws the frozen account, transfers the amount with the config PDA as delegate authority, then leaves the source account frozen.

## Required Event Surface

`SSS-2` emits the normal lifecycle events plus:

- `AddressBlacklisted`
- `AddressUnblacklisted`
- `TokensSeized`

Transfer-hook outcomes are synthesized offchain by the indexer into compliance actions such as:

- `transfer_checked`
- `transfer_rejected_source_blacklisted`
- `transfer_rejected_destination_blacklisted`

## Operational Model

`SSS-2` is appropriate where the issuer expects active sanctions workflows, audit export requirements, and transfer-time gating rather than post-facto monitoring.
