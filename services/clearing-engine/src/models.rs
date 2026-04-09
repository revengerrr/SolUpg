use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Transactions Ledger ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TransactionRecord {
    pub id: Uuid,
    pub tx_signature: String,
    pub payment_id: Option<Uuid>,
    pub payer: String,
    pub recipient: String,
    pub amount: i64,
    pub token_mint: String,
    pub fee_amount: i64,
    pub swap_source_token: Option<String>,
    pub swap_rate: Option<f64>,
    pub swap_slippage_bps: Option<i32>,
    pub status: String,
    pub block_slot: i64,
    pub block_time: DateTime<Utc>,
    pub program_id: String,
    pub instruction_type: String,
    pub raw_log: Option<String>,
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedTransaction {
    pub tx_signature: String,
    pub payment_id: Option<Uuid>,
    pub payer: String,
    pub recipient: String,
    pub amount: u64,
    pub token_mint: String,
    pub fee_amount: u64,
    pub swap_source_token: Option<String>,
    pub swap_rate: Option<f64>,
    pub swap_slippage_bps: Option<u16>,
    pub status: String,
    pub block_slot: u64,
    pub block_time: DateTime<Utc>,
    pub program_id: String,
    pub instruction_type: String,
    pub raw_log: Option<String>,
}

// ── Reconciliation ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReconciliationRun {
    pub id: Uuid,
    pub run_type: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_intents: i32,
    pub total_transactions: i32,
    pub matched: i32,
    pub mismatched: i32,
    pub orphaned_tx: i32,
    pub missing_tx: i32,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MismatchType {
    #[serde(rename = "amount")]
    Amount,
    #[serde(rename = "missing_tx")]
    MissingTx,
    #[serde(rename = "orphaned_tx")]
    OrphanedTx,
    #[serde(rename = "status")]
    Status,
}

impl MismatchType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Amount => "amount",
            Self::MissingTx => "missing_tx",
            Self::OrphanedTx => "orphaned_tx",
            Self::Status => "status",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationMismatch {
    pub intent_id: Option<Uuid>,
    pub tx_signature: Option<String>,
    pub mismatch_type: MismatchType,
    pub expected_value: Option<String>,
    pub actual_value: Option<String>,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub run: ReconciliationRun,
    pub mismatches: Vec<ReconciliationMismatch>,
}

// ── Settlement ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SettlementBatch {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_transactions: i32,
    pub total_volume: i64,
    pub total_fees: i64,
    pub net_settlement: i64,
    pub currency_mint: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub finalized_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TokenBreakdown {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub token_mint: String,
    pub transaction_count: i32,
    pub volume: i64,
    pub fees: i64,
}

// ── Dashboard / Analytics ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueStats {
    pub period: String,
    pub total_volume: i64,
    pub total_fees: i64,
    pub transaction_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub token_mint: String,
    pub transaction_count: i64,
    pub total_volume: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFeedItem {
    pub id: Uuid,
    pub tx_signature: String,
    pub payer: String,
    pub recipient: String,
    pub amount: i64,
    pub token_mint: String,
    pub fee_amount: i64,
    pub status: String,
    pub block_time: DateTime<Utc>,
    pub instruction_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

impl PaginationParams {
    pub fn offset(&self) -> u32 {
        let page = self.page.unwrap_or(1).max(1);
        let limit = self.limit();
        (page - 1) * limit
    }

    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(50).min(200)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DateRangeParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionFilterParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub status: Option<String>,
    pub token_mint: Option<String>,
    pub merchant_id: Option<Uuid>,
}

impl TransactionFilterParams {
    pub fn pagination(&self) -> PaginationParams {
        PaginationParams {
            page: self.page,
            limit: self.limit,
        }
    }

    pub fn date_range(&self) -> DateRangeParams {
        DateRangeParams {
            from: self.from,
            to: self.to,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub page: u32,
    pub limit: u32,
    pub total: i64,
}
