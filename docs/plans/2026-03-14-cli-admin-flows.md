# CLI Admin Flows Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `sss-token` fully support the required admin and compliance workflows from the CLI, then verify them with automated tests and a real CLI end-to-end pass.

**Architecture:** Keep lifecycle requests backend-backed and keep governance/compliance actions direct-to-chain through the Rust chain client. Extend the API only where discovery is missing (`operation list`) and extend the CLI only where the on-chain program already has support (`roles`, `freeze`, `thaw`, `blacklist`, `seize`, richer `status`).

**Tech Stack:** Rust, Clap, Axum, sqlx/Postgres, Solana RPC, Anchor-generated accounts, existing devnet fixtures.

---

### Task 1: Extend the CLI surface

**Files:**
- Modify: `crates/sss-admin-cli/src/cli.rs`
- Test: `crates/sss-admin-cli/src/cli.rs`

**Step 1: Write the failing parser tests**

Add parser tests for:

- `sss-token roles get`
- `sss-token roles set --burner <wallet>`
- `sss-token operation list --status requested --type mint --limit 5`

**Step 2: Run test to verify it fails**

Run: `cargo test -p sss-admin-cli parses_roles_get_command -- --nocapture`

Expected: fail because the subcommands do not exist yet.

**Step 3: Add the new Clap subcommands and args**

Add:

- `Roles { command: RolesCommand }`
- `RolesCommand::Get(ReadArgs)`
- `RolesCommand::Set(UpdateRolesArgs)`
- `OperationCommand::List(OperationListArgs)`
- `InitArgs.custom` as a compatibility alias mapped to config path if needed

**Step 4: Run parser tests to verify they pass**

Run: `cargo test -p sss-admin-cli`

Expected: parser tests pass.

### Task 2: Add direct chain support for roles, freeze/thaw, blacklist, seize, and status reads

**Files:**
- Modify: `crates/sss-admin-cli/src/chain.rs`
- Modify: `crates/sss-admin-cli/src/lib.rs`
- Test: `crates/sss-admin-cli/src/lib.rs`

**Step 1: Write failing unit tests around command execution helpers**

Add focused tests for:

- `status` output structure
- `roles set` argument handling
- command dispatch no longer bailing on `freeze`, `thaw`, `blacklist`, and `seize`

**Step 2: Run targeted tests to verify failure**

Run: `cargo test -p sss-admin-cli command_dispatch -- --nocapture`

Expected: fail because those runtime branches still bail out.

**Step 3: Implement chain client helpers**

Add:

- read helpers for `StablecoinConfig` and `RoleConfig`
- instruction builders and send helpers for `update_roles`, `freeze_account`, `thaw_account`, `add_to_blacklist`, `remove_from_blacklist`, and `seize`
- optional ATA or owner helpers only when needed by the exact command shape

**Step 4: Wire runtime command execution**

Replace the current `anyhow::bail!` branches in `lib.rs` with direct chain calls and implement rich `status` formatting plus concise `supply`.

**Step 5: Run the CLI crate tests**

Run: `cargo test -p sss-admin-cli`

Expected: all CLI unit and parser tests pass.

### Task 3: Add lifecycle request listing to the API and CLI backend client

**Files:**
- Modify: `backend/crates/domain/src/lib.rs`
- Modify: `backend/crates/db/src/lib.rs`
- Modify: `backend/crates/api/src/dto.rs`
- Modify: `backend/crates/api/src/routes.rs`
- Modify: `crates/sss-admin-cli/src/backend.rs`
- Test: `backend/crates/api/tests/api_integration.rs`

**Step 1: Write the failing API test**

Add an integration test that seeds multiple lifecycle requests, then calls a new list endpoint with status/type filters and asserts the response shape and ordering.

**Step 2: Run the targeted test to verify failure**

Run: `cargo test -p sss-api api_routes_cover_lifecycle_list -- --nocapture`

Expected: fail because the endpoint and DB query do not exist.

**Step 3: Implement the listing path**

Add:

- domain types for list query params and response if needed
- DB query with optional `status`, `type`, `mint`, and `limit`
- API route `GET /v1/operations`
- CLI backend client method `list_operations(...)`
- CLI runtime `operation list`

**Step 4: Run backend tests**

Run: `cargo test -p sss-api`

Expected: API integration tests pass.

### Task 4: Add CLI integration coverage for request flows

**Files:**
- Create or modify: `crates/sss-admin-cli/tests/cli_integration.rs`
- Modify: `crates/sss-admin-cli/Cargo.toml` if test-only deps are needed

**Step 1: Write failing integration tests**

Add tests that:

- submit mint request via CLI
- list requests via CLI
- approve request via CLI
- submit burn request via CLI

Use an in-process HTTP mock server or existing Axum test harness to avoid depending on a live API process for these tests.

**Step 2: Run targeted tests to verify failure**

Run: `cargo test -p sss-admin-cli --test cli_integration -- --nocapture`

Expected: fail until `operation list` and CLI wiring exist.

**Step 3: Implement any missing formatting or client plumbing**

Keep the output stable enough for assertions:

- list should print `request_id`, `type`, `status`, `mint`
- get/approve/execute should keep the same field names

**Step 4: Re-run the integration tests**

Run: `cargo test -p sss-admin-cli --test cli_integration -- --nocapture`

Expected: pass.

### Task 5: Add a reproducible CLI end-to-end script

**Files:**
- Create: `scripts/cli-e2e.sh`
- Modify: `README.md` or `OPERATIONS.md`

**Step 1: Write the script**

Implement a script that:

- ensures program ids and API URL are set
- creates fresh wallets for admin, operator, and recipient
- runs `sss-token init --preset sss-2`
- runs `sss-token roles set ...`
- submits mint request
- lists requests
- approves and executes the request
- checks `status` and `supply`
- submits burn request and approves it
- runs `freeze`, `thaw`, `blacklist add`, `blacklist remove`

**Step 2: Run the script against the configured environment**

Run: `bash scripts/cli-e2e.sh`

Expected: complete successfully with clear step logging.

### Task 6: Verify the real CLI flow in this workspace

**Files:**
- None required unless fixes are discovered

**Step 1: Build the workspace**

Run: `cargo build -p sss-admin-cli`

Expected: success.

**Step 2: Run automated tests**

Run:

- `cargo test -p sss-admin-cli`
- `cargo test -p sss-api`

Expected: success.

**Step 3: Run the end-to-end CLI workflow**

Run the new CLI script or the equivalent direct commands in this order:

- create mint
- set role to another wallet
- create mint request to another wallet
- list requests
- approve and execute request
- create burn request
- approve and execute burn
- freeze/thaw
- blacklist add/remove
- status/supply

**Step 4: Record the exact command transcript and any gaps**

Capture the mint address, request ids, transaction signatures, and any command semantics that needed adjustment.
