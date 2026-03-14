use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Serialize;
use sss_domain::{EventRecord, LifecycleRequest};
use uuid::Uuid;

use crate::config::InitConfigFile;

pub struct BackendClient {
    base_url: String,
    http: Client,
}

#[derive(Debug, Serialize)]
struct CreateLifecycleBody {
    mint: String,
    recipient: Option<String>,
    token_account: Option<String>,
    amount: i128,
    minter: Option<String>,
    reason: Option<String>,
    idempotency_key: Option<String>,
    requested_by: String,
}

#[derive(Debug, serde::Deserialize)]
struct EventsResponse {
    events: Vec<EventRecord>,
    total: i64,
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

    pub fn list_mint_events(
        &self,
        mint: &str,
        event_type: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<EventRecord>> {
        let mut url = format!("{}/v1/mints/{}/events", self.base_url, mint);
        let mut params: Vec<String> = Vec::new();
        if let Some(et) = event_type {
            params.push(format!("event_type={}", urlencoding::encode(et)));
        }
        if let Some(f) = from {
            params.push(format!("from={}", urlencoding::encode(f)));
        }
        if let Some(t) = to {
            params.push(format!("to={}", urlencoding::encode(t)));
        }
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }
        let resp: EventsResponse = self
            .http
            .get(&url)
            .send()
            .context("request mint events")?
            .error_for_status()
            .context("mint events request failed")?
            .json()
            .context("decode mint events response")?;
        Ok(resp.events)
    }

    pub fn create_mint_request(
        &self,
        mint: String,
        recipient: String,
        amount: i128,
        reason: Option<String>,
    ) -> Result<LifecycleRequest> {
        self.create_lifecycle_request(
            "/v1/mint-requests",
            CreateLifecycleBody {
                mint,
                recipient: Some(recipient),
                token_account: None,
                amount,
                minter: None,
                reason,
                idempotency_key: Some(Uuid::new_v4().to_string()),
                requested_by: requested_by(),
            },
        )
    }

    pub fn create_burn_request(
        &self,
        mint: String,
        account: Option<String>,
        amount: i128,
        reason: Option<String>,
    ) -> Result<LifecycleRequest> {
        self.create_lifecycle_request(
            "/v1/burn-requests",
            CreateLifecycleBody {
                mint,
                recipient: None,
                token_account: account,
                amount,
                minter: None,
                reason,
                idempotency_key: Some(Uuid::new_v4().to_string()),
                requested_by: requested_by(),
            },
        )
    }

    fn create_lifecycle_request(&self, path: &str, body: CreateLifecycleBody) -> Result<LifecycleRequest> {
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

    pub fn get_operation(&self, id: &str) -> Result<LifecycleRequest> {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            request: LifecycleRequest,
        }
        let w: Wrapper = self
            .http
            .get(format!("{}/v1/operations/{}", self.base_url, id))
            .send()
            .context("request operation")?
            .error_for_status()
            .context("get operation failed")?
            .json()
            .context("decode operation response")?;
        Ok(w.request)
    }

    pub fn approve_operation(&self, id: &str, approved_by: &str) -> Result<LifecycleRequest> {
        #[derive(serde::Serialize)]
        struct Body<'a> {
            approved_by: &'a str,
        }
        self.http
            .post(format!("{}/v1/operations/{}/approve", self.base_url, id))
            .json(&Body { approved_by })
            .send()
            .context("request approve")?
            .error_for_status()
            .context("approve failed")?
            .json()
            .context("decode approve response")
    }

    pub fn execute_operation(&self, id: &str) -> Result<LifecycleRequest> {
        self.http
            .post(format!("{}/v1/operations/{}/execute", self.base_url, id))
            .send()
            .context("request execute")?
            .error_for_status()
            .context("execute failed")?
            .json()
            .context("decode execute response")
    }
}

fn requested_by() -> String {
    std::env::var("USER").unwrap_or_else(|_| "sss-token".to_string())
}
