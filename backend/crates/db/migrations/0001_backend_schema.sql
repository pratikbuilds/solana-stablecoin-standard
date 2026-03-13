create table if not exists indexer_checkpoints (
    pipeline_name text primary key,
    program_id text not null,
    last_finalized_slot bigint not null default 0,
    last_tx_signature text,
    updated_at timestamptz not null default now()
);

create table if not exists chain_events (
    id bigserial primary key,
    event_uid text not null unique,
    program_id text not null,
    mint text,
    event_source text not null,
    event_type text not null,
    slot bigint not null,
    tx_signature text not null,
    instruction_index integer not null,
    inner_instruction_index integer,
    event_index integer,
    block_time timestamptz,
    payload jsonb not null,
    created_at timestamptz not null default now()
);

create index if not exists chain_events_mint_slot_idx on chain_events (mint, slot desc, id desc);
create index if not exists chain_events_signature_idx on chain_events (tx_signature);
create index if not exists chain_events_program_slot_idx on chain_events (program_id, slot desc);

create table if not exists mints (
    mint text primary key,
    preset text not null,
    authority text not null,
    name text not null,
    symbol text not null,
    uri text not null,
    decimals smallint not null,
    enable_permanent_delegate boolean not null,
    enable_transfer_hook boolean not null,
    default_account_frozen boolean not null,
    paused boolean not null,
    total_minted numeric(20,0) not null default 0,
    total_burned numeric(20,0) not null default 0,
    created_at timestamptz not null,
    last_changed_by text not null,
    last_changed_at timestamptz not null,
    indexed_slot bigint not null
);

create table if not exists mint_roles (
    mint text primary key references mints(mint) on delete cascade,
    master_authority text not null,
    pauser text not null,
    burner text not null,
    blacklister text not null,
    seizer text not null,
    updated_at timestamptz not null,
    indexed_slot bigint not null
);

create table if not exists minter_quotas (
    mint text not null references mints(mint) on delete cascade,
    minter text not null,
    quota numeric(20,0) not null,
    minted numeric(20,0) not null,
    active boolean not null,
    updated_at timestamptz not null,
    indexed_slot bigint not null,
    primary key (mint, minter)
);

create table if not exists blacklist_entries (
    mint text not null references mints(mint) on delete cascade,
    wallet text not null,
    reason text not null,
    blacklisted_by text not null,
    blacklisted_at timestamptz not null,
    active boolean not null,
    removed_at timestamptz,
    indexed_slot bigint not null,
    primary key (mint, wallet)
);

create table if not exists compliance_actions (
    id bigserial primary key,
    mint text not null references mints(mint) on delete cascade,
    action_type text not null,
    wallet text,
    token_account text,
    authority text not null,
    amount numeric(20,0),
    tx_signature text not null,
    slot bigint not null,
    related_operation_id uuid,
    details jsonb not null default '{}'::jsonb,
    occurred_at timestamptz not null
);

create table if not exists operation_requests (
    id uuid primary key,
    kind text not null,
    mint text not null references mints(mint) on delete cascade,
    target_wallet text,
    target_token_account text,
    amount numeric(20,0),
    reason text,
    external_reference text,
    idempotency_key text not null unique,
    status text not null,
    requested_by text not null,
    approved_by text,
    tx_signature text,
    failure_reason text,
    metadata jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index if not exists operation_requests_status_created_idx on operation_requests (status, created_at asc);

create table if not exists operation_attempts (
    id bigserial primary key,
    operation_id uuid not null references operation_requests(id) on delete cascade,
    attempt_number integer not null,
    status text not null,
    signer_backend text not null,
    tx_signature text,
    rpc_endpoint text,
    error_message text,
    started_at timestamptz not null,
    finished_at timestamptz,
    unique (operation_id, attempt_number)
);

create table if not exists webhook_endpoints (
    id uuid primary key,
    name text not null,
    url text not null,
    secret text not null,
    subscribed_event_types text[] not null,
    active boolean not null default true,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table if not exists webhook_deliveries (
    id bigserial primary key,
    webhook_endpoint_id uuid not null references webhook_endpoints(id) on delete cascade,
    source_event_key text not null,
    event_type text not null,
    payload jsonb not null,
    status text not null,
    attempt_count integer not null default 0,
    next_attempt_at timestamptz,
    last_http_status integer,
    last_error text,
    delivered_at timestamptz,
    created_at timestamptz not null default now(),
    unique (webhook_endpoint_id, source_event_key)
);

create index if not exists webhook_deliveries_status_next_idx on webhook_deliveries (status, next_attempt_at asc);

create table if not exists audit_exports (
    id uuid primary key,
    status text not null,
    requested_by text not null,
    filters jsonb not null default '{}'::jsonb,
    artifact_path text,
    error_message text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
