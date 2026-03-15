use std::{
    collections::HashMap,
    process::Command,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sss_domain::{LifecycleRequest, LifecycleRequestType, LifecycleStatus};
use uuid::Uuid;

#[derive(Clone, Default)]
struct MockState {
    requests: Arc<Mutex<HashMap<String, LifecycleRequest>>>,
}

#[derive(serde::Deserialize)]
struct CreateLifecycleBody {
    mint: String,
    recipient: Option<String>,
    token_account: Option<String>,
    amount: i128,
    requested_by: String,
}

#[derive(serde::Deserialize)]
struct ApproveBody {
    approved_by: String,
}

#[derive(serde::Deserialize)]
struct ListQuery {
    mint: Option<String>,
    status: Option<LifecycleStatus>,
    #[serde(rename = "type")]
    type_: Option<LifecycleRequestType>,
    limit: Option<usize>,
}

#[tokio::test(flavor = "multi_thread")]
async fn cli_request_commands_round_trip_against_mock_api() {
    let state = MockState::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .route("/v1/mint-requests", post(create_mint_request))
        .route("/v1/burn-requests", post(create_burn_request))
        .route("/v1/operations", get(list_operations))
        .route("/v1/operations/{id}/approve", post(approve_operation))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let base_url = format!("http://{}", address);
    let mint = "Mint111111111111111111111111111111111111111";

    let mint_output = run_cli(
        &[
            "mint",
            "Recipient1111111111111111111111111111111111",
            "1000",
            "--yes",
        ],
        &base_url,
        mint,
    );
    assert!(mint_output.status.success(), "mint stderr: {}", String::from_utf8_lossy(&mint_output.stderr));
    let mint_stdout = String::from_utf8_lossy(&mint_output.stdout);
    assert!(mint_stdout.contains("type: mint"));
    assert!(mint_stdout.contains("status: requested"));

    let list_output = run_cli(
        &["operation", "list", "--status", "requested", "--type", "mint"],
        &base_url,
        mint,
    );
    assert!(list_output.status.success(), "list stderr: {}", String::from_utf8_lossy(&list_output.stderr));
    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_stdout.contains("type: mint"));
    assert!(list_stdout.contains("status: requested"));

    let request_id = extract_request_id(&list_stdout);
    let approve_output = run_cli(&["operation", "approve", &request_id], &base_url, mint);
    assert!(
        approve_output.status.success(),
        "approve stderr: {}",
        String::from_utf8_lossy(&approve_output.stderr)
    );
    let approve_stdout = String::from_utf8_lossy(&approve_output.stdout);
    assert!(approve_stdout.contains("status: approved"));

    let burn_output = run_cli(&["burn", "500", "--account", "Ata1111111111111111111111111111111111111", "--yes"], &base_url, mint);
    assert!(burn_output.status.success(), "burn stderr: {}", String::from_utf8_lossy(&burn_output.stderr));
    let burn_stdout = String::from_utf8_lossy(&burn_output.stdout);
    assert!(burn_stdout.contains("type: burn"));
    assert!(burn_stdout.contains("status: requested"));

    server.abort();
}

fn run_cli(args: &[&str], base_url: &str, mint: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sss-token"))
        .args(args)
        .env("SSS_API_URL", base_url)
        .env("SSS_MINT", mint)
        .env("USER", "cli-test")
        .output()
        .unwrap()
}

fn extract_request_id(output: &str) -> String {
    output
        .lines()
        .find_map(|line| line.strip_prefix("request_id: "))
        .expect("request id line present")
        .to_string()
}

async fn create_mint_request(
    State(state): State<MockState>,
    Json(body): Json<CreateLifecycleBody>,
) -> Json<LifecycleRequest> {
    Json(insert_request(
        &state,
        LifecycleRequestType::Mint,
        body.mint,
        body.recipient,
        body.token_account,
        body.amount,
        body.requested_by,
    ))
}

async fn create_burn_request(
    State(state): State<MockState>,
    Json(body): Json<CreateLifecycleBody>,
) -> Json<LifecycleRequest> {
    Json(insert_request(
        &state,
        LifecycleRequestType::Burn,
        body.mint,
        body.recipient,
        body.token_account,
        body.amount,
        body.requested_by,
    ))
}

async fn list_operations(
    State(state): State<MockState>,
    Query(query): Query<ListQuery>,
) -> Json<serde_json::Value> {
    let mut requests = state
        .requests
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect::<Vec<_>>();
    requests.retain(|request| {
        query.mint.as_ref().is_none_or(|mint| request.mint == *mint)
            && query.status.is_none_or(|status| request.status == status)
            && query.type_.is_none_or(|type_| request.type_ == type_)
    });
    requests.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    if let Some(limit) = query.limit {
        requests.truncate(limit);
    }
    Json(serde_json::json!({
        "requests": requests,
        "total": requests.len(),
    }))
}

async fn approve_operation(
    State(state): State<MockState>,
    Path(id): Path<String>,
    Json(body): Json<ApproveBody>,
) -> Json<LifecycleRequest> {
    let mut requests = state.requests.lock().unwrap();
    let request = requests.get_mut(&id).expect("request exists");
    request.status = LifecycleStatus::Approved;
    request.approved_by = Some(body.approved_by);
    request.updated_at = Utc::now();
    Json(request.clone())
}

fn insert_request(
    state: &MockState,
    type_: LifecycleRequestType,
    mint: String,
    recipient: Option<String>,
    token_account: Option<String>,
    amount: i128,
    requested_by: String,
) -> LifecycleRequest {
    let now = Utc::now();
    let request = LifecycleRequest {
        id: Uuid::new_v4().to_string(),
        type_,
        status: LifecycleStatus::Requested,
        mint,
        recipient,
        token_account,
        amount,
        minter: None,
        reason: None,
        idempotency_key: None,
        requested_by,
        approved_by: None,
        tx_signature: None,
        error: None,
        created_at: now,
        updated_at: now,
    };
    state
        .requests
        .lock()
        .unwrap()
        .insert(request.id.clone(), request.clone());
    request
}
