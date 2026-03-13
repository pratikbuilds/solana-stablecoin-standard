use std::net::SocketAddr;

use sss_db::Store;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub database_url: String,
    pub bind_address: SocketAddr,
    /// When true, spawn background workers (operation executor, webhooks, audit).
    pub run_workers: bool,
}

impl ApiConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/sss_backend".to_string()),
            bind_address: std::env::var("SSS_API_BIND")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(|| "127.0.0.1:8080".parse().expect("valid bind addr")),
            run_workers: std::env::var("SSS_RUN_WORKERS").ok().as_deref() == Some("1"),
        }
    }
}
