use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    sync::Arc,
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use serde_json::{json, Value};
use sss_api::{
    AuthorityKeypairSigner, build_router, AppState, AuditExportWorker, OperationExecutorWorker,
    WebhookRetryWorker,
};
use sss_db::Store;
use sss_domain::{
    AuditExport, AuditExporter, CreateAuditExport, CreateOperationRequest, CreateWebhookEndpoint,
    MintRecord, OperationExecutionResult, OperationKind, OperationRequest, SignerBackend,
    WebhookDelivery, WebhookDeliveryStatus, WebhookDispatcher, WorkerError,
};
use tempfile::TempDir;
use tower::util::ServiceExt;

struct PostgresHarness {
    _dir: Option<TempDir>,
    process: Option<Child>,
    database_url: String,
    admin_url: Option<String>,
    db_name: Option<String>,
}

impl PostgresHarness {
    fn database_url(&self) -> String {
        self.database_url.clone()
    }
}

impl Drop for PostgresHarness {
    fn drop(&mut self) {
        if let Some(process) = &mut self.process {
            let _ = process.kill();
            let _ = process.wait();
        }
        if let (Some(admin_url), Some(db_name)) = (&self.admin_url, &self.db_name) {
            let _ = Command::new("psql")
                .arg(admin_url)
                .arg("-c")
                .arg(format!("drop database if exists {db_name}"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

fn find_free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn database_url_for(admin_url: &str, db_name: &str) -> Result<String> {
    let (prefix, _) = admin_url
        .rsplit_once('/')
        .context("TEST_DATABASE_ADMIN_URL must include a database name")?;
    Ok(format!("{prefix}/{db_name}"))
}

fn next_db_name(prefix: &str) -> String {
    let suffix = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_{}_{}", std::process::id(), suffix)
}

fn start_postgres() -> Result<PostgresHarness> {
    let db_name = next_db_name("sss_test");
    if let Ok(admin_url) = std::env::var("TEST_DATABASE_ADMIN_URL") {
        let createdb = Command::new("psql")
            .arg(&admin_url)
            .arg("-c")
            .arg(format!("create database {db_name}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("failed to create test database")?;
        anyhow::ensure!(createdb.success(), "create database failed");

        return Ok(PostgresHarness {
            _dir: None,
            process: None,
            database_url: database_url_for(&admin_url, &db_name)?,
            admin_url: Some(admin_url),
            db_name: Some(db_name),
        });
    }

    let dir = TempDir::new()?;
    let port = find_free_port()?;

    let initdb = Command::new("initdb")
        .arg("-A")
        .arg("trust")
        .arg("-U")
        .arg("postgres")
        .arg("--set")
        .arg("dynamic_shared_memory_type=none")
        .arg("-D")
        .arg(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to run initdb")?;
    anyhow::ensure!(initdb.success(), "initdb failed");

    let process = Command::new("postgres")
        .arg("-D")
        .arg(dir.path())
        .arg("-p")
        .arg(port.to_string())
        .arg("-c")
        .arg("listen_addresses=127.0.0.1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to start postgres")?;

    for _ in 0..50 {
        let status = Command::new("psql")
            .arg("-h")
            .arg("127.0.0.1")
            .arg("-p")
            .arg(port.to_string())
            .arg("-U")
            .arg("postgres")
            .arg("-d")
            .arg("postgres")
            .arg("-c")
            .arg("select 1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Ok(status) = status {
            if status.success() {
                break;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    let createdb = Command::new("psql")
        .arg("-h")
        .arg("127.0.0.1")
        .arg("-p")
        .arg(port.to_string())
        .arg("-U")
        .arg("postgres")
        .arg("-d")
        .arg("postgres")
        .arg("-c")
        .arg(format!("create database {db_name}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to create test database")?;
    anyhow::ensure!(createdb.success(), "create database failed");

    Ok(PostgresHarness {
        _dir: Some(dir),
        process: Some(process),
        database_url: format!("postgres://postgres@127.0.0.1:{port}/{db_name}"),
        admin_url: None,
        db_name: None,
    })
}

async fn seeded_store() -> Result<(PostgresHarness, Store)> {
    let harness = start_postgres()?;
    let store = Store::connect(&harness.database_url()).await?;
    store.migrate().await?;
    store
        .upsert_mint(&MintRecord {
            mint: "mint-1".to_string(),
            preset: "SSS-2".to_string(),
            authority: "auth-1".to_string(),
            name: "Regulated USD".to_string(),
            symbol: "RUSD".to_string(),
            uri: "https://example.com".to_string(),
            decimals: 6,
            enable_permanent_delegate: true,
            enable_transfer_hook: true,
            default_account_frozen: true,
            paused: false,
            total_minted: 0,
            total_burned: 0,
            created_at: Utc::now(),
            last_changed_by: "auth-1".to_string(),
            last_changed_at: Utc::now(),
            indexed_slot: 1,
        })
        .await?;
    Ok((harness, store))
}

struct MockSigner;

#[async_trait]
impl SignerBackend for MockSigner {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn execute(&self, operation: &OperationRequest) -> Result<OperationExecutionResult, WorkerError> {
        Ok(OperationExecutionResult {
            operation_id: operation.id,
            tx_signature: format!("sig-{}", operation.id),
        })
    }
}

struct MockDispatcher {
    fail: bool,
}

#[async_trait]
impl WebhookDispatcher for MockDispatcher {
    async fn deliver(
        &self,
        _endpoint: &sss_domain::WebhookEndpoint,
        _delivery: &WebhookDelivery,
    ) -> Result<Option<i32>, WorkerError> {
        if self.fail {
            Err(WorkerError::Dependency("dispatch failed".to_string()))
        } else {
            Ok(Some(200))
        }
    }
}

struct MockExporter;

#[async_trait]
impl AuditExporter for MockExporter {
    async fn export(&self, export: &AuditExport) -> Result<String, WorkerError> {
        Ok(format!("exports/{}.json", export.id))
    }
}

#[tokio::test]
async fn api_routes_cover_core_paths() -> Result<()> {
    let (_harness, store) = seeded_store().await?;
    store
        .insert_chain_event(&sss_domain::ChainEvent {
            event_uid: "evt-1".to_string(),
            program_id: "program".to_string(),
            mint: Some("mint-1".to_string()),
            event_source: sss_domain::EventSource::AnchorEvent,
            event_type: "TokensMinted".to_string(),
            slot: 10,
            tx_signature: "sig-1".to_string(),
            instruction_index: 0,
            inner_instruction_index: None,
            event_index: Some(0),
            block_time: Some(Utc::now()),
            payload: json!({"mint":"mint-1","authority":"auth-1","amount":"100"}),
        })
        .await?;
    store
        .upsert_blacklist_entry(&sss_domain::BlacklistEntryRecord {
            mint: "mint-1".to_string(),
            wallet: "wallet-1".to_string(),
            reason: "screened".to_string(),
            blacklisted_by: "auth-1".to_string(),
            blacklisted_at: Utc::now(),
            active: true,
            removed_at: None,
            indexed_slot: 11,
        })
        .await?;

    let app = build_router(AppState { store: store.clone() });

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/readyz").body(Body::empty()).unwrap())
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/v1/mints").body(Body::empty()).unwrap())
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/v1/mints/mint-1").body(Body::empty()).unwrap())
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/v1/mints/mint-1/events").body(Body::empty()).unwrap())
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/v1/mints/mint-1/blacklist").body(Body::empty()).unwrap())
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    for (uri, kind) in [
        ("/v1/mint-requests", OperationKind::Mint),
        ("/v1/burn-requests", OperationKind::Burn),
        ("/v1/compliance/blacklists", OperationKind::BlacklistAdd),
        ("/v1/compliance/freeze", OperationKind::Freeze),
        ("/v1/compliance/thaw", OperationKind::Thaw),
        ("/v1/compliance/seize", OperationKind::Seize),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "mint": "mint-1",
                            "target_wallet": "wallet-1",
                            "target_token_account": "ata-1",
                            "amount": 100,
                            "reason": "ops",
                            "external_reference": "ref-1",
                            "idempotency_key": format!("{}-1", kind.as_str()),
                            "requested_by": "tester",
                            "metadata": {}
                        }))?,
                    ))
                    .unwrap(),
            )
            .await?;
        assert_eq!(response.status(), StatusCode::CREATED, "{uri}");
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/v1/compliance/blacklists/mint-1/wallet-2")
                .body(Body::empty())
                .unwrap(),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CREATED);

    let operation = store
        .create_operation_request(&CreateOperationRequest {
            kind: OperationKind::Mint,
            mint: "mint-1".to_string(),
            target_wallet: Some("wallet-3".to_string()),
            target_token_account: Some("ata-3".to_string()),
            amount: Some(500),
            reason: Some("seed".to_string()),
            external_reference: None,
            idempotency_key: "seed-op".to_string(),
            requested_by: "tester".to_string(),
            metadata: json!({}),
        })
        .await?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/operations/{}/approve", operation.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&json!({"approved_by":"ops"}))?))
                .unwrap(),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/operations/{}/execute", operation.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/operations/{}", operation.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/webhooks/endpoints")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&CreateWebhookEndpoint {
                    name: "ops".to_string(),
                    url: "https://example.com/hook".to_string(),
                    secret: "secret".to_string(),
                    subscribed_event_types: vec!["TokensMinted".to_string()],
                })?))
                .unwrap(),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/compliance/audit-exports")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&json!({
                    "requested_by":"tester",
                    "filters": {"mint":"mint-1"}
                }))?))
                .unwrap(),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CREATED);

    Ok(())
}

#[tokio::test]
async fn operation_worker_submits_approved_operations() -> Result<()> {
    let (_harness, store) = seeded_store().await?;
    let operation = store
        .create_operation_request(&CreateOperationRequest {
            kind: OperationKind::Mint,
            mint: "mint-1".to_string(),
            target_wallet: Some("wallet-4".to_string()),
            target_token_account: Some("ata-4".to_string()),
            amount: Some(100),
            reason: Some("worker".to_string()),
            external_reference: None,
            idempotency_key: "worker-op".to_string(),
            requested_by: "tester".to_string(),
            metadata: json!({}),
        })
        .await?;
    store.approve_operation(operation.id, "ops").await?;

    let worker = OperationExecutorWorker {
        store: store.clone(),
        signer: Arc::new(MockSigner),
        poll_limit: 10,
    };
    let processed = worker.run_once().await?;
    assert_eq!(processed, 1);

    let updated = store.get_operation_request(operation.id).await?.unwrap();
    assert_eq!(updated.status, sss_domain::OperationStatus::Submitted);
    assert!(updated.tx_signature.is_some());
    let attempts = store.list_attempts(operation.id).await?;
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].status, sss_domain::AttemptStatus::Submitted);
    Ok(())
}

#[tokio::test]
async fn webhook_and_audit_workers_update_state() -> Result<()> {
    let (_harness, store) = seeded_store().await?;
    let endpoint = store
        .create_webhook_endpoint(&CreateWebhookEndpoint {
            name: "ops".to_string(),
            url: "https://example.com".to_string(),
            secret: "secret".to_string(),
            subscribed_event_types: vec!["TokensMinted".to_string()],
        })
        .await?;
    store
        .enqueue_webhook_delivery(&WebhookDelivery {
            id: None,
            webhook_endpoint_id: endpoint.id,
            source_event_key: "evt-1".to_string(),
            event_type: "TokensMinted".to_string(),
            payload: json!({"mint":"mint-1"}),
            status: WebhookDeliveryStatus::Pending,
            attempt_count: 0,
            next_attempt_at: None,
            last_http_status: None,
            last_error: None,
            delivered_at: None,
            created_at: Utc::now(),
        })
        .await?;

    let export = store
        .create_audit_export(&CreateAuditExport {
            requested_by: "tester".to_string(),
            filters: Value::Null,
        })
        .await?;

    let webhook_worker = WebhookRetryWorker {
        store: store.clone(),
        dispatcher: Arc::new(MockDispatcher { fail: false }),
        poll_limit: 10,
        max_attempts: 3,
    };
    let audit_worker = AuditExportWorker {
        store: store.clone(),
        exporter: Arc::new(MockExporter),
        poll_limit: 10,
    };

    assert_eq!(webhook_worker.run_once().await?, 1);
    assert_eq!(audit_worker.run_once().await?, 1);

    let deliveries = store.list_due_deliveries(Utc::now(), 10).await?;
    assert!(deliveries.is_empty());
    let exports = store
        .list_audit_exports_by_status(sss_domain::AuditExportStatus::Completed, 10)
        .await?;
    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].id, export.id);
    Ok(())
}

/// Full mint flow on DevNet: request → approve → execute → worker submits tx.
/// Requires: SSS_DEVNET_E2E=1, SOLANA_RPC_URL, SSS_STABLECOIN_PROGRAM_ID,
/// SSS_AUTHORITY_SECRET_KEY (or SSS_AUTHORITY_KEYPAIR), SSS_DEVNET_MINT,
/// SSS_DEVNET_TARGET_ATA or SSS_DEVNET_TARGET_WALLET.
/// If DATABASE_URL is not set, starts an ephemeral Postgres (same as other integration tests).
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn devnet_e2e_mint_execution() -> Result<()> {
    if std::env::var("SSS_DEVNET_E2E").ok().as_deref() != Some("1") {
        return Ok(());
    }
    let (_pg_harness, database_url) = match std::env::var("DATABASE_URL") {
        Ok(url) => (None, url),
        Err(_) => {
            let harness = start_postgres()?;
            let url = harness.database_url();
            (Some(harness), url)
        }
    };
    let _ = std::env::var("SOLANA_RPC_URL").context("SOLANA_RPC_URL required")?;
    let mint_pubkey = std::env::var("SSS_DEVNET_MINT").context("SSS_DEVNET_MINT required")?;
    let target_ata = std::env::var("SSS_DEVNET_TARGET_ATA").ok();
    let target_wallet = std::env::var("SSS_DEVNET_TARGET_WALLET").ok();
    if target_ata.is_none() && target_wallet.is_none() {
        anyhow::bail!("set SSS_DEVNET_TARGET_ATA or SSS_DEVNET_TARGET_WALLET");
    }
    let signer = AuthorityKeypairSigner::from_env()
        .map_err(|e| anyhow::anyhow!("AuthorityKeypairSigner::from_env: {}", e))?;

    let store = Store::connect(&database_url).await?;
    store.migrate().await?;

    store
        .upsert_mint(&MintRecord {
            mint: mint_pubkey.clone(),
            preset: "SSS-1".to_string(),
            authority: signer.name().to_string(),
            name: "E2E USD".to_string(),
            symbol: "E2E".to_string(),
            uri: "https://example.com/e2e".to_string(),
            decimals: 6,
            enable_permanent_delegate: false,
            enable_transfer_hook: false,
            default_account_frozen: false,
            paused: false,
            total_minted: 0,
            total_burned: 0,
            created_at: Utc::now(),
            last_changed_by: signer.name().to_string(),
            last_changed_at: Utc::now(),
            indexed_slot: 0,
        })
        .await?;

    let operation = store
        .create_operation_request(&CreateOperationRequest {
            kind: OperationKind::Mint,
            mint: mint_pubkey.clone(),
            target_wallet: target_wallet.clone(),
            target_token_account: target_ata.clone(),
            amount: Some(1_000_000),
            reason: Some("devnet-e2e".to_string()),
            external_reference: None,
            idempotency_key: format!("e2e-{}", std::process::id()),
            requested_by: "e2e-test".to_string(),
            metadata: json!({}),
        })
        .await?;
    store.approve_operation(operation.id, "e2e").await?;

    let worker = OperationExecutorWorker {
        store: store.clone(),
        signer: Arc::new(signer),
        poll_limit: 10,
    };
    for _ in 0..20 {
        let n = worker.run_once().await?;
        if n > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let updated = store.get_operation_request(operation.id).await?.context("operation gone")?;
    assert_eq!(
        updated.status,
        sss_domain::OperationStatus::Submitted,
        "expected Submitted, got {:?}; tx_sig = {:?}",
        updated.status,
        updated.tx_signature
    );
    assert!(
        updated.tx_signature.is_some(),
        "operation should have tx_signature"
    );
    Ok(())
}
