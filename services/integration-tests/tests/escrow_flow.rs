//! End-to-end tests for escrow lifecycle: create → release / cancel / dispute.
//!
//! All tests are `#[ignore]`'d. Run with:
//!   cargo test -p integration-tests -- --ignored --test-threads=1

use integration_tests::{require_stack, ApiClient};
use serde_json::{json, Value};

async fn post_escrow(client: &ApiClient, body: &Value) -> anyhow::Result<Value> {
    let url = format!("{}/v1/escrows", client.cfg.api_gateway);
    let mut builder = client.http.post(&url).json(body);
    if let Some(ref key) = client.cfg.api_key {
        builder = builder.header("X-API-Key", key);
    }
    let resp = builder.send().await?;
    let status = resp.status();
    let body: Value = resp.json().await?;
    anyhow::ensure!(status.is_success(), "create escrow: {status} {body}");
    Ok(body)
}

async fn post_escrow_action(
    client: &ApiClient,
    id: &str,
    action: &str,
) -> anyhow::Result<Value> {
    let url = format!("{}/v1/escrows/{id}/{action}", client.cfg.api_gateway);
    let mut builder = client.http.post(&url);
    if let Some(ref key) = client.cfg.api_key {
        builder = builder.header("X-API-Key", key);
    }
    let resp = builder.send().await?;
    let status = resp.status();
    let body: Value = resp.json().await?;
    anyhow::ensure!(status.is_success(), "escrow {action}: {status} {body}");
    Ok(body)
}

fn base_escrow_payload() -> Value {
    json!({
        "payer": "PayerWalletAddr11111111111111111111111111111",
        "recipient": { "type": "Wallet", "value": "RecipWallet111111111111111111111111111111111" },
        "source_token": "SOL",
        "destination_token": "SOL",
        "amount": 50_000u64,
        "condition": "AuthorityApproval",
        "metadata": "integration-test:escrow",
    })
}

#[tokio::test]
#[ignore]
async fn escrow_create_and_release() {
    let client = ApiClient::new();
    if !require_stack(&client).await {
        return;
    }
    if client.cfg.api_key.is_none() {
        eprintln!("[skip] API_KEY not set");
        return;
    }

    let created = post_escrow(&client, &base_escrow_payload())
        .await
        .expect("escrow create");
    let id = created
        .get("id")
        .and_then(|v| v.as_str())
        .expect("escrow response must include id")
        .to_string();

    // Release is idempotent in the gateway stub (it just flips DB status),
    // so this should succeed even if on-chain confirmation lagged.
    let released = post_escrow_action(&client, &id, "release")
        .await
        .expect("escrow release");
    let status = released
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        matches!(status, "releasing" | "released" | "confirmed"),
        "unexpected release status: {released}"
    );
}

#[tokio::test]
#[ignore]
async fn escrow_create_and_cancel() {
    let client = ApiClient::new();
    if !require_stack(&client).await {
        return;
    }
    if client.cfg.api_key.is_none() {
        eprintln!("[skip] API_KEY not set");
        return;
    }

    let created = post_escrow(&client, &base_escrow_payload())
        .await
        .expect("escrow create");
    let id = created["id"].as_str().unwrap().to_string();

    let cancelled = post_escrow_action(&client, &id, "cancel")
        .await
        .expect("escrow cancel");
    assert_eq!(
        cancelled.get("status").and_then(|v| v.as_str()),
        Some("cancelled"),
        "expected cancelled status: {cancelled}"
    );
}

#[tokio::test]
#[ignore]
async fn escrow_create_and_dispute() {
    let client = ApiClient::new();
    if !require_stack(&client).await {
        return;
    }
    if client.cfg.api_key.is_none() {
        eprintln!("[skip] API_KEY not set");
        return;
    }

    let mut payload = base_escrow_payload();
    payload["condition"] = json!("MutualApproval");

    let created = post_escrow(&client, &payload).await.expect("escrow create");
    let id = created["id"].as_str().unwrap().to_string();

    let disputed = post_escrow_action(&client, &id, "dispute")
        .await
        .expect("escrow dispute");
    assert_eq!(
        disputed.get("status").and_then(|v| v.as_str()),
        Some("disputed"),
        "expected disputed status: {disputed}"
    );
}
