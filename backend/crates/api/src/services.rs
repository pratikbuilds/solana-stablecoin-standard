use async_trait::async_trait;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use sss_domain::{
    AuditExport, AuditExporter, OperationExecutionResult, OperationRequest, SignerBackend,
    WebhookDelivery, WebhookDispatcher, WebhookEndpoint, WorkerError,
};

pub struct HttpWebhookDispatcher {
    client: Client,
}

impl Default for HttpWebhookDispatcher {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl WebhookDispatcher for HttpWebhookDispatcher {
    async fn deliver(
        &self,
        endpoint: &WebhookEndpoint,
        delivery: &WebhookDelivery,
    ) -> Result<Option<i32>, WorkerError> {
        type HmacSha256 = Hmac<Sha256>;

        let body =
            serde_json::to_vec(&delivery.payload).map_err(|err| WorkerError::Dependency(err.to_string()))?;
        let mut mac = HmacSha256::new_from_slice(endpoint.secret.as_bytes())
            .map_err(|err| WorkerError::Dependency(err.to_string()))?;
        mac.update(&body);
        let signature = hex::encode(mac.finalize().into_bytes());

        let response = self
            .client
            .post(&endpoint.url)
            .header("x-sss-signature", signature)
            .json(&delivery.payload)
            .send()
            .await
            .map_err(|err| WorkerError::Dependency(err.to_string()))?;

        if response.status().is_success() {
            Ok(Some(response.status().as_u16() as i32))
        } else {
            Err(WorkerError::Dependency(format!(
                "webhook delivery failed with status {}",
                response.status()
            )))
        }
    }
}

pub struct JsonAuditExporter;

#[async_trait]
impl AuditExporter for JsonAuditExporter {
    async fn export(&self, export: &AuditExport) -> Result<String, WorkerError> {
        Ok(format!("audit-export-{}.json", export.id))
    }
}

pub struct LocalKeypairSigner;

#[async_trait]
impl SignerBackend for LocalKeypairSigner {
    fn name(&self) -> &'static str {
        "local_keypair"
    }

    async fn execute(&self, operation: &OperationRequest) -> Result<OperationExecutionResult, WorkerError> {
        Err(WorkerError::Dependency(format!(
            "execution for {} is not wired yet",
            operation.kind.as_str()
        )))
    }
}
