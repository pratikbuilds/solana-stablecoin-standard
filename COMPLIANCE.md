# Compliance

This repository does not encode jurisdiction-specific legal advice. It provides technical controls that can support a regulated stablecoin operating model.

## Regulatory Considerations

Typical areas an issuer should map outside the codebase:

- reserve attestation and reconciliation
- mint and burn approval workflow
- sanctions screening and watchlist sourcing
- case management for freeze, thaw, and seizure decisions
- retention policy for operation and audit records
- custody and signer segregation

The code supports these controls in a few specific ways:

- role separation through `RoleConfig`
- per-minter quotas through `MinterQuota`
- immutable onchain event emission
- indexed audit tables in Postgres
- request, approval, and attempt tracking for backend-driven operations

## Audit Trail Format

The backend schema provides two audit surfaces:

### `chain_events`

Raw chain activity captured by the indexer.

Recommended interpretation:

- `event_uid`: deterministic dedupe key
- `program_id`: source program
- `mint`: affected mint when present
- `event_source`: `anchor_event`, `instruction`, or `synthetic_transfer_hook`
- `event_type`: lifecycle or compliance event name
- `slot` and `tx_signature`: chain position
- `payload`: event-specific JSON

### `compliance_actions`

Derived compliance-oriented projection optimized for reviews and exports.

Fields:

- `mint`
- `action_type`
- `wallet`
- `token_account`
- `authority`
- `amount`
- `tx_signature`
- `slot`
- `related_operation_id`
- `details`
- `occurred_at`

Recommended export shape:

```json
{
  "mint": "mint-address",
  "action_type": "tokens_seized",
  "wallet": "wallet-address",
  "token_account": "token-account-address",
  "authority": "authority-address",
  "amount": "400000",
  "tx_signature": "5abc...",
  "slot": 123456789,
  "related_operation_id": "00000000-0000-0000-0000-000000000000",
  "details": {
    "reason": "sanctions review",
    "event_source": "anchor_event"
  },
  "occurred_at": "2026-03-13T12:00:00Z"
}
```

## Minimum Evidence to Preserve

For every compliance-sensitive action, preserve:

- the onchain transaction signature
- the mint address
- the acting authority
- the target wallet and token account when applicable
- the amount when applicable
- the case or external reference
- the human-readable reason

## Practical Gaps

The repository intentionally leaves some policy items to the operator:

- sanctions list ingestion
- legal hold workflows
- case approval UI
- final artifact storage for audit export
- external signer or HSM integration

Those gaps should be closed before production use in a regulated environment.
