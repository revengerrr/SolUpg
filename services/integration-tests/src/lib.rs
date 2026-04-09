//! Cross-service integration test helpers for SolUPG.
//!
//! The tests in this crate are all `#[ignore]`'d so they don't run in the
//! normal `cargo test` loop. They assume the docker-compose stack is up and
//! healthy. Run with:
//!
//!   cd services
//!   docker compose up -d
//!   cargo test -p integration-tests -- --ignored --test-threads=1
//!
//! Environment overrides (with defaults):
//!
//!   API_GATEWAY_URL       http://localhost:3002
//!   ROUTING_ENGINE_URL    http://localhost:3000
//!   DIRECTORY_URL         http://localhost:3001
//!   CLEARING_URL          http://localhost:3003
//!   MONITORING_URL        http://localhost:3004
//!   SOLANA_RPC_URL        http://localhost:8899
//!   API_KEY               (blank — tests that hit authed endpoints will skip if unset)
//!   TEST_TIMEOUT_SECS     30

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// Endpoints for the running stack. All pulled from env with sensible defaults.
#[derive(Debug, Clone)]
pub struct StackConfig {
    pub api_gateway: String,
    pub routing_engine: String,
    pub directory: String,
    pub clearing: String,
    pub monitoring: String,
    pub solana_rpc: String,
    pub api_key: Option<String>,
    pub timeout: Duration,
}

impl StackConfig {
    pub fn from_env() -> Self {
        fn get(key: &str, default: &str) -> String {
            std::env::var(key).unwrap_or_else(|_| default.to_string())
        }
        let timeout_secs: u64 = std::env::var("TEST_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        Self {
            api_gateway: get("API_GATEWAY_URL", "http://localhost:3002"),
            routing_engine: get("ROUTING_ENGINE_URL", "http://localhost:3000"),
            directory: get("DIRECTORY_URL", "http://localhost:3001"),
            clearing: get("CLEARING_URL", "http://localhost:3003"),
            monitoring: get("MONITORING_URL", "http://localhost:3004"),
            solana_rpc: get("SOLANA_RPC_URL", "http://localhost:8899"),
            api_key: std::env::var("API_KEY").ok().filter(|s| !s.is_empty()),
            timeout: Duration::from_secs(timeout_secs),
        }
    }
}

/// Thin HTTP client preconfigured with timeout and optional API key header.
#[derive(Debug, Clone)]
pub struct ApiClient {
    pub http: Client,
    pub cfg: StackConfig,
}

impl ApiClient {
    pub fn new() -> Self {
        let cfg = StackConfig::from_env();
        let http = Client::builder()
            .timeout(cfg.timeout)
            .build()
            .expect("reqwest client");
        Self { http, cfg }
    }

    fn with_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.cfg.api_key {
            Some(key) => builder.header("X-API-Key", key),
            None => builder,
        }
    }

    /// Wait until every service's /health endpoint returns 2xx.
    /// Returns Err if the stack isn't healthy within the global timeout.
    pub async fn wait_for_stack(&self) -> Result<()> {
        let urls = [
            &self.cfg.api_gateway,
            &self.cfg.routing_engine,
            &self.cfg.directory,
            &self.cfg.clearing,
            &self.cfg.monitoring,
        ];
        let deadline = Instant::now() + self.cfg.timeout;
        for base in urls {
            let health = format!("{base}/health");
            loop {
                if Instant::now() > deadline {
                    anyhow::bail!("stack not healthy within timeout: {health}");
                }
                match self.http.get(&health).send().await {
                    Ok(resp) if resp.status().is_success() => break,
                    _ => tokio::time::sleep(Duration::from_millis(500)).await,
                }
            }
        }
        Ok(())
    }

    pub async fn health(&self, base_url: &str) -> Result<StatusCode> {
        let resp = self
            .http
            .get(format!("{base_url}/health"))
            .send()
            .await
            .with_context(|| format!("GET {base_url}/health"))?;
        Ok(resp.status())
    }

    /// POST /v1/payments on the API gateway.
    pub async fn create_payment(&self, req: &CreatePaymentRequest) -> Result<Value> {
        let url = format!("{}/v1/payments", self.cfg.api_gateway);
        let resp = self
            .with_auth(self.http.post(&url).json(req))
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;
        let status = resp.status();
        let body: Value = resp.json().await.context("decode create_payment body")?;
        anyhow::ensure!(
            status.is_success(),
            "create_payment returned {status}: {body}"
        );
        Ok(body)
    }

    pub async fn get_payment(&self, id: &str) -> Result<Value> {
        let url = format!("{}/v1/payments/{id}", self.cfg.api_gateway);
        let resp = self
            .with_auth(self.http.get(&url))
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let status = resp.status();
        let body: Value = resp.json().await.context("decode get_payment body")?;
        anyhow::ensure!(status.is_success(), "get_payment returned {status}: {body}");
        Ok(body)
    }

    /// Poll /v1/payments/{id} until status transitions from pending/submitted
    /// to a terminal state or the deadline elapses.
    pub async fn poll_payment_terminal(&self, id: &str) -> Result<Value> {
        let deadline = Instant::now() + self.cfg.timeout;
        loop {
            let body = self.get_payment(id).await?;
            let status = body
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if matches!(
                status.as_str(),
                "confirmed" | "failed" | "cancelled" | "expired"
            ) {
                return Ok(body);
            }
            if Instant::now() > deadline {
                anyhow::bail!("payment {id} never reached terminal state; last: {body}");
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Request/response helpers mirroring the API Gateway JSON contract.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePaymentRequest {
    pub payer: String,
    pub recipient: Value,
    pub amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u16>,
}

impl CreatePaymentRequest {
    pub fn direct_pay(payer: &str, recipient_wallet: &str, amount: u64) -> Self {
        Self {
            payer: payer.to_string(),
            recipient: json!({ "type": "Wallet", "value": recipient_wallet }),
            amount,
            source_token: Some("SOL".to_string()),
            destination_token: Some("SOL".to_string()),
            metadata: Some("integration-test".to_string()),
            route_type: Some("DirectPay".to_string()),
            slippage_bps: None,
        }
    }
}

/// Skip test cleanly when the stack isn't reachable.
pub async fn require_stack(client: &ApiClient) -> bool {
    match client.wait_for_stack().await {
        Ok(()) => true,
        Err(e) => {
            eprintln!("[skip] stack not reachable: {e}");
            false
        }
    }
}
