use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use sss_db::Store;
use sss_domain::{ChainEvent, EventSource};
use sss_indexer::{IndexerConfig, IndexerService};
use tempfile::TempDir;

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
    let db_name = next_db_name("sss_indexer_test");
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

#[tokio::test]
async fn indexer_ingests_events_into_projections() -> Result<()> {
    let harness = start_postgres()?;
    let service = IndexerService::new(IndexerConfig {
        database_url: harness.database_url(),
        rpc_url: "https://api.devnet.solana.com".to_string(),
        stablecoin_program_id: "stablecoin".to_string(),
        transfer_hook_program_id: "transfer-hook".to_string(),
        start_slot: 0,
        disable_block_subscribe: true,
    })
    .await?;

    service
        .ingest_chain_event(&ChainEvent {
            event_uid: "evt-init".to_string(),
            program_id: "stablecoin".to_string(),
            mint: Some("mint-1".to_string()),
            event_source: EventSource::AnchorEvent,
            event_type: "StablecoinInitialized".to_string(),
            slot: 1,
            tx_signature: "sig-1".to_string(),
            instruction_index: 0,
            inner_instruction_index: None,
            event_index: Some(0),
            block_time: Some(Utc::now()),
            payload: json!({
                "mint":"mint-1",
                "authority":"auth-1",
                "preset":"SSS-2",
                "name":"Regulated USD",
                "symbol":"RUSD",
                "uri":"https://example.com",
                "decimals": 6,
                "enable_permanent_delegate": true,
                "enable_transfer_hook": true,
                "default_account_frozen": true
            }),
        })
        .await?;
    service
        .ingest_chain_event(&ChainEvent {
            event_uid: "evt-blacklist".to_string(),
            program_id: "stablecoin".to_string(),
            mint: Some("mint-1".to_string()),
            event_source: EventSource::AnchorEvent,
            event_type: "AddressBlacklisted".to_string(),
            slot: 2,
            tx_signature: "sig-2".to_string(),
            instruction_index: 0,
            inner_instruction_index: None,
            event_index: Some(0),
            block_time: Some(Utc::now()),
            payload: json!({
                "mint":"mint-1",
                "wallet":"wallet-1",
                "authority":"auth-1",
                "reason":"screening"
            }),
        })
        .await?;
    service
        .ingest_chain_event(&ChainEvent {
            event_uid: "evt-minter".to_string(),
            program_id: "stablecoin".to_string(),
            mint: Some("mint-1".to_string()),
            event_source: EventSource::AnchorEvent,
            event_type: "MinterUpdated".to_string(),
            slot: 3,
            tx_signature: "sig-3".to_string(),
            instruction_index: 0,
            inner_instruction_index: None,
            event_index: Some(0),
            block_time: Some(Utc::now()),
            payload: json!({
                "mint":"mint-1",
                "minter":"auth-1",
                "quota":"1000000",
                "minted":"0",
                "active":true
            }),
        })
        .await?;
    service
        .ingest_chain_event(&ChainEvent {
            event_uid: "evt-mint".to_string(),
            program_id: "stablecoin".to_string(),
            mint: Some("mint-1".to_string()),
            event_source: EventSource::AnchorEvent,
            event_type: "TokensMinted".to_string(),
            slot: 4,
            tx_signature: "sig-4".to_string(),
            instruction_index: 0,
            inner_instruction_index: None,
            event_index: Some(0),
            block_time: Some(Utc::now()),
            payload: json!({
                "mint":"mint-1",
                "authority":"auth-1",
                "amount":"250000"
            }),
        })
        .await?;

    let store = Store::connect(&harness.database_url()).await?;
    let mint = store.get_mint("mint-1").await?.expect("mint projection");
    assert_eq!(mint.symbol, "RUSD");
    assert_eq!(mint.total_minted, 250_000);
    let blacklist = store.list_blacklist_entries("mint-1").await?;
    assert_eq!(blacklist.len(), 1);
    let quota = store
        .get_minter_quota("mint-1", "auth-1")
        .await?
        .expect("minter quota projection");
    assert_eq!(quota.minted, 250_000);
    let events = store.list_chain_events("mint-1", 10).await?;
    assert_eq!(events.len(), 4);
    Ok(())
}
