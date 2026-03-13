# API

The backend API is implemented in `backend/crates/api` and is backed by Postgres projections from the indexer.

Base URL defaults to `http://127.0.0.1:8080`.

## Health

### `GET /healthz`

Returns process liveness.

Response:

```json
{ "status": "ok" }
```

### `GET /readyz`

Returns readiness after a database check.

Response:

```json
{ "status": "ready" }
```

## Mint Catalog

### `GET /v1/mints`

Returns indexed mint records.

### `GET /v1/mints/:mint`

Returns one mint record or `404`.

### `GET /v1/mints/:mint/events`

Returns up to 100 indexed `chain_events` for the mint.

### `GET /v1/mints/:mint/blacklist`

Returns indexed blacklist entries for the mint.

## Operation Requests

Mutating routes create offchain operation requests. Worker execution is optional and controlled by `SSS_RUN_WORKERS=1`.

Shared request body for mint, burn, freeze, thaw, and seize:

```json
{
  "mint": "mint-address",
  "target_wallet": "wallet-address",
  "target_token_account": "token-account-address",
  "amount": 1000000,
  "reason": "issuer action",
  "external_reference": "case-123",
  "idempotency_key": "unique-key",
  "requested_by": "ops@example.com",
  "metadata": {}
}
```

Fields may be omitted when the operation type does not require them.

### `POST /v1/mint-requests`

Creates an operation with `kind = mint`.

### `POST /v1/burn-requests`

Creates an operation with `kind = burn`.

### `POST /v1/compliance/freeze`

Creates an operation with `kind = freeze`.

### `POST /v1/compliance/thaw`

Creates an operation with `kind = thaw`.

### `POST /v1/compliance/seize`

Creates an operation with `kind = seize`.

### `POST /v1/compliance/blacklists`

Creates an operation with `kind = blacklist_add`.

Typical body:

```json
{
  "mint": "mint-address",
  "target_wallet": "wallet-address",
  "reason": "sanctions review",
  "idempotency_key": "blacklist-wallet-address",
  "requested_by": "compliance@example.com",
  "metadata": {
    "source": "manual-review"
  }
}
```

### `DELETE /v1/compliance/blacklists/:mint/:wallet`

Creates an operation with `kind = blacklist_remove`.

The API generates:

- `reason = "remove from blacklist"`
- `requested_by = "system"`
- `idempotency_key = "blacklist-remove:<wallet>"`

### `GET /v1/operations/:id`

Returns:

```json
{
  "operation": {
    "id": "uuid",
    "kind": "mint",
    "status": "requested"
  },
  "attempts": []
}
```

### `POST /v1/operations/:id/approve`

Approves an operation.

Request body:

```json
{
  "approved_by": "approver@example.com"
}
```

### `POST /v1/operations/:id/execute`

Returns `202 Accepted` if the operation is already `approved` or `submitted`.

This endpoint does not directly submit the chain transaction. It signals readiness for worker execution.

## Webhooks

### `POST /v1/webhooks/endpoints`

Registers a webhook endpoint.

Request body:

```json
{
  "name": "compliance-sink",
  "url": "https://example.com/hooks/sss",
  "secret": "shared-secret",
  "subscribed_event_types": [
    "AddressBlacklisted",
    "TokensSeized",
    "transfer_rejected_source_blacklisted"
  ]
}
```

## Audit Exports

### `POST /v1/compliance/audit-exports`

Creates an audit export request.

Request body:

```json
{
  "requested_by": "auditor@example.com",
  "filters": {
    "mint": "mint-address",
    "from": "2026-01-01T00:00:00Z",
    "to": "2026-03-13T00:00:00Z"
  }
}
```

The default exporter is a stub JSON exporter wired through the worker abstraction.
