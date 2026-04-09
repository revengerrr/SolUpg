use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::IndexedTransaction;

/// The transaction indexer listens for Solana program events and stores them
/// in the transactions_ledger table for querying and reconciliation.
pub struct TransactionIndexer {
    pool: PgPool,
    /// Solana RPC URL for WebSocket subscriptions
    rpc_url: String,
    /// Program IDs to monitor (SolUPG on-chain programs)
    program_ids: Vec<String>,
}

impl TransactionIndexer {
    pub fn new(pool: PgPool, rpc_url: String, program_ids: Vec<String>) -> Self {
        Self {
            pool,
            rpc_url,
            program_ids,
        }
    }

    /// Index a single transaction into the ledger.
    /// Used by both the real-time listener and batch backfill.
    pub async fn index_transaction(&self, tx: &IndexedTransaction) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO transactions_ledger (
                id, tx_signature, payment_id, payer, recipient, amount, token_mint,
                fee_amount, swap_source_token, swap_rate, swap_slippage_bps,
                status, block_slot, block_time, program_id, instruction_type, raw_log
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ON CONFLICT (tx_signature) DO NOTHING
            "#,
        )
        .bind(id)
        .bind(&tx.tx_signature)
        .bind(tx.payment_id)
        .bind(&tx.payer)
        .bind(&tx.recipient)
        .bind(tx.amount as i64)
        .bind(&tx.token_mint)
        .bind(tx.fee_amount as i64)
        .bind(&tx.swap_source_token)
        .bind(tx.swap_rate)
        .bind(tx.swap_slippage_bps.map(|v| v as i32))
        .bind(&tx.status)
        .bind(tx.block_slot as i64)
        .bind(tx.block_time)
        .bind(&tx.program_id)
        .bind(&tx.instruction_type)
        .bind(&tx.raw_log)
        .execute(&self.pool)
        .await?;

        info!(tx_sig = %tx.tx_signature, "Indexed transaction");
        Ok(id)
    }

    /// Batch index multiple transactions (e.g., during backfill).
    pub async fn index_batch(&self, transactions: &[IndexedTransaction]) -> Result<u64> {
        let mut indexed = 0u64;
        for tx in transactions {
            match self.index_transaction(tx).await {
                Ok(_) => indexed += 1,
                Err(e) => {
                    warn!(tx_sig = %tx.tx_signature, error = %e, "Failed to index transaction");
                }
            }
        }
        info!(count = indexed, "Batch indexing complete");
        Ok(indexed)
    }

    /// Get the latest indexed block slot (for resuming after restart).
    pub async fn get_latest_slot(&self) -> Result<Option<i64>> {
        let row: Option<(Option<i64>,)> =
            sqlx::query_as("SELECT MAX(block_slot) FROM transactions_ledger")
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.and_then(|r| r.0))
    }

    /// Count total indexed transactions.
    pub async fn get_total_count(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions_ledger")
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    /// Get transactions by payment ID.
    pub async fn get_by_payment_id(
        &self,
        payment_id: Uuid,
    ) -> Result<Vec<crate::models::TransactionRecord>> {
        let records = sqlx::query_as::<_, crate::models::TransactionRecord>(
            "SELECT * FROM transactions_ledger WHERE payment_id = $1 ORDER BY block_time DESC",
        )
        .bind(payment_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get transactions within a time range.
    pub async fn get_by_time_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::TransactionRecord>> {
        let records = sqlx::query_as::<_, crate::models::TransactionRecord>(
            r#"
            SELECT * FROM transactions_ledger
            WHERE block_time >= $1 AND block_time <= $2
            ORDER BY block_time DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Parse a Solana transaction log into an IndexedTransaction.
    /// In production, this would decode instruction data and program logs.
    /// Currently provides the structural parser that services connect to.
    pub fn parse_transaction_log(
        tx_signature: &str,
        payer: &str,
        recipient: &str,
        amount: u64,
        token_mint: &str,
        fee_amount: u64,
        block_slot: u64,
        block_time: DateTime<Utc>,
        program_id: &str,
        instruction_type: &str,
        raw_log: Option<&str>,
    ) -> IndexedTransaction {
        IndexedTransaction {
            tx_signature: tx_signature.to_string(),
            payment_id: None,
            payer: payer.to_string(),
            recipient: recipient.to_string(),
            amount,
            token_mint: token_mint.to_string(),
            fee_amount,
            swap_source_token: None,
            swap_rate: None,
            swap_slippage_bps: None,
            status: "confirmed".to_string(),
            block_slot,
            block_time,
            program_id: program_id.to_string(),
            instruction_type: instruction_type.to_string(),
            raw_log: raw_log.map(|s| s.to_string()),
        }
    }

    /// Start the real-time indexer loop.
    /// In production, this subscribes to Solana WebSocket for program log events.
    /// Returns a handle that can be used to stop the indexer.
    pub async fn start_realtime(&self) -> Result<()> {
        info!(
            rpc = %self.rpc_url,
            programs = ?self.program_ids,
            "Starting real-time transaction indexer"
        );

        // Production implementation would:
        // 1. Connect to Solana RPC WebSocket
        // 2. Subscribe to logsSubscribe for each program ID
        // 3. Parse incoming logs and call index_transaction
        // 4. Handle reconnection on disconnect
        //
        // For now, the indexer is driven externally via index_transaction/index_batch
        // which the API gateway and routing engine call after processing payments.

        info!("Real-time indexer initialized (event-driven mode)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_parse_transaction_log() {
        let tx = TransactionIndexer::parse_transaction_log(
            "5UfDuX..abc",
            "PayerWa11et111111111111111111111111111111111",
            "RecipWa11et111111111111111111111111111111111",
            1_000_000,
            "So11111111111111111111111111111111111111112",
            5_000,
            200_000_000,
            Utc::now(),
            "SoLuPGPaym111111111111111111111111111111111",
            "process_payment",
            Some("Program log: Payment processed"),
        );

        assert_eq!(tx.tx_signature, "5UfDuX..abc");
        assert_eq!(tx.amount, 1_000_000);
        assert_eq!(tx.fee_amount, 5_000);
        assert_eq!(tx.status, "confirmed");
        assert_eq!(tx.instruction_type, "process_payment");
        assert!(tx.payment_id.is_none());
    }

    #[test]
    fn test_parse_transaction_no_log() {
        let tx = TransactionIndexer::parse_transaction_log(
            "sig123",
            "payer1",
            "recip1",
            500,
            "mint1",
            0,
            100,
            Utc::now(),
            "prog1",
            "transfer",
            None,
        );

        assert!(tx.raw_log.is_none());
        assert_eq!(tx.fee_amount, 0);
    }
}
