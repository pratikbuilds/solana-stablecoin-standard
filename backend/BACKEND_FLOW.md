# Backend flow: Indexer, API, DB, Workers

High-level: **Solana** → **Indexer** (Carbon pipeline) → **Postgres** ← **API** (Axum HTTP + optional workers).

---

## 1. Crates

| Crate    | Role |
|---------|------|
| **domain** | Shared types: `ChainEvent`, `MintRecord`, `OperationRequest`, traits (`SignerBackend`, `WebhookDispatcher`, `AuditExporter`). No I/O. |
| **db**     | Postgres: `Store` (sqlx), migrations, all reads/writes. Single source of truth. |
| **indexer**| Listens to Solana, decodes events, writes `chain_events` and applies projections into mints/roles/quotas/blacklist/compliance. |
| **api**    | HTTP API (Axum): read mints/events/blacklist, create operations/webhooks/audit exports; optional background workers. |

---

## 2. Indexer flow (chain → DB)

### Entry

- **Binary**: `backend/crates/indexer/src/main.rs` → `IndexerService::new(config).run_live()`.
- **Config** (`IndexerConfig::from_env()`): `DATABASE_URL`, `SOLANA_RPC_URL`, `SSS_STABLECOIN_PROGRAM_ID`, `SSS_TRANSFER_HOOK_PROGRAM_ID`, `SSS_START_SLOT`, `SSS_DISABLE_BLOCK_SUBSCRIBE`.

### Pipeline (`pipeline.rs`)

1. **Carbon pipeline**:
   - **Data sources** (one or both):
     - **Block subscribe** (WebSocket): live blocks from RPC (`RpcBlockSubscribe`), unless `SSS_DISABLE_BLOCK_SUBSCRIBE=true`.
     - **Block crawler** (RPC): historical from `SSS_START_SLOT` to latest slot (`RpcBlockCrawler`), only if `start_slot > 0`.
   - **Processor**: `CarbonTransactionProcessor` receives transactions with instructions.

2. **Filtering**: Only transactions that mention the **stablecoin program** or **transfer-hook program** are processed.

3. **Decoding** (`decode.rs`):
   - **Stablecoin events**: From tx log lines `Program data: <base64>`. First 8 bytes = Anchor discriminator; rest = event body. Decoded into `ChainEvent` (e.g. `StablecoinInitialized`, `TokensMinted`, `AddressBlacklisted`, …).
   - **Transfer-hook**: No event log; from **instruction accounts** (mint, source, dest, authority) and **logs** (blacklist/reject messages). Synthesized into `ChainEvent` (e.g. `transfer_checked`, `transfer_rejected_source_blacklisted`).

4. **Ingestion** (`service.rs`):
   - **Persist**: `store.insert_chain_event(event)` → `chain_events` (by `event_uid`, dedup).
   - **Project**: `apply_projection(event)` updates derived tables by `event_type`:
     - `StablecoinInitialized` → `mints` (upsert).
     - `RolesUpdated` / `AuthorityTransferred` → `mint_roles`.
     - `MinterUpdated` → `minter_quotas`.
     - `TokensMinted` / `TokensBurned` → `mints` (total_minted/total_burned) and optionally minter quota.
     - `PauseChanged` → `mints.paused`.
     - `AddressBlacklisted` / `AddressUnblacklisted` → `blacklist_entries`.
     - Blacklist + freeze/thaw/seize/transfer_rejected/transfer_checked → `compliance_actions`.
   - **Checkpoint**: After each tx, `upsert_checkpoint("stablecoin-main", program_id, slot, tx_sig)` (used by Carbon for resume).

So: **RPC/WS → raw txs → decode logs/instructions → ChainEvent → DB (chain_events + projections)**.

---

## 3. API flow (HTTP → DB)

### Entry

- **Binary**: `backend/crates/api/src/main.rs` → `run(ApiConfig::from_env())`.
- **Config**: `DATABASE_URL`, `SSS_API_BIND` (default `127.0.0.1:8080`).
- **Startup**: `Store::connect` + `migrate()` then Axum server. **Workers are not started** by `run()`; you can call `spawn_default_workers(store)` if you want background jobs.

### Routes (`routes.rs`)

API is grouped into: **mints** (read-only), **lifecycle** (fiat–stablecoin), **compliance** (SSS-2), **webhooks** (SSS-2).

- **Health**: `GET /healthz`, `GET /readyz` (DB check).
- **Mints (read-only, from indexer)**:
  - `GET /v1/mints` → `list_mints`
  - `GET /v1/mints/:mint` → `get_mint`
  - `GET /v1/mints/:mint/events` → `list_chain_events`
  - `GET /v1/mints/:mint/blacklist` → `list_blacklist_entries`
- **Fiat-to-stablecoin (request → verify → execute)**:
  - `POST /v1/mint-requests`, `POST /v1/burn-requests` → create operation (status `requested`).
  - `GET /v1/operations/:id` → operation + attempts.
  - `POST /v1/operations/:id/approve` → admin verification → status `approved`.
  - `POST /v1/operations/:id/execute` → 202 + operation (actual mint/burn on-chain is done by **worker** via signer).
- **Compliance (SSS-2: blacklist, sanctions point, audit)**:
  - `POST /v1/compliance/blacklists`, `DELETE /v1/compliance/blacklists/:mint/:wallet`, `POST /v1/compliance/freeze`, `thaw`, `seize` → create `operation_requests` (status `requested`).
  - `POST /v1/compliance/audit-exports` → `audit_exports` (status `requested`).
- **Webhooks (SSS-2: event notifications with retry)**: `POST /v1/webhooks/endpoints` → `webhook_endpoints`.

All mutations go through `Store`; the API is stateless and DB-backed.

---

## 4. Workers (optional background jobs)

Defined in `api/src/workers.rs`. Started only if you call `spawn_default_workers(store)` (e.g. from a custom `main` or test).

1. **OperationExecutorWorker**
   - Polls `operation_requests` with status `approved`.
   - Marks `signing`, creates `operation_attempts`, calls `SignerBackend::execute()` (default impl: `LocalKeypairSigner` returns “not wired yet”).
   - On success: `Submitted` + tx_sig; on failure: `Failed` + reason.

2. **WebhookRetryWorker**
   - Polls `webhook_deliveries` (pending/failed, `next_attempt_at <= now`).
   - Uses `WebhookDispatcher::deliver()` (default: `HttpWebhookDispatcher` – POST with HMAC signature).
   - Updates status (delivered / failed / dead_letter) and backoff.

3. **AuditExportWorker**
   - Polls `audit_exports` with status `requested`.
   - Marks `processing`, calls `AuditExporter::export()` (default: `JsonAuditExporter` – stub path), then `completed` or `failed`.

Note: Webhook **enqueue** is not in the current indexer path (no automatic “on chain_event insert → enqueue webhook_deliveries”). That would be an extra step (e.g. in indexer after `insert_chain_event` or a separate process reading `chain_events`).

---

## 5. Database (schema)

- **indexer_checkpoints**: pipeline name, program_id, last slot/signature (Carbon resume).
- **chain_events**: raw events (event_uid, program_id, mint, event_type, slot, tx_signature, payload, …).
- **mints**: one row per mint (authority, supply, paused, …).
- **mint_roles**, **minter_quotas**, **blacklist_entries**: per-mint state.
- **compliance_actions**: audit trail (freeze, thaw, blacklist, seize, transfer_checked/rejected).
- **operation_requests** + **operation_attempts**: request/approve/execute lifecycle.
- **webhook_endpoints** + **webhook_deliveries**.
- **audit_exports**.

---

## 6. How to run and test

### Prerequisites

- Rust toolchain, Postgres, and (for indexer) Solana RPC (e.g. devnet).

### Database

```bash
# Create DB and run migrations (from backend/)
createdb sss_backend
DATABASE_URL=postgres://localhost/sss_backend cargo run -p sss_api -- --help  # or run once to trigger migrate
# Or run indexer once; it also migrates on startup
```

### API only (read-only + create operations)

```bash
export DATABASE_URL=postgres://localhost/sss_backend
cargo run -p sss_api
# Default: http://127.0.0.1:8080
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/readyz
curl http://127.0.0.1:8080/v1/mints
```

### Indexer (fill DB from chain)

```bash
export DATABASE_URL=postgres://localhost/sss_backend
export SOLANA_RPC_URL=https://api.devnet.solana.com
# Optional: SSS_STABLECOIN_PROGRAM_ID, SSS_TRANSFER_HOOK_PROGRAM_ID, SSS_START_SLOT, SSS_DISABLE_BLOCK_SUBSCRIBE
cargo run -p sss_indexer
```

Run indexer first to backfill/catch up; then use API to query mints/events/blacklist and create operations.

### Integration tests

- **API** (`backend/crates/api/tests/api_integration.rs`): Starts Postgres (or uses `TEST_DATABASE_ADMIN_URL`), migrates, seeds a mint + chain event + blacklist, then hits routes (health, mints, events, blacklist, create mint/burn/compliance ops, approve, execute). Also tests workers with mocks.

  ```bash
  cd backend
  cargo test -p sss_api
  # With existing Postgres (faster, no initdb):
  TEST_DATABASE_ADMIN_URL=postgres://user:pass@localhost/postgres cargo test -p sss_api
  ```

- **Indexer** (`backend/crates/indexer/tests/projections.rs`): Postgres harness, `IndexerService` with a test config, then `ingest_chain_event` for various event types and asserts on `mints` / `mint_roles` / `minter_quotas` / `blacklist_entries` / `compliance_actions`.

  ```bash
  cargo test -p sss_indexer
  # Or with existing DB:
  TEST_DATABASE_ADMIN_URL=postgres://user:pass@localhost/postgres cargo test -p sss_indexer
  ```

### Quick local E2E

1. Start Postgres, set `DATABASE_URL`.
2. Start API: `cargo run -p sss_api` (optional: add `spawn_default_workers` in code to run workers).
3. Start indexer: `cargo run -p sss_indexer` (point at devnet or local validator).
4. After some blocks: `curl http://127.0.0.1:8080/v1/mints` and `curl http://127.0.0.1:8080/v1/mints/<mint>/events`.

---

## 7. Summary diagram

```
Solana (RPC + WS)
       │
       ▼
┌──────────────────────────────────────────────────────────────┐
│  Indexer (Carbon pipeline)                                   │
│  • Block subscribe / block crawler                           │
│  • Filter by stablecoin + transfer-hook program             │
│  • decode_stablecoin_events (logs) + synthesize_transfer_   │
│    hook_from_instruction (accounts + logs)                    │
│  • insert_chain_event + apply_projection                     │
└──────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────┐
│  Postgres (Store)                                            │
│  chain_events, mints, mint_roles, minter_quotas,            │
│  blacklist_entries, compliance_actions,                     │
│  operation_requests, operation_attempts,                     │
│  webhook_endpoints, webhook_deliveries, audit_exports        │
└──────────────────────────────────────────────────────────────┘
       ▲
       │
┌──────────────────────────────────────────────────────────────┐
│  API (Axum)                                                  │
│  • HTTP routes → Store (read + create operations/webhooks/   │
│    audit exports)                                            │
│  • Optional: spawn_default_workers → OperationExecutor,      │
│    WebhookRetry, AuditExport (poll DB, call Signer/Dispatcher/│
│    Exporter)                                                  │
└──────────────────────────────────────────────────────────────┘
```

This is the full flow from chain to DB and from HTTP/workers to DB.
