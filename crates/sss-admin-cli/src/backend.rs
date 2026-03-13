use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::{json, Value};
use sss_domain::{ChainEvent, MintRecord, OperationRequest};
use uuid::Uuid;

use crate::config::InitConfigFile;

pub struct BackendClient {
    base_url: String,
    http: Client,
}

#[derive(Debug, Serialize)]
struct CreateOperationBody {
    mint: String,
    target_wallet: Option<String>,
    target_token_account: Option<String>,
    amount: Option<i128>,
    reason: Option<String>,
    external_reference: Option<String>,
    idempotency_key: String,
    requested_by: String,
    metadata: Value,
}

impl BackendClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: Client::new(),
        }
    }

    pub fn from_runtime(config: Option<&InitConfigFile>) -> Result<Self> {
        let base_url = config
            .and_then(|cfg| cfg.api_url.clone())
            .or_else(|| std::env::var("SSS_API_URL").ok())
            .context("api_url must be set in config or SSS_API_URL for backend-backed CLI commands")?;
        Ok(Self::new(base_url))
    }

    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("SSS_API_URL")
            .context("SSS_API_URL must be set for backend-backed CLI commands")?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: Client::new(),
        })
    }

    pub fn get_mint(&self, mint: &str) -> Result<MintRecord> {
        self.http
            .get(format!("{}/v1/mints/{}", self.base_url, mint))
            .send()
            .context("request mint status")?
            .error_for_status()
            .context("mint status request failed")?
            .json()
            .context("decode mint status response")
    }

    pub fn list_mint_events(&self, mint: &str) -> Result<Vec<ChainEvent>> {
        self.http
            .get(format!("{}/v1/mints/{}/events", self.base_url, mint))
            .send()
            .context("request mint events")?
            .error_for_status()
            .context("mint events request failed")?
            .json()
            .context("decode mint events response")
    }

    pub fn create_mint_request(
        &self,
        mint: String,
        recipient: String,
        amount: i128,
        reason: Option<String>,
    ) -> Result<OperationRequest> {
        self.create_operation(
            "/v1/mint-requests",
            CreateOperationBody {
                mint,
                target_wallet: Some(recipient),
                target_token_account: None,
                amount: Some(amount),
                reason,
                external_reference: None,
                idempotency_key: Uuid::new_v4().to_string(),
                requested_by: requested_by(),
                metadata: json!({}),
            },
        )
    }

    pub fn create_burn_request(
        &self,
        mint: String,
        account: Option<String>,
        amount: i128,
        reason: Option<String>,
    ) -> Result<OperationRequest> {
        self.create_operation(
            "/v1/burn-requests",
            CreateOperationBody {
                mint,
                target_wallet: None,
                target_token_account: account,
                amount: Some(amount),
                reason,
                external_reference: None,
                idempotency_key: Uuid::new_v4().to_string(),
                requested_by: requested_by(),
                metadata: json!({}),
            },
        )
    }

    pub fn create_freeze_request(
        &self,
        mint: String,
        address: String,
        reason: Option<String>,
    ) -> Result<OperationRequest> {
        self.create_operation(
            "/v1/compliance/freeze",
            CreateOperationBody {
                mint,
                target_wallet: Some(address),
                target_token_account: None,
                amount: None,
                reason,
                external_reference: None,
                idempotency_key: Uuid::new_v4().to_string(),
                requested_by: requested_by(),
                metadata: json!({}),
            },
        )
    }

    pub fn create_thaw_request(
        &self,
        mint: String,
        address: String,
        reason: Option<String>,
    ) -> Result<OperationRequest> {
        self.create_operation(
            "/v1/compliance/thaw",
            CreateOperationBody {
                mint,
                target_wallet: Some(address),
                target_token_account: None,
                amount: None,
                reason,
                external_reference: None,
                idempotency_key: Uuid::new_v4().to_string(),
                requested_by: requested_by(),
                metadata: json!({}),
            },
        )
    }

    pub fn create_blacklist_add_request(
        &self,
        mint: String,
        address: String,
        reason: String,
    ) -> Result<OperationRequest> {
        self.create_operation(
            "/v1/compliance/blacklists",
            CreateOperationBody {
                mint,
                target_wallet: Some(address),
                target_token_account: None,
                amount: None,
                reason: Some(reason),
                external_reference: None,
                idempotency_key: Uuid::new_v4().to_string(),
                requested_by: requested_by(),
                metadata: json!({}),
            },
        )
    }

    pub fn create_blacklist_remove_request(&self, mint: String, address: String) -> Result<OperationRequest> {
        self.http
            .delete(format!(
                "{}/v1/compliance/blacklists/{}/{}",
                self.base_url, mint, address
            ))
            .send()
            .context("request blacklist removal")?
            .error_for_status()
            .context("blacklist removal request failed")?
            .json()
            .context("decode blacklist removal response")
    }

    pub fn create_seize_request(
        &self,
        mint: String,
        address: String,
        treasury: String,
        amount: Option<i128>,
        reason: Option<String>,
    ) -> Result<OperationRequest> {
        self.create_operation(
            "/v1/compliance/seize",
            CreateOperationBody {
                mint,
                target_wallet: Some(address),
                target_token_account: Some(treasury),
                amount,
                reason,
                external_reference: None,
                idempotency_key: Uuid::new_v4().to_string(),
                requested_by: requested_by(),
                metadata: json!({}),
            },
        )
    }

    fn create_operation(&self, path: &str, body: CreateOperationBody) -> Result<OperationRequest> {
        self.http
            .post(format!("{}{}", self.base_url, path))
            .json(&body)
            .send()
            .with_context(|| format!("request {}", path))?
            .error_for_status()
            .with_context(|| format!("{} failed", path))?
            .json()
            .with_context(|| format!("decode response for {}", path))
    }
}

fn requested_by() -> String {
    std::env::var("USER").unwrap_or_else(|_| "sss-token".to_string())
}
