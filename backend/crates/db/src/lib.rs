use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use sss_domain::{
    CreateLifecycleRequest, CreateWebhookSubscription, EventFilters, EventRecord, EventSort,
    HealthReport, InsertEvent, LifecycleRequest, LifecycleStatus, LifecycleRequestType,
    SortOrder, WebhookDelivery, WebhookDeliveryStatus, WebhookSubscription,
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

    pub async fn upsert_indexer_state(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        sqlx::query(
            r#"
            insert into indexer_state (key, value)
            values ($1, $2)
            on conflict (key) do update set value = excluded.value
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_indexer_state(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query(
            r#"select value from indexer_state where key = $1"#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.get("value")))
    }

    pub async fn insert_event(&self, event: &InsertEvent) -> Result<()> {
        sqlx::query(
            r#"
            insert into events (event_type, program_id, mint, tx_signature, slot, block_time, instruction_index, data)
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            on conflict (tx_signature, instruction_index, event_type) do nothing
            "#,
        )
        .bind(&event.event_type)
        .bind(&event.program_id)
        .bind(&event.mint)
        .bind(&event.tx_signature)
        .bind(event.slot)
        .bind(event.block_time)
        .bind(event.instruction_index)
        .bind(&event.data)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_events(
        &self,
        mint: Option<&str>,
        filters: &EventFilters,
        sort: EventSort,
        order: SortOrder,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<EventRecord>, i64)> {
        let limit = limit.clamp(1, 500);
        let (order_col, order_dir) = (
            sort.as_str(),
            order.as_str(),
        );

        let total: i64 = sqlx::query_scalar::<_, i64>(
            r#"
            select count(*)::bigint from events
            where ($1::text is null or mint = $1)
              and ($2::text is null or event_type = $2)
              and ($3::text is null or program_id = $3)
              and ($4::text is null or tx_signature = $4)
              and ($5::bigint is null or slot >= $5)
              and ($6::bigint is null or slot <= $6)
              and ($7::timestamptz is null or block_time >= $7)
              and ($8::timestamptz is null or block_time <= $8)
            "#,
        )
        .bind(mint)
        .bind(filters.event_type.as_deref())
        .bind(filters.program_id.as_deref())
        .bind(filters.tx_signature.as_deref())
        .bind(filters.slot_min)
        .bind(filters.slot_max)
        .bind(filters.from)
        .bind(filters.to)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query(
            &format!(
                r#"
                select id, event_type, program_id, mint, tx_signature, slot, block_time, instruction_index, data, created_at
                from events
                where ($1::text is null or mint = $1)
                  and ($2::text is null or event_type = $2)
                  and ($3::text is null or program_id = $3)
                  and ($4::text is null or tx_signature = $4)
                  and ($5::bigint is null or slot >= $5)
                  and ($6::bigint is null or slot <= $6)
                  and ($7::timestamptz is null or block_time >= $7)
                  and ($8::timestamptz is null or block_time <= $8)
                order by {} {}, id desc
                limit $9 offset $10
                "#,
                order_col, order_dir
            ),
        )
        .bind(mint)
        .bind(filters.event_type.as_deref())
        .bind(filters.program_id.as_deref())
        .bind(filters.tx_signature.as_deref())
        .bind(filters.slot_min)
        .bind(filters.slot_max)
        .bind(filters.from)
        .bind(filters.to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let events = rows
            .into_iter()
            .map(|row| {
                Ok(EventRecord {
                    id: row.try_get("id")?,
                    event_type: row.try_get("event_type")?,
                    program_id: row.try_get("program_id")?,
                    mint: row.try_get("mint")?,
                    tx_signature: row.try_get("tx_signature")?,
                    slot: row.try_get("slot")?,
                    block_time: row.try_get("block_time")?,
                    instruction_index: row.try_get("instruction_index")?,
                    data: row.try_get("data")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((events, total))
    }

    pub async fn create_lifecycle_request(
        &self,
        id: &str,
        request: &CreateLifecycleRequest,
    ) -> Result<LifecycleRequest> {
        let row = sqlx::query(
            r#"
            insert into lifecycle_requests (id, type, status, mint, recipient, token_account, amount, minter, reason, idempotency_key, requested_by)
            values ($1, $2, 'requested', $3, $4, $5, $6::numeric, $7, $8, $9, $10)
            returning id, type, status, mint, recipient, token_account, amount::text as amount, minter, reason, idempotency_key, requested_by, approved_by, tx_signature, error, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(request.type_.as_str())
        .bind(&request.mint)
        .bind(&request.recipient)
        .bind(&request.token_account)
        .bind(request.amount.to_string())
        .bind(&request.minter)
        .bind(&request.reason)
        .bind(&request.idempotency_key)
        .bind(&request.requested_by)
        .fetch_one(&self.pool)
        .await?;
        row_to_lifecycle_request(row)
    }

    pub async fn get_lifecycle_request(&self, id: &str) -> Result<Option<LifecycleRequest>> {
        let row = sqlx::query(
            r#"
            select id, type, status, mint, recipient, token_account, amount::text as amount, minter, reason, idempotency_key, requested_by, approved_by, tx_signature, error, created_at, updated_at
            from lifecycle_requests where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_lifecycle_request).transpose()
    }

    pub async fn approve_lifecycle_request(
        &self,
        id: &str,
        approved_by: &str,
    ) -> Result<Option<LifecycleRequest>> {
        let row = sqlx::query(
            r#"
            update lifecycle_requests
            set status = 'approved', approved_by = $2, updated_at = now()
            where id = $1 and status = 'requested'
            returning id, type, status, mint, recipient, token_account, amount::text as amount, minter, reason, idempotency_key, requested_by, approved_by, tx_signature, error, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(approved_by)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_lifecycle_request).transpose()
    }

    pub async fn mark_lifecycle_status(
        &self,
        id: &str,
        status: LifecycleStatus,
        tx_signature: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            update lifecycle_requests
            set status = $2, tx_signature = coalesce($3, tx_signature), error = $4, updated_at = now()
            where id = $1
            "#,
        )
        .bind(id)
        .bind(status.as_str())
        .bind(tx_signature)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_lifecycle_requests_by_status(
        &self,
        status: LifecycleStatus,
        limit: i64,
    ) -> Result<Vec<LifecycleRequest>> {
        let rows = sqlx::query(
            r#"
            select id, type, status, mint, recipient, token_account, amount::text as amount, minter, reason, idempotency_key, requested_by, approved_by, tx_signature, error, created_at, updated_at
            from lifecycle_requests where status = $1 order by created_at asc limit $2
            "#,
        )
        .bind(status.as_str())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_lifecycle_request).collect()
    }

    pub async fn create_webhook_subscription(
        &self,
        sub: &CreateWebhookSubscription,
    ) -> Result<WebhookSubscription> {
        let row = sqlx::query(
            r#"
            insert into webhook_subscriptions (name, url, events, secret)
            values ($1, $2, $3, $4)
            returning id, name, url, events, secret, active, created_at, updated_at
            "#,
        )
        .bind(&sub.name)
        .bind(&sub.url)
        .bind(&sub.events)
        .bind(&sub.secret)
        .fetch_one(&self.pool)
        .await?;
        row_to_webhook_subscription(row)
    }

    pub async fn list_webhook_subscriptions(&self) -> Result<Vec<WebhookSubscription>> {
        let rows = sqlx::query(
            r#"
            select id, name, url, events, secret, active, created_at, updated_at
            from webhook_subscriptions where active = true order by created_at asc
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_webhook_subscription).collect()
    }

    pub async fn enqueue_webhook_delivery(
        &self,
        subscription_id: Uuid,
        event_id: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            insert into webhook_deliveries (subscription_id, event_id, status)
            values ($1, $2, 'pending')
            on conflict (subscription_id, event_id) do nothing
            "#,
        )
        .bind(subscription_id)
        .bind(event_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_due_webhook_deliveries(
        &self,
        before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<WebhookDelivery>> {
        let rows = sqlx::query(
            r#"
            select id, subscription_id, event_id, status, attempts, max_attempts, last_attempt_at, next_retry_at, response_code, error, created_at
            from webhook_deliveries
            where status in ('pending', 'failed') and coalesce(next_retry_at, created_at) <= $1
            order by created_at asc limit $2
            "#,
        )
        .bind(before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_webhook_delivery).collect()
    }

    pub async fn get_event(&self, id: i64) -> Result<Option<EventRecord>> {
        let row = sqlx::query(
            r#"
            select id, event_type, program_id, mint, tx_signature, slot, block_time, instruction_index, data, created_at
            from events where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| {
            Ok(EventRecord {
                id: r.try_get("id")?,
                event_type: r.try_get("event_type")?,
                program_id: r.try_get("program_id")?,
                mint: r.try_get("mint")?,
                tx_signature: r.try_get("tx_signature")?,
                slot: r.try_get("slot")?,
                block_time: r.try_get("block_time")?,
                instruction_index: r.try_get("instruction_index")?,
                data: r.try_get("data")?,
                created_at: r.try_get("created_at")?,
            })
        }).transpose()
    }

    pub async fn mark_webhook_delivery(
        &self,
        id: i64,
        status: WebhookDeliveryStatus,
        attempts: i32,
        next_retry_at: Option<DateTime<Utc>>,
        response_code: Option<i32>,
        error: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            update webhook_deliveries
            set status = $2, attempts = $3, last_attempt_at = now(), next_retry_at = $4, response_code = $5, error = $6
            where id = $1
            "#,
        )
        .bind(id)
        .bind(status.as_str())
        .bind(attempts)
        .bind(next_retry_at)
        .bind(response_code)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_audit_trail(
        &self,
        action: &str,
        actor: &str,
        target: Option<&str>,
        mint: Option<&str>,
        request_id: Option<&str>,
        details: Option<&serde_json::Value>,
        tx_signature: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            insert into audit_trail (action, actor, target, mint, request_id, details, tx_signature)
            values ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(action)
        .bind(actor)
        .bind(target)
        .bind(mint)
        .bind(request_id)
        .bind(details)
        .bind(tx_signature)
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

fn row_to_lifecycle_request(row: sqlx::postgres::PgRow) -> Result<LifecycleRequest> {
    let type_str: String = row.try_get("type")?;
    let type_ = match type_str.as_str() {
        "mint" => LifecycleRequestType::Mint,
        "burn" => LifecycleRequestType::Burn,
        _ => anyhow::bail!("unknown lifecycle type {type_str}"),
    };
    let status_str: String = row.try_get("status")?;
    let status = parse_lifecycle_status(&status_str)?;
    let amount_str: String = row.try_get("amount")?;
    let amount = amount_str.parse::<i128>().context("parse amount")?;
    Ok(LifecycleRequest {
        id: row.try_get("id")?,
        type_,
        status,
        mint: row.try_get("mint")?,
        recipient: row.try_get("recipient")?,
        token_account: row.try_get("token_account")?,
        amount,
        minter: row.try_get("minter")?,
        reason: row.try_get("reason")?,
        idempotency_key: row.try_get("idempotency_key")?,
        requested_by: row.try_get("requested_by")?,
        approved_by: row.try_get("approved_by")?,
        tx_signature: row.try_get("tx_signature")?,
        error: row.try_get("error")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn parse_lifecycle_status(s: &str) -> Result<LifecycleStatus> {
    match s {
        "requested" => Ok(LifecycleStatus::Requested),
        "approved" => Ok(LifecycleStatus::Approved),
        "signing" => Ok(LifecycleStatus::Signing),
        "submitted" => Ok(LifecycleStatus::Submitted),
        "finalized" => Ok(LifecycleStatus::Finalized),
        "failed" => Ok(LifecycleStatus::Failed),
        "cancelled" => Ok(LifecycleStatus::Cancelled),
        _ => anyhow::bail!("unknown lifecycle status {s}"),
    }
}

fn row_to_webhook_subscription(row: sqlx::postgres::PgRow) -> Result<WebhookSubscription> {
    Ok(WebhookSubscription {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        url: row.try_get("url")?,
        events: row.try_get("events")?,
        secret: row.try_get("secret")?,
        active: row.try_get("active")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn row_to_webhook_delivery(row: sqlx::postgres::PgRow) -> Result<WebhookDelivery> {
    let status_str: String = row.try_get("status")?;
    let status = match status_str.as_str() {
        "pending" => WebhookDeliveryStatus::Pending,
        "delivering" => WebhookDeliveryStatus::Delivering,
        "delivered" => WebhookDeliveryStatus::Delivered,
        "failed" => WebhookDeliveryStatus::Failed,
        "dead_letter" => WebhookDeliveryStatus::DeadLetter,
        _ => anyhow::bail!("unknown webhook delivery status {status_str}"),
    };
    Ok(WebhookDelivery {
        id: row.try_get("id")?,
        subscription_id: row.try_get("subscription_id")?,
        event_id: row.try_get("event_id")?,
        status,
        attempts: row.try_get("attempts")?,
        max_attempts: row.try_get("max_attempts")?,
        last_attempt_at: row.try_get("last_attempt_at")?,
        next_retry_at: row.try_get("next_retry_at")?,
        response_code: row.try_get("response_code")?,
        error: row.try_get("error")?,
        created_at: row.try_get("created_at")?,
    })
}