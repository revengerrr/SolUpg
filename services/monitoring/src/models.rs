use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Fraud Rules ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleType {
    #[serde(rename = "velocity")]
    Velocity,
    #[serde(rename = "threshold")]
    Threshold,
    #[serde(rename = "sanctions")]
    Sanctions,
    #[serde(rename = "pattern")]
    Pattern,
    #[serde(rename = "geo")]
    Geo,
}

impl RuleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Velocity => "velocity",
            Self::Threshold => "threshold",
            Self::Sanctions => "sanctions",
            Self::Pattern => "pattern",
            Self::Geo => "geo",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "block")]
    Block,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
            Self::Block => "block",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FraudRule {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub rule_type: String,
    pub config: serde_json::Value,
    pub severity: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityConfig {
    pub max_transactions: u32,
    pub window_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub max_amount: u64,
    pub token_mint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoConfig {
    pub blocked_countries: Vec<String>,
}

// ── Risk Scores ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RiskScore {
    pub id: Uuid,
    pub wallet_address: String,
    pub score: i32,
    pub factors: serde_json::Value,
    pub last_evaluated: DateTime<Utc>,
    pub source: String,
}

// ── Fraud Alerts ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertStatus {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "reviewing")]
    Reviewing,
    #[serde(rename = "resolved")]
    Resolved,
    #[serde(rename = "dismissed")]
    Dismissed,
}

impl AlertStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Reviewing => "reviewing",
            Self::Resolved => "resolved",
            Self::Dismissed => "dismissed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FraudAlert {
    pub id: Uuid,
    pub rule_id: Option<Uuid>,
    pub wallet_address: String,
    pub tx_signature: Option<String>,
    pub intent_id: Option<Uuid>,
    pub alert_type: String,
    pub severity: String,
    pub details: serde_json::Value,
    pub status: String,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ── Sanctions ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SanctionEntry {
    pub id: Uuid,
    pub wallet_address: String,
    pub list_source: String,
    pub reason: Option<String>,
    pub added_at: DateTime<Utc>,
    pub is_active: bool,
}

// ── Audit Trail ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditEntry {
    pub id: Uuid,
    pub event_type: String,
    pub actor_type: String,
    pub actor_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ── Alert Channels ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlertChannel {
    pub id: Uuid,
    pub name: String,
    pub channel_type: String,
    pub config: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

// ── Metrics (Prometheus-style) ──

#[derive(Debug, Clone, Serialize)]
pub struct ServiceMetrics {
    pub service_name: String,
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub error_count: u64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransactionMetrics {
    pub total_processed: u64,
    pub successful: u64,
    pub failed: u64,
    pub tps_current: f64,
    pub tps_peak: f64,
    pub avg_amount: f64,
}

// ── Request types ──

#[derive(Debug, Clone, Deserialize)]
pub struct ScreeningRequest {
    pub wallet_address: String,
    pub tx_signature: Option<String>,
    pub intent_id: Option<Uuid>,
    pub amount: u64,
    pub token_mint: String,
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreeningResult {
    pub wallet_address: String,
    pub risk_score: i32,
    pub alerts: Vec<FraudAlert>,
    pub action: ScreeningAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreeningAction {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "flag")]
    Flag,
    #[serde(rename = "block")]
    Block,
}
