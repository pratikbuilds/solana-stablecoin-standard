use std::{sync::Arc, time::Duration};

use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use sss_db::Store;
use sss_domain::{
    AttemptStatus, AuditExporter, AuditExportStatus, OperationAttempt, OperationStatus,
    SignerBackend, WebhookDeliveryStatus, WebhookDispatcher,
};
use tracing::{error, warn};

use crate::services::{HttpWebhookDispatcher, JsonAuditExporter, LocalKeypairSigner};
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
        let operations = self
            .store
            .list_operations_by_status(OperationStatus::Approved, self.poll_limit)
            .await?;
        let mut processed = 0usize;

        for operation in operations {
            processed += 1;
            self.store
                .mark_operation_status(operation.id, OperationStatus::Signing, None, None)
                .await?;

            let attempt = OperationAttempt {
                id: None,
                operation_id: operation.id,
                attempt_number: 1,
                status: AttemptStatus::Started,
                signer_backend: self.signer.name().to_string(),
                tx_signature: None,
                rpc_endpoint: None,
                error_message: None,
                started_at: Utc::now(),
                finished_at: None,
            };
            self.store.create_attempt(&attempt).await?;

            match self.signer.as_ref().execute(&operation).await {
                Ok(result) => {
                    self.store
                        .create_attempt(&OperationAttempt {
                            status: AttemptStatus::Submitted,
                            tx_signature: Some(result.tx_signature.clone()),
                            finished_at: Some(Utc::now()),
                            ..attempt.clone()
                        })
                        .await?;
                    self.store
                        .mark_operation_status(
                            result.operation_id,
                            OperationStatus::Submitted,
                            Some(&result.tx_signature),
                            None,
                        )
                        .await?;
                }
                Err(error) => {
                    self.store
                        .create_attempt(&OperationAttempt {
                            status: AttemptStatus::Failed,
                            error_message: Some(error.to_string()),
                            finished_at: Some(Utc::now()),
                            ..attempt.clone()
                        })
                        .await?;
                    self.store
                        .mark_operation_status(
                            operation.id,
                            OperationStatus::Failed,
                            None,
                            Some(&error.to_string()),
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
            .list_due_deliveries(Utc::now(), self.poll_limit)
            .await?;
        let endpoints = self.store.list_webhook_endpoints().await?;
        let mut processed = 0usize;

        for delivery in deliveries {
            processed += 1;
            let Some(delivery_id) = delivery.id else {
                continue;
            };
            let Some(endpoint) = endpoints
                .iter()
                .find(|candidate| candidate.id == delivery.webhook_endpoint_id)
            else {
                continue;
            };

            match self.dispatcher.deliver(endpoint, &delivery).await {
                Ok(status) => {
                    self.store
                        .mark_webhook_delivery(
                            delivery_id,
                            WebhookDeliveryStatus::Delivered,
                            delivery.attempt_count + 1,
                            None,
                            status,
                            None,
                            Some(Utc::now()),
                        )
                        .await?;
                }
                Err(error) => {
                    let attempt_count = delivery.attempt_count + 1;
                    let status = if attempt_count >= self.max_attempts {
                        WebhookDeliveryStatus::DeadLetter
                    } else {
                        WebhookDeliveryStatus::Failed
                    };
                    let next_attempt_at = if status == WebhookDeliveryStatus::DeadLetter {
                        None
                    } else {
                        Some(Utc::now() + ChronoDuration::seconds(2_i64.pow(attempt_count as u32)))
                    };
                    self.store
                        .mark_webhook_delivery(
                            delivery_id,
                            status,
                            attempt_count,
                            next_attempt_at,
                            None,
                            Some(&error.to_string()),
                            None,
                        )
                        .await?;
                }
            }
        }

        Ok(processed)
    }
}

pub struct AuditExportWorker<E> {
    pub store: Store,
    pub exporter: Arc<E>,
    pub poll_limit: i64,
}

impl<E> AuditExportWorker<E>
where
    E: AuditExporter + 'static,
{
    pub async fn run_once(&self) -> Result<usize> {
        let exports = self
            .store
            .list_audit_exports_by_status(AuditExportStatus::Requested, self.poll_limit)
            .await?;
        let mut processed = 0usize;

        for export in exports {
            processed += 1;
            self.store
                .mark_audit_export(export.id, AuditExportStatus::Processing, None, None)
                .await?;
            match self.exporter.export(&export).await {
                Ok(path) => {
                    self.store
                        .mark_audit_export(export.id, AuditExportStatus::Completed, Some(&path), None)
                        .await?;
                }
                Err(error) => {
                    self.store
                        .mark_audit_export(
                            export.id,
                            AuditExportStatus::Failed,
                            None,
                            Some(&error.to_string()),
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
    let exporter = Arc::new(JsonAuditExporter);

    let operation_worker = OperationExecutorWorker {
        store: store.clone(),
        signer,
        poll_limit: 25,
    };
    let webhook_worker = WebhookRetryWorker {
        store: store.clone(),
        dispatcher,
        poll_limit: 25,
        max_attempts: 5,
    };
    let audit_worker = AuditExportWorker {
        store,
        exporter,
        poll_limit: 10,
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

    tokio::spawn(async move {
        loop {
            if let Err(error) = audit_worker.run_once().await {
                error!(?error, "audit worker tick failed");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}
