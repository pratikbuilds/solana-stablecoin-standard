use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use sss_domain::{
    AttemptStatus, AuditExport, AuditExportStatus, BlacklistEntryRecord, ChainEvent,
    ComplianceActionRecord, CreateAuditExport, CreateOperationRequest, CreateWebhookEndpoint,
    EventSource, HealthReport, MintRecord, MintRoleRecord, MinterQuotaRecord, OperationAttempt,
    OperationKind, OperationRequest, OperationStatus, WebhookDelivery, WebhookDeliveryStatus,
    WebhookEndpoint,
};
use uuid::Uuid;

pub const MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .with_context(|| "failed to connect to postgres")?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<()> {
        MIGRATOR.run(&self.pool).await?;
        Ok(())
    }

    pub async fn readiness_check(&self) -> Result<()> {
        sqlx::query("select 1").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn upsert_checkpoint(
        &self,
        pipeline_name: &str,
        program_id: &str,
        last_finalized_slot: i64,
        last_tx_signature: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            insert into indexer_checkpoints (pipeline_name, program_id, last_finalized_slot, last_tx_signature)
            values ($1, $2, $3, $4)
            on conflict (pipeline_name) do update set
                program_id = excluded.program_id,
                last_finalized_slot = excluded.last_finalized_slot,
                last_tx_signature = excluded.last_tx_signature,
                updated_at = now()
            "#,
        )
        .bind(pipeline_name)
        .bind(program_id)
        .bind(last_finalized_slot)
        .bind(last_tx_signature)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_chain_event(&self, event: &ChainEvent) -> Result<()> {
        sqlx::query(
            r#"
            insert into chain_events (
                event_uid, program_id, mint, event_source, event_type, slot,
                tx_signature, instruction_index, inner_instruction_index, event_index,
                block_time, payload
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            on conflict (event_uid) do nothing
            "#,
        )
        .bind(&event.event_uid)
        .bind(&event.program_id)
        .bind(&event.mint)
        .bind(event.event_source.as_str())
        .bind(&event.event_type)
        .bind(event.slot)
        .bind(&event.tx_signature)
        .bind(event.instruction_index)
        .bind(event.inner_instruction_index)
        .bind(event.event_index)
        .bind(event.block_time)
        .bind(&event.payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_chain_events(&self, mint: &str, limit: i64) -> Result<Vec<ChainEvent>> {
        let rows = sqlx::query(
            r#"
            select event_uid, program_id, mint, event_source, event_type, slot, tx_signature,
                   instruction_index, inner_instruction_index, event_index, block_time, payload
            from chain_events
            where mint = $1
            order by slot desc, id desc
            limit $2
            "#,
        )
        .bind(mint)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_chain_event).collect()
    }

    pub async fn upsert_mint(&self, mint: &MintRecord) -> Result<()> {
        sqlx::query(
            r#"
            insert into mints (
                mint, preset, authority, name, symbol, uri, decimals,
                enable_permanent_delegate, enable_transfer_hook, default_account_frozen,
                paused, total_minted, total_burned, created_at, last_changed_by,
                last_changed_at, indexed_slot
            )
            values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12::numeric,$13::numeric,$14,$15,$16,$17)
            on conflict (mint) do update set
                preset = excluded.preset,
                authority = excluded.authority,
                name = excluded.name,
                symbol = excluded.symbol,
                uri = excluded.uri,
                decimals = excluded.decimals,
                enable_permanent_delegate = excluded.enable_permanent_delegate,
                enable_transfer_hook = excluded.enable_transfer_hook,
                default_account_frozen = excluded.default_account_frozen,
                paused = excluded.paused,
                total_minted = excluded.total_minted,
                total_burned = excluded.total_burned,
                last_changed_by = excluded.last_changed_by,
                last_changed_at = excluded.last_changed_at,
                indexed_slot = excluded.indexed_slot
            "#,
        )
        .bind(&mint.mint)
        .bind(&mint.preset)
        .bind(&mint.authority)
        .bind(&mint.name)
        .bind(&mint.symbol)
        .bind(&mint.uri)
        .bind(mint.decimals)
        .bind(mint.enable_permanent_delegate)
        .bind(mint.enable_transfer_hook)
        .bind(mint.default_account_frozen)
        .bind(mint.paused)
        .bind(mint.total_minted.to_string())
        .bind(mint.total_burned.to_string())
        .bind(mint.created_at)
        .bind(&mint.last_changed_by)
        .bind(mint.last_changed_at)
        .bind(mint.indexed_slot)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_mints(&self) -> Result<Vec<MintRecord>> {
        let rows = sqlx::query(
            r#"
            select mint, preset, authority, name, symbol, uri, decimals,
                   enable_permanent_delegate, enable_transfer_hook, default_account_frozen,
                   paused, total_minted::text as total_minted, total_burned::text as total_burned, created_at, last_changed_by,
                   last_changed_at, indexed_slot
            from mints
            order by created_at desc
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_mint).collect()
    }

    pub async fn get_mint(&self, mint: &str) -> Result<Option<MintRecord>> {
        let row = sqlx::query(
            r#"
            select mint, preset, authority, name, symbol, uri, decimals,
                   enable_permanent_delegate, enable_transfer_hook, default_account_frozen,
                   paused, total_minted::text as total_minted, total_burned::text as total_burned, created_at, last_changed_by,
                   last_changed_at, indexed_slot
            from mints
            where mint = $1
            "#,
        )
        .bind(mint)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_mint).transpose()
    }

    pub async fn upsert_mint_roles(&self, roles: &MintRoleRecord) -> Result<()> {
        sqlx::query(
            r#"
            insert into mint_roles (
                mint, master_authority, pauser, burner, blacklister, seizer, updated_at, indexed_slot
            )
            values ($1,$2,$3,$4,$5,$6,$7,$8)
            on conflict (mint) do update set
                master_authority = excluded.master_authority,
                pauser = excluded.pauser,
                burner = excluded.burner,
                blacklister = excluded.blacklister,
                seizer = excluded.seizer,
                updated_at = excluded.updated_at,
                indexed_slot = excluded.indexed_slot
            "#,
        )
        .bind(&roles.mint)
        .bind(&roles.master_authority)
        .bind(&roles.pauser)
        .bind(&roles.burner)
        .bind(&roles.blacklister)
        .bind(&roles.seizer)
        .bind(roles.updated_at)
        .bind(roles.indexed_slot)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_minter_quota(&self, quota: &MinterQuotaRecord) -> Result<()> {
        sqlx::query(
            r#"
            insert into minter_quotas (mint, minter, quota, minted, active, updated_at, indexed_slot)
            values ($1,$2,$3::numeric,$4::numeric,$5,$6,$7)
            on conflict (mint, minter) do update set
                quota = excluded.quota,
                minted = excluded.minted,
                active = excluded.active,
                updated_at = excluded.updated_at,
                indexed_slot = excluded.indexed_slot
            "#,
        )
        .bind(&quota.mint)
        .bind(&quota.minter)
        .bind(quota.quota.to_string())
        .bind(quota.minted.to_string())
        .bind(quota.active)
        .bind(quota.updated_at)
        .bind(quota.indexed_slot)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_minter_quota(&self, mint: &str, minter: &str) -> Result<Option<MinterQuotaRecord>> {
        let row = sqlx::query(
            r#"
            select mint, minter, quota::text as quota, minted::text as minted, active, updated_at, indexed_slot
            from minter_quotas
            where mint = $1 and minter = $2
            "#,
        )
        .bind(mint)
        .bind(minter)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_minter_quota).transpose()
    }

    pub async fn upsert_blacklist_entry(&self, entry: &BlacklistEntryRecord) -> Result<()> {
        sqlx::query(
            r#"
            insert into blacklist_entries (
                mint, wallet, reason, blacklisted_by, blacklisted_at, active, removed_at, indexed_slot
            )
            values ($1,$2,$3,$4,$5,$6,$7,$8)
            on conflict (mint, wallet) do update set
                reason = excluded.reason,
                blacklisted_by = excluded.blacklisted_by,
                blacklisted_at = excluded.blacklisted_at,
                active = excluded.active,
                removed_at = excluded.removed_at,
                indexed_slot = excluded.indexed_slot
            "#,
        )
        .bind(&entry.mint)
        .bind(&entry.wallet)
        .bind(&entry.reason)
        .bind(&entry.blacklisted_by)
        .bind(entry.blacklisted_at)
        .bind(entry.active)
        .bind(entry.removed_at)
        .bind(entry.indexed_slot)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_blacklist_entries(&self, mint: &str) -> Result<Vec<BlacklistEntryRecord>> {
        let rows = sqlx::query(
            r#"
            select mint, wallet, reason, blacklisted_by, blacklisted_at, active, removed_at, indexed_slot
            from blacklist_entries
            where mint = $1 and active = true
            order by blacklisted_at desc
            "#,
        )
        .bind(mint)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_blacklist_entry).collect()
    }

    pub async fn insert_compliance_action(&self, action: &ComplianceActionRecord) -> Result<()> {
        sqlx::query(
            r#"
            insert into compliance_actions (
                mint, action_type, wallet, token_account, authority, amount, tx_signature,
                slot, related_operation_id, details, occurred_at
            )
            values ($1,$2,$3,$4,$5,$6::numeric,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(&action.mint)
        .bind(&action.action_type)
        .bind(&action.wallet)
        .bind(&action.token_account)
        .bind(&action.authority)
        .bind(action.amount.map(|v| v.to_string()))
        .bind(&action.tx_signature)
        .bind(action.slot)
        .bind(action.related_operation_id)
        .bind(&action.details)
        .bind(action.occurred_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn create_operation_request(&self, request: &CreateOperationRequest) -> Result<OperationRequest> {
        let row = sqlx::query(
            r#"
            insert into operation_requests (
                id, kind, mint, target_wallet, target_token_account, amount, reason,
                external_reference, idempotency_key, status, requested_by, metadata
            )
            values ($1,$2,$3,$4,$5,$6::numeric,$7,$8,$9,$10,$11,$12)
            returning id, kind, mint, target_wallet, target_token_account, amount::text as amount, reason,
                      external_reference, idempotency_key, status, requested_by, approved_by,
                      tx_signature, failure_reason, metadata, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(request.kind.as_str())
        .bind(&request.mint)
        .bind(&request.target_wallet)
        .bind(&request.target_token_account)
        .bind(request.amount.map(|v| v.to_string()))
        .bind(&request.reason)
        .bind(&request.external_reference)
        .bind(&request.idempotency_key)
        .bind(OperationStatus::Requested.as_str())
        .bind(&request.requested_by)
        .bind(&request.metadata)
        .fetch_one(&self.pool)
        .await?;
        row_to_operation_request(row)
    }

    pub async fn get_operation_request(&self, id: Uuid) -> Result<Option<OperationRequest>> {
        let row = sqlx::query(
            r#"
            select id, kind, mint, target_wallet, target_token_account, amount::text as amount, reason,
                   external_reference, idempotency_key, status, requested_by, approved_by,
                   tx_signature, failure_reason, metadata, created_at, updated_at
            from operation_requests
            where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_operation_request).transpose()
    }

    pub async fn approve_operation(&self, id: Uuid, approved_by: &str) -> Result<Option<OperationRequest>> {
        let row = sqlx::query(
            r#"
            update operation_requests
            set status = $2, approved_by = $3, updated_at = now()
            where id = $1 and status = 'requested'
            returning id, kind, mint, target_wallet, target_token_account, amount::text as amount, reason,
                      external_reference, idempotency_key, status, requested_by, approved_by,
                      tx_signature, failure_reason, metadata, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(OperationStatus::Approved.as_str())
        .bind(approved_by)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_operation_request).transpose()
    }

    pub async fn mark_operation_status(
        &self,
        id: Uuid,
        status: OperationStatus,
        tx_signature: Option<&str>,
        failure_reason: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            update operation_requests
            set status = $2,
                tx_signature = coalesce($3, tx_signature),
                failure_reason = $4,
                updated_at = now()
            where id = $1
            "#,
        )
        .bind(id)
        .bind(status.as_str())
        .bind(tx_signature)
        .bind(failure_reason)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_operations_by_status(&self, status: OperationStatus, limit: i64) -> Result<Vec<OperationRequest>> {
        let rows = sqlx::query(
            r#"
            select id, kind, mint, target_wallet, target_token_account, amount::text as amount, reason,
                   external_reference, idempotency_key, status, requested_by, approved_by,
                   tx_signature, failure_reason, metadata, created_at, updated_at
            from operation_requests
            where status = $1
            order by created_at asc
            limit $2
            "#,
        )
        .bind(status.as_str())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_operation_request).collect()
    }

    pub async fn create_attempt(&self, attempt: &OperationAttempt) -> Result<()> {
        sqlx::query(
            r#"
            insert into operation_attempts (
                operation_id, attempt_number, status, signer_backend, tx_signature,
                rpc_endpoint, error_message, started_at, finished_at
            )
            values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            on conflict (operation_id, attempt_number) do update set
                status = excluded.status,
                tx_signature = excluded.tx_signature,
                rpc_endpoint = excluded.rpc_endpoint,
                error_message = excluded.error_message,
                finished_at = excluded.finished_at
            "#,
        )
        .bind(attempt.operation_id)
        .bind(attempt.attempt_number)
        .bind(attempt.status.as_str())
        .bind(&attempt.signer_backend)
        .bind(&attempt.tx_signature)
        .bind(&attempt.rpc_endpoint)
        .bind(&attempt.error_message)
        .bind(attempt.started_at)
        .bind(attempt.finished_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_attempts(&self, operation_id: Uuid) -> Result<Vec<OperationAttempt>> {
        let rows = sqlx::query(
            r#"
            select id, operation_id, attempt_number, status, signer_backend, tx_signature,
                   rpc_endpoint, error_message, started_at, finished_at
            from operation_attempts
            where operation_id = $1
            order by attempt_number asc
            "#,
        )
        .bind(operation_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_attempt).collect()
    }

    pub async fn create_webhook_endpoint(&self, endpoint: &CreateWebhookEndpoint) -> Result<WebhookEndpoint> {
        let row = sqlx::query(
            r#"
            insert into webhook_endpoints (id, name, url, secret, subscribed_event_types)
            values ($1,$2,$3,$4,$5)
            returning id, name, url, secret, subscribed_event_types, active, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&endpoint.name)
        .bind(&endpoint.url)
        .bind(&endpoint.secret)
        .bind(&endpoint.subscribed_event_types)
        .fetch_one(&self.pool)
        .await?;
        row_to_webhook_endpoint(row)
    }

    pub async fn list_webhook_endpoints(&self) -> Result<Vec<WebhookEndpoint>> {
        let rows = sqlx::query(
            r#"
            select id, name, url, secret, subscribed_event_types, active, created_at, updated_at
            from webhook_endpoints
            where active = true
            order by created_at asc
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_webhook_endpoint).collect()
    }

    pub async fn enqueue_webhook_delivery(&self, delivery: &WebhookDelivery) -> Result<()> {
        sqlx::query(
            r#"
            insert into webhook_deliveries (
                webhook_endpoint_id, source_event_key, event_type, payload, status,
                attempt_count, next_attempt_at, last_http_status, last_error, delivered_at, created_at
            )
            values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            on conflict (webhook_endpoint_id, source_event_key) do nothing
            "#,
        )
        .bind(delivery.webhook_endpoint_id)
        .bind(&delivery.source_event_key)
        .bind(&delivery.event_type)
        .bind(&delivery.payload)
        .bind(delivery.status.as_str())
        .bind(delivery.attempt_count)
        .bind(delivery.next_attempt_at)
        .bind(delivery.last_http_status)
        .bind(&delivery.last_error)
        .bind(delivery.delivered_at)
        .bind(delivery.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_due_deliveries(&self, before: DateTime<Utc>, limit: i64) -> Result<Vec<WebhookDelivery>> {
        let rows = sqlx::query(
            r#"
            select id, webhook_endpoint_id, source_event_key, event_type, payload, status,
                   attempt_count, next_attempt_at, last_http_status, last_error, delivered_at, created_at
            from webhook_deliveries
            where status in ('pending', 'failed') and coalesce(next_attempt_at, created_at) <= $1
            order by created_at asc
            limit $2
            "#,
        )
        .bind(before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_webhook_delivery).collect()
    }

    pub async fn mark_webhook_delivery(
        &self,
        id: i64,
        status: WebhookDeliveryStatus,
        attempt_count: i32,
        next_attempt_at: Option<DateTime<Utc>>,
        http_status: Option<i32>,
        last_error: Option<&str>,
        delivered_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            update webhook_deliveries
            set status = $2,
                attempt_count = $3,
                next_attempt_at = $4,
                last_http_status = $5,
                last_error = $6,
                delivered_at = $7
            where id = $1
            "#,
        )
        .bind(id)
        .bind(status.as_str())
        .bind(attempt_count)
        .bind(next_attempt_at)
        .bind(http_status)
        .bind(last_error)
        .bind(delivered_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn create_audit_export(&self, request: &CreateAuditExport) -> Result<AuditExport> {
        let row = sqlx::query(
            r#"
            insert into audit_exports (id, status, requested_by, filters)
            values ($1,$2,$3,$4)
            returning id, status, requested_by, filters, artifact_path, error_message, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(AuditExportStatus::Requested.as_str())
        .bind(&request.requested_by)
        .bind(&request.filters)
        .fetch_one(&self.pool)
        .await?;
        row_to_audit_export(row)
    }

    pub async fn list_audit_exports_by_status(&self, status: AuditExportStatus, limit: i64) -> Result<Vec<AuditExport>> {
        let rows = sqlx::query(
            r#"
            select id, status, requested_by, filters, artifact_path, error_message, created_at, updated_at
            from audit_exports
            where status = $1
            order by created_at asc
            limit $2
            "#,
        )
        .bind(status.as_str())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_audit_export).collect()
    }

    pub async fn mark_audit_export(
        &self,
        id: Uuid,
        status: AuditExportStatus,
        artifact_path: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            update audit_exports
            set status = $2,
                artifact_path = $3,
                error_message = $4,
                updated_at = now()
            where id = $1
            "#,
        )
        .bind(id)
        .bind(status.as_str())
        .bind(artifact_path)
        .bind(error_message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

pub fn health_report(name: &str, layer: &str) -> HealthReport {
    HealthReport {
        component: name.to_string(),
        layer: layer.to_string(),
        status: "ready".to_string(),
    }
}

fn row_to_chain_event(row: sqlx::postgres::PgRow) -> Result<ChainEvent> {
    Ok(ChainEvent {
        event_uid: row.try_get("event_uid")?,
        program_id: row.try_get("program_id")?,
        mint: row.try_get("mint")?,
        event_source: parse_event_source(&row.try_get::<String, _>("event_source")?)?,
        event_type: row.try_get("event_type")?,
        slot: row.try_get("slot")?,
        tx_signature: row.try_get("tx_signature")?,
        instruction_index: row.try_get("instruction_index")?,
        inner_instruction_index: row.try_get("inner_instruction_index")?,
        event_index: row.try_get("event_index")?,
        block_time: row.try_get("block_time")?,
        payload: row.try_get("payload")?,
    })
}

fn row_to_mint(row: sqlx::postgres::PgRow) -> Result<MintRecord> {
    Ok(MintRecord {
        mint: row.try_get("mint")?,
        preset: row.try_get("preset")?,
        authority: row.try_get("authority")?,
        name: row.try_get("name")?,
        symbol: row.try_get("symbol")?,
        uri: row.try_get("uri")?,
        decimals: row.try_get("decimals")?,
        enable_permanent_delegate: row.try_get("enable_permanent_delegate")?,
        enable_transfer_hook: row.try_get("enable_transfer_hook")?,
        default_account_frozen: row.try_get("default_account_frozen")?,
        paused: row.try_get("paused")?,
        total_minted: parse_numeric(&row.try_get::<String, _>("total_minted")?)?,
        total_burned: parse_numeric(&row.try_get::<String, _>("total_burned")?)?,
        created_at: row.try_get("created_at")?,
        last_changed_by: row.try_get("last_changed_by")?,
        last_changed_at: row.try_get("last_changed_at")?,
        indexed_slot: row.try_get("indexed_slot")?,
    })
}

fn row_to_blacklist_entry(row: sqlx::postgres::PgRow) -> Result<BlacklistEntryRecord> {
    Ok(BlacklistEntryRecord {
        mint: row.try_get("mint")?,
        wallet: row.try_get("wallet")?,
        reason: row.try_get("reason")?,
        blacklisted_by: row.try_get("blacklisted_by")?,
        blacklisted_at: row.try_get("blacklisted_at")?,
        active: row.try_get("active")?,
        removed_at: row.try_get("removed_at")?,
        indexed_slot: row.try_get("indexed_slot")?,
    })
}

fn row_to_minter_quota(row: sqlx::postgres::PgRow) -> Result<MinterQuotaRecord> {
    Ok(MinterQuotaRecord {
        mint: row.try_get("mint")?,
        minter: row.try_get("minter")?,
        quota: parse_numeric(&row.try_get::<String, _>("quota")?)?,
        minted: parse_numeric(&row.try_get::<String, _>("minted")?)?,
        active: row.try_get("active")?,
        updated_at: row.try_get("updated_at")?,
        indexed_slot: row.try_get("indexed_slot")?,
    })
}

fn row_to_operation_request(row: sqlx::postgres::PgRow) -> Result<OperationRequest> {
    Ok(OperationRequest {
        id: row.try_get("id")?,
        kind: parse_operation_kind(&row.try_get::<String, _>("kind")?)?,
        mint: row.try_get("mint")?,
        target_wallet: row.try_get("target_wallet")?,
        target_token_account: row.try_get("target_token_account")?,
        amount: row
            .try_get::<Option<String>, _>("amount")?
            .map(|value| parse_numeric(&value))
            .transpose()?,
        reason: row.try_get("reason")?,
        external_reference: row.try_get("external_reference")?,
        idempotency_key: row.try_get("idempotency_key")?,
        status: parse_operation_status(&row.try_get::<String, _>("status")?)?,
        requested_by: row.try_get("requested_by")?,
        approved_by: row.try_get("approved_by")?,
        tx_signature: row.try_get("tx_signature")?,
        failure_reason: row.try_get("failure_reason")?,
        metadata: row.try_get("metadata")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn row_to_attempt(row: sqlx::postgres::PgRow) -> Result<OperationAttempt> {
    Ok(OperationAttempt {
        id: row.try_get("id")?,
        operation_id: row.try_get("operation_id")?,
        attempt_number: row.try_get("attempt_number")?,
        status: parse_attempt_status(&row.try_get::<String, _>("status")?)?,
        signer_backend: row.try_get("signer_backend")?,
        tx_signature: row.try_get("tx_signature")?,
        rpc_endpoint: row.try_get("rpc_endpoint")?,
        error_message: row.try_get("error_message")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
    })
}

fn row_to_webhook_endpoint(row: sqlx::postgres::PgRow) -> Result<WebhookEndpoint> {
    Ok(WebhookEndpoint {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        url: row.try_get("url")?,
        secret: row.try_get("secret")?,
        subscribed_event_types: row.try_get("subscribed_event_types")?,
        active: row.try_get("active")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn row_to_webhook_delivery(row: sqlx::postgres::PgRow) -> Result<WebhookDelivery> {
    Ok(WebhookDelivery {
        id: row.try_get("id")?,
        webhook_endpoint_id: row.try_get("webhook_endpoint_id")?,
        source_event_key: row.try_get("source_event_key")?,
        event_type: row.try_get("event_type")?,
        payload: row.try_get("payload")?,
        status: parse_webhook_status(&row.try_get::<String, _>("status")?)?,
        attempt_count: row.try_get("attempt_count")?,
        next_attempt_at: row.try_get("next_attempt_at")?,
        last_http_status: row.try_get("last_http_status")?,
        last_error: row.try_get("last_error")?,
        delivered_at: row.try_get("delivered_at")?,
        created_at: row.try_get("created_at")?,
    })
}

fn row_to_audit_export(row: sqlx::postgres::PgRow) -> Result<AuditExport> {
    Ok(AuditExport {
        id: row.try_get("id")?,
        status: parse_audit_export_status(&row.try_get::<String, _>("status")?)?,
        requested_by: row.try_get("requested_by")?,
        filters: row.try_get("filters")?,
        artifact_path: row.try_get("artifact_path")?,
        error_message: row.try_get("error_message")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn parse_numeric(value: &str) -> Result<i128> {
    value.parse::<i128>().context("failed to parse numeric value")
}

fn parse_event_source(value: &str) -> Result<EventSource> {
    match value {
        "anchor_event" => Ok(EventSource::AnchorEvent),
        "instruction" => Ok(EventSource::Instruction),
        "synthetic_transfer_hook" => Ok(EventSource::SyntheticTransferHook),
        _ => anyhow::bail!("unknown event source {value}"),
    }
}

fn parse_operation_kind(value: &str) -> Result<OperationKind> {
    match value {
        "mint" => Ok(OperationKind::Mint),
        "burn" => Ok(OperationKind::Burn),
        "blacklist_add" => Ok(OperationKind::BlacklistAdd),
        "blacklist_remove" => Ok(OperationKind::BlacklistRemove),
        "freeze" => Ok(OperationKind::Freeze),
        "thaw" => Ok(OperationKind::Thaw),
        "seize" => Ok(OperationKind::Seize),
        _ => anyhow::bail!("unknown operation kind {value}"),
    }
}

fn parse_operation_status(value: &str) -> Result<OperationStatus> {
    match value {
        "requested" => Ok(OperationStatus::Requested),
        "approved" => Ok(OperationStatus::Approved),
        "signing" => Ok(OperationStatus::Signing),
        "submitted" => Ok(OperationStatus::Submitted),
        "finalized" => Ok(OperationStatus::Finalized),
        "failed" => Ok(OperationStatus::Failed),
        "cancelled" => Ok(OperationStatus::Cancelled),
        _ => anyhow::bail!("unknown operation status {value}"),
    }
}

fn parse_attempt_status(value: &str) -> Result<AttemptStatus> {
    match value {
        "started" => Ok(AttemptStatus::Started),
        "signed" => Ok(AttemptStatus::Signed),
        "submitted" => Ok(AttemptStatus::Submitted),
        "confirmed" => Ok(AttemptStatus::Confirmed),
        "failed" => Ok(AttemptStatus::Failed),
        _ => anyhow::bail!("unknown attempt status {value}"),
    }
}

fn parse_webhook_status(value: &str) -> Result<WebhookDeliveryStatus> {
    match value {
        "pending" => Ok(WebhookDeliveryStatus::Pending),
        "delivering" => Ok(WebhookDeliveryStatus::Delivering),
        "delivered" => Ok(WebhookDeliveryStatus::Delivered),
        "failed" => Ok(WebhookDeliveryStatus::Failed),
        "dead_letter" => Ok(WebhookDeliveryStatus::DeadLetter),
        _ => anyhow::bail!("unknown webhook status {value}"),
    }
}

fn parse_audit_export_status(value: &str) -> Result<AuditExportStatus> {
    match value {
        "requested" => Ok(AuditExportStatus::Requested),
        "processing" => Ok(AuditExportStatus::Processing),
        "completed" => Ok(AuditExportStatus::Completed),
        "failed" => Ok(AuditExportStatus::Failed),
        _ => anyhow::bail!("unknown audit export status {value}"),
    }
}
