# Rust Admin CLI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a tested Rust `sss-token` admin CLI that supports initialization, admin operations, backend reads, and operator-safe confirmation flows.

**Architecture:** Add a new Rust CLI crate to the workspace. Keep command parsing, config resolution, routing, and rendering separate so the CLI can route to either direct chain execution or backend workflows without exposing that distinction to operators.

**Tech Stack:** Rust, Clap, Serde, Reqwest, Tokio, Solana SDK, existing `sss_*` backend crates

---

### Task 1: Create the CLI crate skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/sss-admin-cli/Cargo.toml`
- Create: `crates/sss-admin-cli/src/main.rs`
- Create: `crates/sss-admin-cli/src/lib.rs`

**Step 1: Write the failing build setup**

Add the new crate to the workspace and reference a `main.rs` that does not exist yet.

**Step 2: Run workspace check to verify it fails**

Run: `cargo check -p sss-admin-cli`

Expected: FAIL because the crate source files are missing.

**Step 3: Write minimal implementation**

Add the crate files with a minimal `main` function and exported `run` entrypoint.

**Step 4: Run crate check to verify it passes**

Run: `cargo check -p sss-admin-cli`

Expected: PASS

### Task 2: Add command parsing and shared runtime types

**Files:**
- Modify: `crates/sss-admin-cli/Cargo.toml`
- Create: `crates/sss-admin-cli/src/cli.rs`
- Create: `crates/sss-admin-cli/src/context.rs`
- Create: `crates/sss-admin-cli/src/error.rs`
- Modify: `crates/sss-admin-cli/src/lib.rs`

**Step 1: Write the failing parser tests**

Add tests covering:
- `init --preset sss-1 --name ... --symbol ... --decimals 6 --uri ...`
- `mint <recipient> <amount>`
- `blacklist add <address> --reason ...`
- `minters list`

**Step 2: Run parser tests to verify failure**

Run: `cargo test -p sss-admin-cli cli::tests -- --nocapture`

Expected: FAIL because parser structures are not implemented.

**Step 3: Write minimal implementation**

Use Clap derive to define the command tree and basic shared option types.

**Step 4: Run parser tests to verify pass**

Run: `cargo test -p sss-admin-cli cli::tests -- --nocapture`

Expected: PASS

### Task 3: Implement config loading and init config resolution

**Files:**
- Create: `crates/sss-admin-cli/src/config.rs`
- Create: `crates/sss-admin-cli/src/presets.rs`
- Create: `crates/sss-admin-cli/src/init.rs`
- Modify: `crates/sss-admin-cli/src/lib.rs`

**Step 1: Write the failing config tests**

Cover:
- preset resolution for `sss-1`
- preset resolution for `sss-2`
- required `uri`
- loading TOML/JSON config
- creating missing config file after successful init resolution

**Step 2: Run config tests to verify failure**

Run: `cargo test -p sss-admin-cli config::tests init::tests -- --nocapture`

Expected: FAIL because config loaders and serializers are missing.

**Step 3: Write minimal implementation**

Implement config structs, preset application, serialization helpers, and missing-file creation logic for `init`.

**Step 4: Run config tests to verify pass**

Run: `cargo test -p sss-admin-cli config::tests init::tests -- --nocapture`

Expected: PASS

### Task 4: Add confirmation and output rendering

**Files:**
- Create: `crates/sss-admin-cli/src/confirm.rs`
- Create: `crates/sss-admin-cli/src/output.rs`
- Modify: `crates/sss-admin-cli/src/lib.rs`

**Step 1: Write the failing tests**

Cover:
- mutating commands require confirmation by default
- `--yes` bypasses confirmation
- JSON output serialization for success results

**Step 2: Run tests to verify failure**

Run: `cargo test -p sss-admin-cli confirm::tests output::tests -- --nocapture`

Expected: FAIL because the helpers do not exist.

**Step 3: Write minimal implementation**

Add a confirmation abstraction that is testable and an output renderer that supports human and JSON modes.

**Step 4: Run tests to verify pass**

Run: `cargo test -p sss-admin-cli confirm::tests output::tests -- --nocapture`

Expected: PASS

### Task 5: Add backend client and read commands

**Files:**
- Create: `crates/sss-admin-cli/src/backend.rs`
- Create: `crates/sss-admin-cli/src/commands/read.rs`
- Modify: `crates/sss-admin-cli/src/lib.rs`

**Step 1: Write the failing tests**

Cover:
- `status`
- `supply`
- `holders`
- `audit-log`
- backend-first routing using a mock HTTP server

**Step 2: Run tests to verify failure**

Run: `cargo test -p sss-admin-cli read::tests -- --nocapture`

Expected: FAIL because the backend client and handlers are missing.

**Step 3: Write minimal implementation**

Implement typed backend calls for the current API surface and route read commands through them. If an API gap blocks `holders` or `audit-log`, add a TODO marker in code and document the backend endpoint needed.

**Step 4: Run tests to verify pass**

Run: `cargo test -p sss-admin-cli read::tests -- --nocapture`

Expected: PASS for covered endpoints.

### Task 6: Add chain and governed operation routing for mutating commands

**Files:**
- Create: `crates/sss-admin-cli/src/router.rs`
- Create: `crates/sss-admin-cli/src/chain.rs`
- Create: `crates/sss-admin-cli/src/commands/mutate.rs`
- Modify: `crates/sss-admin-cli/src/lib.rs`

**Step 1: Write the failing tests**

Cover:
- `mint`
- `burn`
- `freeze`
- `thaw`
- `pause`
- `unpause`
- `blacklist add/remove`
- `seize`
- `minters add/remove/list`

**Step 2: Run tests to verify failure**

Run: `cargo test -p sss-admin-cli mutate::tests -- --nocapture`

Expected: FAIL because routing and handlers are missing.

**Step 3: Write minimal implementation**

Implement internal policy routing, backend operation submission where available, and chain execution stubs or direct calls using generated clients where necessary.

**Step 4: Run tests to verify pass**

Run: `cargo test -p sss-admin-cli mutate::tests -- --nocapture`

Expected: PASS for mocked and unit-covered paths.

### Task 7: Wire init execution against generated client

**Files:**
- Modify: `crates/sss-admin-cli/src/init.rs`
- Modify: `crates/sss-admin-cli/src/chain.rs`
- Test: `crates/sss-admin-cli/src/init.rs`

**Step 1: Write the failing tests**

Cover:
- `init` with `sss-1`
- `init` with `sss-2`
- `init --config config.toml`
- generated config file creation when missing

**Step 2: Run tests to verify failure**

Run: `cargo test -p sss-admin-cli init::tests::execution -- --nocapture`

Expected: FAIL because the execution layer is incomplete.

**Step 3: Write minimal implementation**

Translate resolved init config into the generated stablecoin client calls and persist the config file when needed after successful resolution/execution.

**Step 4: Run tests to verify pass**

Run: `cargo test -p sss-admin-cli init::tests::execution -- --nocapture`

Expected: PASS

### Task 8: Add end-to-end command tests

**Files:**
- Create: `crates/sss-admin-cli/tests/cli_smoke.rs`
- Modify: `crates/sss-admin-cli/Cargo.toml`

**Step 1: Write the failing smoke tests**

Cover representative commands:
- `init`
- `status`
- `supply`
- `mint`
- `freeze`
- `blacklist add`
- `minters list`

**Step 2: Run tests to verify failure**

Run: `cargo test -p sss-admin-cli --test cli_smoke -- --nocapture`

Expected: FAIL because command dispatch is incomplete.

**Step 3: Write minimal implementation**

Add the missing command wiring and test utilities to execute the CLI in-process with mocked dependencies.

**Step 4: Run tests to verify pass**

Run: `cargo test -p sss-admin-cli --test cli_smoke -- --nocapture`

Expected: PASS

### Task 9: Run verification and document backend gaps

**Files:**
- Modify: `README.md`
- Modify: `backend/BACKEND_FLOW.md` if CLI-driven backend additions are needed

**Step 1: Run the verification commands**

Run:
- `cargo test -p sss-admin-cli`
- `cargo check -p sss-admin-cli`
- `cargo test -p sss_api`

Expected: PASS, or clear notes about unrelated failures in the dirty worktree.

**Step 2: Document command usage**

Add a short CLI section to the repo docs with examples for `init`, admin actions, and read commands.

**Step 3: Commit**

Commit only the CLI-specific files and docs if the worktree state is clean enough to do so safely.
