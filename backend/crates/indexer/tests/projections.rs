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
use sss_domain::{EventFilters, EventSort, InsertEvent, SortOrder};
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
async fn indexer_ingests_events() -> Result<()> {
    let harness = start_postgres()?;
    let service = IndexerService::new(IndexerConfig {
        database_url: harness.database_url(),
        rpc_url: "https://api.devnet.solana.com".to_string(),
        indexer_rpc_url: None,
        stablecoin_program_id: "stablecoin".to_string(),
        transfer_hook_program_id: "transfer-hook".to_string(),
        start_slot: 0,
        disable_block_subscribe: true,
    })
    .await?;

    let events = [
        InsertEvent {
            event_type: "StablecoinInitialized".to_string(),
            program_id: Some("stablecoin".to_string()),
            mint: Some("mint-1".to_string()),
            tx_signature: "sig-1".to_string(),
            slot: 1,
            block_time: Some(Utc::now()),
            instruction_index: 0,
            data: json!({
                "mint":"mint-1",
                "authority":"auth-1",
                "preset":"SSS-2",
                "name":"Regulated USD",
                "symbol":"RUSD",
            }),
        },
        InsertEvent {
            event_type: "AddressBlacklisted".to_string(),
            program_id: Some("stablecoin".to_string()),
            mint: Some("mint-1".to_string()),
            tx_signature: "sig-2".to_string(),
            slot: 2,
            block_time: Some(Utc::now()),
            instruction_index: 0,
            data: json!({
                "mint":"mint-1",
                "wallet":"wallet-1",
                "authority":"auth-1",
                "reason":"screening"
            }),
        },
        InsertEvent {
            event_type: "TokensMinted".to_string(),
            program_id: Some("stablecoin".to_string()),
            mint: Some("mint-1".to_string()),
            tx_signature: "sig-3".to_string(),
            slot: 3,
            block_time: Some(Utc::now()),
            instruction_index: 0,
            data: json!({
                "mint":"mint-1",
                "authority":"auth-1",
                "amount":"250000"
            }),
        },
    ];

    for event in &events {
        service.ingest_event(event).await?;
    }

    let store = Store::connect(&harness.database_url()).await?;
    let (listed, total) = store
        .list_events(
            Some("mint-1"),
            &EventFilters::default(),
            EventSort::Slot,
            SortOrder::Asc,
            10,
            0,
        )
        .await?;
    assert_eq!(listed.len(), 3);
    assert_eq!(total, 3);
    assert_eq!(listed[0].event_type, "StablecoinInitialized");
    assert_eq!(listed[1].event_type, "AddressBlacklisted");
    assert_eq!(listed[2].event_type, "TokensMinted");
    Ok(())
}
