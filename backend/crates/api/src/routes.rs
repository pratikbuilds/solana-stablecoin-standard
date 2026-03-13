//! API routes grouped by responsibility:
//!
//! - **Mints (read-only)**: Indexed mint catalog and events from chain.
//! - **Fiat-to-stablecoin**: Request → admin verify → execute (mint/burn via /v1/mint-requests, /v1/burn-requests, /v1/operations/...).
//! - **Compliance (SSS-2)**: Blacklist management, sanctions integration point, freeze/thaw/seize, audit trail export.
//! - **Webhooks (SSS-2)**: Configurable event notifications with retry logic.

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use sss_db::Store;
use sss_domain::{
    CreateAuditExport, CreateOperationRequest, CreateWebhookEndpoint, OperationKind, OperationStatus,
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::info;
use uuid::Uuid;

use crate::{
    config::{ApiConfig, AppState},
    dto::{ApproveOperationBody, CreateAuditExportBody, CreateOperationBody, OperationDetailsResponse},
    error::ApiError,
    workers::spawn_default_workers,
};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        // Mints: read-only catalog (indexer data)
        .route("/v1/mints", get(list_mints))
        .route("/v1/mints/{mint}", get(get_mint))
        .route("/v1/mints/{mint}/events", get(list_mint_events))
        .route("/v1/mints/{mint}/blacklist", get(list_blacklist))
        // Fiat-to-stablecoin: request → verify → execute (mint/burn)
        .route("/v1/mint-requests", post(create_mint_request))
        .route("/v1/burn-requests", post(create_burn_request))
        .route("/v1/operations/{id}", get(get_operation))
        .route("/v1/operations/{id}/approve", post(approve_operation))
        .route("/v1/operations/{id}/execute", post(execute_operation))
        // Compliance (SSS-2): blacklist, freeze/thaw/seize, audit export
        .route("/v1/compliance/blacklists", post(create_blacklist_add_request))
        .route(
            "/v1/compliance/blacklists/{mint}/{wallet}",
            delete(create_blacklist_remove_request),
        )
        .route("/v1/compliance/freeze", post(create_freeze_request))
        .route("/v1/compliance/thaw", post(create_thaw_request))
        .route("/v1/compliance/seize", post(create_seize_request))
        .route("/v1/compliance/audit-exports", post(create_audit_export))
        // Webhooks (SSS-2): event notifications with retry
        .route("/v1/webhooks/endpoints", post(create_webhook_endpoint))
        .layer(ServiceBuilder::new())
        .with_state(state)
}

pub async fn run(config: ApiConfig) -> Result<()> {
    let store = Store::connect(&config.database_url).await?;
    store.migrate().await?;
    if config.run_workers {
        let store_for_workers = store.clone();
        tokio::spawn(async move {
            spawn_default_workers(store_for_workers).await;
        });
        info!("workers spawned (SSS_RUN_WORKERS=1)");
    }
    let app = build_router(AppState { store });
    let listener = TcpListener::bind(config.bind_address).await?;
    info!(address = %config.bind_address, "starting sss-api");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

async fn readyz(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    state.store.readiness_check().await.map_err(ApiError::from)?;
    Ok(Json(json!({ "status": "ready" })))
}

async fn list_mints(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.store.list_mints().await.map_err(ApiError::from)?))
}

async fn get_mint(Path(mint): Path<String>, State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let Some(record) = state.store.get_mint(&mint).await.map_err(ApiError::from)? else {
        return Err(ApiError::not_found("mint not found"));
    };
    Ok(Json(record))
}

async fn list_mint_events(
    Path(mint): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.store.list_chain_events(&mint, 100).await.map_err(ApiError::from)?))
}

async fn list_blacklist(
    Path(mint): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.store.list_blacklist_entries(&mint).await.map_err(ApiError::from)?))
}

async fn create_mint_request(
    State(state): State<AppState>,
    Json(body): Json<CreateOperationBody>,
) -> Result<impl IntoResponse, ApiError> {
    create_operation(State(state), body, OperationKind::Mint).await
}

async fn create_burn_request(
    State(state): State<AppState>,
    Json(body): Json<CreateOperationBody>,
) -> Result<impl IntoResponse, ApiError> {
    create_operation(State(state), body, OperationKind::Burn).await
}

async fn create_blacklist_add_request(
    State(state): State<AppState>,
    Json(body): Json<CreateOperationBody>,
) -> Result<impl IntoResponse, ApiError> {
    create_operation(State(state), body, OperationKind::BlacklistAdd).await
}

async fn create_blacklist_remove_request(
    Path((mint, wallet)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let request = state
        .store
        .create_operation_request(&CreateOperationRequest {
            kind: OperationKind::BlacklistRemove,
            mint,
            target_wallet: Some(wallet.clone()),
            target_token_account: None,
            amount: None,
            reason: Some("remove from blacklist".to_string()),
            external_reference: None,
            idempotency_key: format!("blacklist-remove:{wallet}"),
            requested_by: "system".to_string(),
            metadata: json!({}),
        })
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(request)))
}

async fn create_freeze_request(
    State(state): State<AppState>,
    Json(body): Json<CreateOperationBody>,
) -> Result<impl IntoResponse, ApiError> {
    create_operation(State(state), body, OperationKind::Freeze).await
}

async fn create_thaw_request(
    State(state): State<AppState>,
    Json(body): Json<CreateOperationBody>,
) -> Result<impl IntoResponse, ApiError> {
    create_operation(State(state), body, OperationKind::Thaw).await
}

async fn create_seize_request(
    State(state): State<AppState>,
    Json(body): Json<CreateOperationBody>,
) -> Result<impl IntoResponse, ApiError> {
    create_operation(State(state), body, OperationKind::Seize).await
}

async fn create_operation(
    State(state): State<AppState>,
    body: CreateOperationBody,
    kind: OperationKind,
) -> Result<impl IntoResponse, ApiError> {
    let request = state
        .store
        .create_operation_request(&CreateOperationRequest {
            kind,
            mint: body.mint,
            target_wallet: body.target_wallet,
            target_token_account: body.target_token_account,
            amount: body.amount,
            reason: body.reason,
            external_reference: body.external_reference,
            idempotency_key: body.idempotency_key,
            requested_by: body.requested_by,
            metadata: body.metadata,
        })
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(request)))
}

async fn get_operation(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let Some(operation) = state.store.get_operation_request(id).await.map_err(ApiError::from)? else {
        return Err(ApiError::not_found("operation not found"));
    };
    let attempts = state.store.list_attempts(id).await.map_err(ApiError::from)?;
    Ok(Json(OperationDetailsResponse { operation, attempts }))
}

async fn approve_operation(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(body): Json<ApproveOperationBody>,
) -> Result<impl IntoResponse, ApiError> {
    let Some(operation) = state
        .store
        .approve_operation(id, &body.approved_by)
        .await
        .map_err(ApiError::from)?
    else {
        return Err(ApiError::unprocessable("operation must be requested before approval"));
    };
    Ok(Json(operation))
}

async fn execute_operation(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let Some(operation) = state.store.get_operation_request(id).await.map_err(ApiError::from)? else {
        return Err(ApiError::not_found("operation not found"));
    };
    if operation.status != OperationStatus::Approved && operation.status != OperationStatus::Submitted {
        return Err(ApiError::unprocessable("operation must be approved before execution"));
    }
    Ok((StatusCode::ACCEPTED, Json(operation)))
}

async fn create_webhook_endpoint(
    State(state): State<AppState>,
    Json(body): Json<CreateWebhookEndpoint>,
) -> Result<impl IntoResponse, ApiError> {
    let endpoint = state
        .store
        .create_webhook_endpoint(&body)
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(endpoint)))
}

async fn create_audit_export(
    State(state): State<AppState>,
    Json(body): Json<CreateAuditExportBody>,
) -> Result<impl IntoResponse, ApiError> {
    let export = state
        .store
        .create_audit_export(&CreateAuditExport {
            requested_by: body.requested_by,
            filters: body.filters,
        })
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(export)))
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use tower::util::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn healthz_route_returns_ok() {
        let app = Router::new().route("/healthz", get(healthz));
        let response = app
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
