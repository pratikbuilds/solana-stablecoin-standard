-- Schema revamp: drop old tables, create new simplified schema.
-- Drop order: respect FK dependencies (children before parents).

DROP TABLE IF EXISTS operation_attempts;
DROP TABLE IF EXISTS operation_requests;
DROP TABLE IF EXISTS compliance_actions;
DROP TABLE IF EXISTS blacklist_entries;
DROP TABLE IF EXISTS minter_quotas;
DROP TABLE IF EXISTS mint_roles;
DROP TABLE IF EXISTS mints;
DROP TABLE IF EXISTS webhook_deliveries;
DROP TABLE IF EXISTS webhook_endpoints;
DROP TABLE IF EXISTS audit_exports;
DROP TABLE IF EXISTS chain_events;
DROP TABLE IF EXISTS indexer_checkpoints;

-- lifecycle_requests (mint/burn lifecycle)
CREATE TABLE lifecycle_requests (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL CHECK (type IN ('mint', 'burn')),
  status TEXT NOT NULL CHECK (status IN ('requested', 'approved', 'signing', 'submitted', 'finalized', 'failed', 'cancelled')),
  mint TEXT NOT NULL,
  recipient TEXT,
  token_account TEXT,
  amount NUMERIC(20,0) NOT NULL,
  minter TEXT,
  reason TEXT,
  idempotency_key TEXT UNIQUE,
  requested_by TEXT NOT NULL,
  approved_by TEXT,
  tx_signature TEXT,
  error TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_lifecycle_requests_status ON lifecycle_requests(status);
CREATE INDEX idx_lifecycle_requests_created ON lifecycle_requests(created_at ASC);

-- events (on-chain event log from indexer)
CREATE TABLE events (
  id BIGSERIAL PRIMARY KEY,
  event_type TEXT NOT NULL,
  program_id TEXT,
  mint TEXT,
  tx_signature TEXT NOT NULL,
  slot BIGINT NOT NULL,
  block_time TIMESTAMPTZ,
  instruction_index INTEGER NOT NULL DEFAULT 0,
  data JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(tx_signature, instruction_index, event_type)
);
CREATE INDEX idx_events_event_type ON events(event_type);
CREATE INDEX idx_events_slot ON events(slot DESC);
CREATE INDEX idx_events_tx ON events(tx_signature);
CREATE INDEX idx_events_mint_slot ON events(mint, slot DESC) WHERE mint IS NOT NULL;
CREATE INDEX idx_events_program ON events(program_id) WHERE program_id IS NOT NULL;
CREATE INDEX idx_events_block_time ON events(block_time) WHERE block_time IS NOT NULL;

-- webhook_subscriptions
CREATE TABLE webhook_subscriptions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT,
  url TEXT NOT NULL,
  events TEXT[] NOT NULL,
  secret TEXT,
  active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- webhook_deliveries
CREATE TABLE webhook_deliveries (
  id BIGSERIAL PRIMARY KEY,
  subscription_id UUID NOT NULL REFERENCES webhook_subscriptions(id) ON DELETE CASCADE,
  event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
  status TEXT NOT NULL CHECK (status IN ('pending', 'delivered', 'failed', 'dead_letter')),
  attempts INTEGER NOT NULL DEFAULT 0,
  max_attempts INTEGER NOT NULL DEFAULT 5,
  last_attempt_at TIMESTAMPTZ,
  next_retry_at TIMESTAMPTZ,
  response_code INTEGER,
  error TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(subscription_id, event_id)
);
CREATE INDEX idx_webhook_deliveries_pending ON webhook_deliveries(status, next_retry_at) WHERE status IN ('pending', 'failed');

-- audit_trail
CREATE TABLE audit_trail (
  id BIGSERIAL PRIMARY KEY,
  action TEXT NOT NULL,
  actor TEXT NOT NULL,
  target TEXT,
  mint TEXT,
  request_id TEXT,
  details JSONB,
  tx_signature TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_trail_action ON audit_trail(action);
CREATE INDEX idx_audit_trail_actor ON audit_trail(actor);
CREATE INDEX idx_audit_trail_mint ON audit_trail(mint) WHERE mint IS NOT NULL;
CREATE INDEX idx_audit_trail_created ON audit_trail(created_at DESC);

-- indexer_state (key-value cursor)
CREATE TABLE indexer_state (
  key TEXT PRIMARY KEY,
  value JSONB NOT NULL
);
