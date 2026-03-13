use serde::{Deserialize, Serialize};
use serde_json::Value;
use sss_domain::{OperationAttempt, OperationRequest};

#[derive(Debug, Deserialize)]
pub struct CreateOperationBody {
    pub mint: String,
    pub target_wallet: Option<String>,
    pub target_token_account: Option<String>,
    pub amount: Option<i128>,
    pub reason: Option<String>,
    pub external_reference: Option<String>,
    pub idempotency_key: String,
    pub requested_by: String,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Deserialize)]
pub struct ApproveOperationBody {
    pub approved_by: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAuditExportBody {
    pub requested_by: String,
    #[serde(default)]
    pub filters: Value,
}

#[derive(Debug, Serialize)]
pub struct OperationDetailsResponse {
    pub operation: OperationRequest,
    pub attempts: Vec<OperationAttempt>,
}
