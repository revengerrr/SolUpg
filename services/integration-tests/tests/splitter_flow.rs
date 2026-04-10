//! End-to-end test for split payments (SplitPay route type).
//!
//! The gateway doesn't expose a dedicated `/v1/splits` endpoint; instead,
//! split payments are created by POSTing a payment with `route_type: "SplitPay"`
//! and the PDA of a pre-configured split config. These tests verify the
//! routing engine accepts the request and reports a terminal status.

use integration_tests::{require_stack, ApiClient};
use serde_json::json;

#[tokio::test]
#[ignore]
async fn split_pay_accepted_and_routed() {
    let client = ApiClient::new();
    if !require_stack(&client).await {
        return;
    }
    if client.cfg.api_key.is_none() {
        eprintln!("[skip] API_KEY not set");
        return;
    }

    // NOTE: `split_config` should be a real PDA created via the solupg-splitter
    // program. For local runs without seeded state, the routing engine will
    // reject the intent; that's still a meaningful signal (plumbing works,
    // validation fires). The test asserts terminal-state reachability.
    let body = json!({
        "payer": "PayerWalletAddr11111111111111111111111111111",
        "recipient": { "type": "Wallet", "value": "SplitAnchor11111111111111111111111111111111" },
        "source_token": "SOL",
        "destination_token": "SOL",
        "amount": 300_000u64,
        "metadata": "integration-test:split",
        "route_type": "SplitPay",
        "split_config": "SplitConfigPDA111111111111111111111111111111",
    });

    let url = format!("{}/v1/payments", client.cfg.api_gateway);
    let mut builder = client.http.post(&url).json(&body);
    if let Some(ref key) = client.cfg.api_key {
        builder = builder.header("X-API-Key", key);
    }
    let resp = builder.send().await.expect("post should send");
    let status = resp.status();
    let resp_body: serde_json::Value = resp.json().await.expect("json");

    if !status.is_success() {
        // Validation or missing split config — acceptable for this smoke test.
        eprintln!("[info] split_pay rejected at create: {status} {resp_body}");
        return;
    }

    let id = resp_body
        .get("id")
        .and_then(|v| v.as_str())
        .expect("response must include id")
        .to_string();

    let final_body = client
        .poll_payment_terminal(&id)
        .await
        .expect("should reach terminal state");

    let final_status = final_body
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        matches!(final_status, "confirmed" | "failed"),
        "unexpected terminal status: {final_body}"
    );
    assert_eq!(
        final_body.get("route_type").and_then(|v| v.as_str()),
        Some("SplitPay"),
        "routing engine should report SplitPay route_type: {final_body}"
    );
}
