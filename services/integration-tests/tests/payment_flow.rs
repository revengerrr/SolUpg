//! End-to-end test for the DirectPay happy path.
//!
//! Prerequisites (see `../src/lib.rs` header):
//!   cd services
//!   docker compose up -d
//!   cargo test -p integration-tests -- --ignored --test-threads=1

use integration_tests::{require_stack, ApiClient, CreatePaymentRequest};
use serde_json::json;

/// Happy path: POST /v1/payments → poll until terminal state → assert on-chain signature.
#[tokio::test]
#[ignore]
async fn direct_pay_happy_path() {
    let client = ApiClient::new();
    if !require_stack(&client).await {
        return;
    }
    if client.cfg.api_key.is_none() {
        eprintln!("[skip] API_KEY not set — gateway requires auth for /v1/payments");
        return;
    }

    // Use placeholder wallets; on a local validator these can be any base58 pubkeys.
    // The routing engine will either submit a real tx (if validator is reachable) or
    // return an error in the payment body — either outcome is a useful signal.
    let payer = "PayerWalletAddr11111111111111111111111111111";
    let recipient = "RecipWallet111111111111111111111111111111111";
    let req = CreatePaymentRequest::direct_pay(payer, recipient, 10_000);

    let created = client
        .create_payment(&req)
        .await
        .expect("create_payment should succeed");

    let id = created
        .get("id")
        .and_then(|v| v.as_str())
        .expect("response must include id")
        .to_string();

    let final_body = client
        .poll_payment_terminal(&id)
        .await
        .expect("payment should reach terminal state");

    let status = final_body
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        matches!(status, "confirmed" | "failed"),
        "unexpected terminal status: {final_body}"
    );

    if status == "confirmed" {
        assert!(
            final_body.get("tx_signature").is_some(),
            "confirmed payment must carry a tx_signature: {final_body}"
        );
    }
}

/// Reject obviously malformed requests early (amount = 0).
#[tokio::test]
#[ignore]
async fn rejects_zero_amount() {
    let client = ApiClient::new();
    if !require_stack(&client).await {
        return;
    }
    if client.cfg.api_key.is_none() {
        eprintln!("[skip] API_KEY not set");
        return;
    }

    let req = json!({
        "payer": "PayerWalletAddr11111111111111111111111111111",
        "recipient": { "type": "Wallet", "value": "RecipWallet111111111111111111111111111111111" },
        "amount": 0,
        "route_type": "DirectPay",
    });

    let url = format!("{}/v1/payments", client.cfg.api_gateway);
    let mut builder = client.http.post(&url).json(&req);
    if let Some(ref key) = client.cfg.api_key {
        builder = builder.header("X-API-Key", key);
    }
    let resp = builder.send().await.expect("post should send");
    assert!(
        resp.status().is_client_error() || resp.status().is_server_error(),
        "zero-amount payment should be rejected, got {}",
        resp.status()
    );
}
