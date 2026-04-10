//! End-to-end test for SwapPay (source_token != destination_token).
//!
//! Exercises the routing engine's Jupiter/orca integration path via the
//! gateway. Without a seeded DEX on the local validator the swap will
//! likely fail at execution — the test asserts plumbing, not settlement.

use integration_tests::{require_stack, ApiClient};
use serde_json::json;

/// Well-known USDC devnet mint. Safe to use as a placeholder on localnet;
/// the routing engine treats it as an opaque pubkey for plan-building.
const USDC_MINT_DEVNET: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

#[tokio::test]
#[ignore]
async fn swap_pay_accepted_and_routed() {
    let client = ApiClient::new();
    if !require_stack(&client).await {
        return;
    }
    if client.cfg.api_key.is_none() {
        eprintln!("[skip] API_KEY not set");
        return;
    }

    let body = json!({
        "payer": "PayerWalletAddr11111111111111111111111111111",
        "recipient": { "type": "Wallet", "value": "RecipWallet111111111111111111111111111111111" },
        "source_token": "SOL",
        "destination_token": USDC_MINT_DEVNET,
        "amount": 20_000u64,
        "route_type": "SwapPay",
        "slippage_bps": 100,
        "metadata": "integration-test:swap",
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
        // Jupiter may reject unknown pairs on local validator — acceptable signal.
        eprintln!("[info] swap_pay rejected at create: {status} {resp_body}");
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

    let route_type = final_body.get("route_type").and_then(|v| v.as_str());
    assert!(
        matches!(route_type, Some("SwapPay") | None),
        "routing engine should report SwapPay (or omit) when tokens differ: {final_body}"
    );
}

/// Rejects a SwapPay intent where source_token == destination_token — that's
/// a contradiction; the gateway/routing engine should refuse or downgrade it.
#[tokio::test]
#[ignore]
async fn swap_pay_rejects_same_token() {
    let client = ApiClient::new();
    if !require_stack(&client).await {
        return;
    }
    if client.cfg.api_key.is_none() {
        eprintln!("[skip] API_KEY not set");
        return;
    }

    let body = json!({
        "payer": "PayerWalletAddr11111111111111111111111111111",
        "recipient": { "type": "Wallet", "value": "RecipWallet111111111111111111111111111111111" },
        "source_token": "SOL",
        "destination_token": "SOL",
        "amount": 10_000u64,
        "route_type": "SwapPay",
    });

    let url = format!("{}/v1/payments", client.cfg.api_gateway);
    let mut builder = client.http.post(&url).json(&body);
    if let Some(ref key) = client.cfg.api_key {
        builder = builder.header("X-API-Key", key);
    }
    let resp = builder.send().await.expect("post should send");

    // Either the gateway rejects up front, or the routing engine downgrades
    // to DirectPay — either is acceptable, but it must NOT silently perform
    // a no-op swap.
    if resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.expect("json");
        let route = body.get("route_type").and_then(|v| v.as_str());
        assert_ne!(
            route,
            Some("SwapPay"),
            "same-token intent should not be routed as SwapPay: {body}"
        );
    }
}
