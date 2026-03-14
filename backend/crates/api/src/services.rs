use async_trait::async_trait;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use sss_domain::{
    EventRecord, LifecycleExecutionResult, LifecycleRequest, SignerBackend, WebhookDelivery,
    WebhookDispatcher, WebhookSubscription, WorkerError,
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
        subscription: &WebhookSubscription,
        delivery: &WebhookDelivery,
        event: &EventRecord,
    ) -> Result<Option<i32>, WorkerError> {
        type HmacSha256 = Hmac<Sha256>;

        let payload = serde_json::json!({
            "event_id": event.id,
            "event_type": event.event_type,
            "program_id": event.program_id,
            "mint": event.mint,
            "tx_signature": event.tx_signature,
            "slot": event.slot,
            "block_time": event.block_time,
            "instruction_index": event.instruction_index,
            "data": event.data,
            "created_at": event.created_at,
        });

        let body = serde_json::to_vec(&payload).map_err(|err| WorkerError::Dependency(err.to_string()))?;
        let mut mac = HmacSha256::new_from_slice(
            subscription
                .secret
                .as_deref()
                .unwrap_or("")
                .as_bytes(),
        )
        .map_err(|err| WorkerError::Dependency(err.to_string()))?;
        mac.update(&body);
        let signature = hex::encode(mac.finalize().into_bytes());

        let response = self
            .client
            .post(&subscription.url)
            .header("x-sss-signature", signature)
            .json(&payload)
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

pub struct LocalKeypairSigner;

#[async_trait]
impl SignerBackend for LocalKeypairSigner {
    fn name(&self) -> &'static str {
        "local_keypair"
    }

    async fn execute(&self, _request: &LifecycleRequest) -> Result<LifecycleExecutionResult, WorkerError> {
        Err(WorkerError::Dependency(
            "execution is not wired for local_keypair signer".to_string(),
        ))
    }
}
