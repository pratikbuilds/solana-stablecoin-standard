use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    AnchorEvent,
    Instruction,
    SyntheticTransferHook,
}

impl EventSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnchorEvent => "anchor_event",
            Self::Instruction => "instruction",
            Self::SyntheticTransferHook => "synthetic_transfer_hook",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Mint,
    Burn,
    BlacklistAdd,
    BlacklistRemove,
    Freeze,
    Thaw,
    Seize,
}

impl OperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mint => "mint",
            Self::Burn => "burn",
            Self::BlacklistAdd => "blacklist_add",
            Self::BlacklistRemove => "blacklist_remove",
            Self::Freeze => "freeze",
            Self::Thaw => "thaw",
            Self::Seize => "seize",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Requested,
    Approved,
    Signing,
    Submitted,
    Finalized,
    Failed,
    Cancelled,
}

impl OperationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Approved => "approved",
            Self::Signing => "signing",
            Self::Submitted => "submitted",
            Self::Finalized => "finalized",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Started,
    Signed,
    Submitted,
    Confirmed,
    Failed,
}

impl AttemptStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Signed => "signed",
            Self::Submitted => "submitted",
            Self::Confirmed => "confirmed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookDeliveryStatus {
    Pending,
    Delivering,
    Delivered,
    Failed,
    DeadLetter,
}

impl WebhookDeliveryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Delivering => "delivering",
            Self::Delivered => "delivered",
            Self::Failed => "failed",
            Self::DeadLetter => "dead_letter",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditExportStatus {
    Requested,
    Processing,
    Completed,
    Failed,
}

impl AuditExportStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChainEvent {
    pub event_uid: String,
    pub program_id: String,
    pub mint: Option<String>,
    pub event_source: EventSource,
    pub event_type: String,
    pub slot: i64,
    pub tx_signature: String,
    pub instruction_index: i32,
    pub inner_instruction_index: Option<i32>,
    pub event_index: Option<i32>,
    pub block_time: Option<DateTime<Utc>>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MintRecord {
    pub mint: String,
    pub preset: String,
    pub authority: String,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub decimals: i16,
    pub enable_permanent_delegate: bool,
    pub enable_transfer_hook: bool,
    pub default_account_frozen: bool,
    pub paused: bool,
    pub total_minted: i128,
    pub total_burned: i128,
    pub created_at: DateTime<Utc>,
    pub last_changed_by: String,
    pub last_changed_at: DateTime<Utc>,
    pub indexed_slot: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MintRoleRecord {
    pub mint: String,
    pub master_authority: String,
    pub pauser: String,
    pub burner: String,
    pub blacklister: String,
    pub seizer: String,
    pub updated_at: DateTime<Utc>,
    pub indexed_slot: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinterQuotaRecord {
    pub mint: String,
    pub minter: String,
    pub quota: i128,
    pub minted: i128,
    pub active: bool,
    pub updated_at: DateTime<Utc>,
    pub indexed_slot: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlacklistEntryRecord {
    pub mint: String,
    pub wallet: String,
    pub reason: String,
    pub blacklisted_by: String,
    pub blacklisted_at: DateTime<Utc>,
    pub active: bool,
    pub removed_at: Option<DateTime<Utc>>,
    pub indexed_slot: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComplianceActionRecord {
    pub id: Option<i64>,
    pub mint: String,
    pub action_type: String,
    pub wallet: Option<String>,
    pub token_account: Option<String>,
    pub authority: String,
    pub amount: Option<i128>,
    pub tx_signature: String,
    pub slot: i64,
    pub related_operation_id: Option<Uuid>,
    pub details: Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationRequest {
    pub id: Uuid,
    pub kind: OperationKind,
    pub mint: String,
    pub target_wallet: Option<String>,
    pub target_token_account: Option<String>,
    pub amount: Option<i128>,
    pub reason: Option<String>,
    pub external_reference: Option<String>,
    pub idempotency_key: String,
    pub status: OperationStatus,
    pub requested_by: String,
    pub approved_by: Option<String>,
    pub tx_signature: Option<String>,
    pub failure_reason: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateOperationRequest {
    pub kind: OperationKind,
    pub mint: String,
    pub target_wallet: Option<String>,
    pub target_token_account: Option<String>,
    pub amount: Option<i128>,
    pub reason: Option<String>,
    pub external_reference: Option<String>,
    pub idempotency_key: String,
    pub requested_by: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationAttempt {
    pub id: Option<i64>,
    pub operation_id: Uuid,
    pub attempt_number: i32,
    pub status: AttemptStatus,
    pub signer_backend: String,
    pub tx_signature: Option<String>,
    pub rpc_endpoint: Option<String>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebhookEndpoint {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub secret: String,
    pub subscribed_event_types: Vec<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateWebhookEndpoint {
    pub name: String,
    pub url: String,
    pub secret: String,
    pub subscribed_event_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebhookDelivery {
    pub id: Option<i64>,
    pub webhook_endpoint_id: Uuid,
    pub source_event_key: String,
    pub event_type: String,
    pub payload: Value,
    pub status: WebhookDeliveryStatus,
    pub attempt_count: i32,
    pub next_attempt_at: Option<DateTime<Utc>>,
    pub last_http_status: Option<i32>,
    pub last_error: Option<String>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditExport {
    pub id: Uuid,
    pub status: AuditExportStatus,
    pub requested_by: String,
    pub filters: Value,
    pub artifact_path: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateAuditExport {
    pub requested_by: String,
    pub filters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationExecutionResult {
    pub operation_id: Uuid,
    pub tx_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthReport {
    pub component: String,
    pub layer: String,
    pub status: String,
}

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("operation {0} cannot transition from {1} to {2}")]
    InvalidTransition(Uuid, &'static str, &'static str),
    #[error("dependency error: {0}")]
    Dependency(String),
}

#[async_trait]
pub trait SignerBackend: Send + Sync {
    fn name(&self) -> &'static str;
    async fn execute(&self, operation: &OperationRequest) -> Result<OperationExecutionResult, WorkerError>;
}

#[async_trait]
pub trait WebhookDispatcher: Send + Sync {
    async fn deliver(
        &self,
        endpoint: &WebhookEndpoint,
        delivery: &WebhookDelivery,
    ) -> Result<Option<i32>, WorkerError>;
}

#[async_trait]
pub trait AuditExporter: Send + Sync {
    async fn export(&self, export: &AuditExport) -> Result<String, WorkerError>;
}
