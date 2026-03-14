use std::{sync::Arc, time::Duration};

use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use sss_db::Store;
use sss_domain::{
    LifecycleStatus, SignerBackend, WebhookDeliveryStatus, WebhookDispatcher,
};
use tracing::{error, warn};

use crate::services::{HttpWebhookDispatcher, LocalKeypairSigner};
use crate::signer::AuthorityKeypairSigner;

pub struct OperationExecutorWorker<B: ?Sized> {
    pub store: Store,
    pub signer: Arc<B>,
    pub poll_limit: i64,
}

impl<B> OperationExecutorWorker<B>
where
    B: SignerBackend + ?Sized + 'static,
{
    pub async fn run_once(&self) -> Result<usize> {
        let requests = self
            .store
            .list_lifecycle_requests_by_status(LifecycleStatus::Approved, self.poll_limit)
            .await?;
        let mut processed = 0usize;

        for request in requests {
            processed += 1;
            self.store
                .mark_lifecycle_status(&request.id, LifecycleStatus::Signing, None, None)
                .await?;

            match self.signer.as_ref().execute(&request).await {
                Ok(result) => {
                    self.store
                        .mark_lifecycle_status(
                            &result.request_id,
                            LifecycleStatus::Finalized,
                            Some(&result.tx_signature),
                            None,
                        )
                        .await?;
                }
                Err(err) => {
                    self.store
                        .mark_lifecycle_status(
                            &request.id,
                            LifecycleStatus::Failed,
                            None,
                            Some(&err.to_string()),
                        )
                        .await?;
                }
            }
        }

        Ok(processed)
    }
}

pub struct WebhookRetryWorker<D> {
    pub store: Store,
    pub dispatcher: Arc<D>,
    pub poll_limit: i64,
    pub max_attempts: i32,
}

impl<D> WebhookRetryWorker<D>
where
    D: WebhookDispatcher + 'static,
{
    pub async fn run_once(&self) -> Result<usize> {
        let deliveries = self
            .store
            .list_due_webhook_deliveries(Utc::now(), self.poll_limit)
            .await?;
        let subscriptions = self.store.list_webhook_subscriptions().await?;
        let mut processed = 0usize;

        for delivery in deliveries {
            processed += 1;
            let Some(subscription) = subscriptions
                .iter()
                .find(|s| s.id == delivery.subscription_id)
            else {
                continue;
            };
            let Some(event) = self.store.get_event(delivery.event_id).await? else {
                continue;
            };

            match self
                .dispatcher
                .deliver(subscription, &delivery, &event)
                .await
            {
                Ok(status) => {
                    self.store
                        .mark_webhook_delivery(
                            delivery.id,
                            WebhookDeliveryStatus::Delivered,
                            delivery.attempts + 1,
                            None,
                            status,
                            None,
                        )
                        .await?;
                }
                Err(err) => {
                    let attempts = delivery.attempts + 1;
                    let status = if attempts >= self.max_attempts {
                        WebhookDeliveryStatus::DeadLetter
                    } else {
                        WebhookDeliveryStatus::Failed
                    };
                    let next_retry_at = if status == WebhookDeliveryStatus::DeadLetter {
                        None
                    } else {
                        Some(Utc::now() + ChronoDuration::seconds(2_i64.pow(attempts as u32)))
                    };
                    self.store
                        .mark_webhook_delivery(
                            delivery.id,
                            status,
                            attempts,
                            next_retry_at,
                            None,
                            Some(&err.to_string()),
                        )
                        .await?;
                }
            }
        }

        Ok(processed)
    }
}

pub async fn spawn_default_workers(store: Store) {
    let signer: Arc<dyn SignerBackend> = match AuthorityKeypairSigner::from_env() {
        Ok(s) => {
            tracing::info!("authority signer loaded from env; mint/burn will be executed on-chain");
            Arc::new(s)
        }
        Err(e) => {
            warn!(%e, "authority signer not configured; operation executor will reject mint/burn");
            Arc::new(LocalKeypairSigner)
        }
    };
    let dispatcher = Arc::new(HttpWebhookDispatcher::default());

    let operation_worker = OperationExecutorWorker {
        store: store.clone(),
        signer,
        poll_limit: 25,
    };
    let webhook_worker = WebhookRetryWorker {
        store,
        dispatcher,
        poll_limit: 25,
        max_attempts: 5,
    };

    tokio::spawn(async move {
        loop {
            if let Err(error) = operation_worker.run_once().await {
                error!(?error, "operation worker tick failed");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    tokio::spawn(async move {
        loop {
            if let Err(error) = webhook_worker.run_once().await {
                error!(?error, "webhook worker tick failed");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}
