use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::models::{SettlementBatch, TokenBreakdown};

/// The settlement engine generates settlement batches for merchants,
/// aggregating transaction volumes, fees, and per-token breakdowns.
pub struct SettlementEngine {
    pool: PgPool,
}

impl SettlementEngine {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Generate a settlement batch for a merchant over a time period.
    pub async fn generate_batch(
        &self,
        merchant_id: Uuid,
        merchant_wallet: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<SettlementBatch> {
        let batch_id = Uuid::new_v4();

        // Aggregate transactions for this merchant (as recipient)
        let stats: Option<(i64, Option<i64>, Option<i64>)> = sqlx::query_as(
            r#"
            SELECT COUNT(*), SUM(amount), SUM(fee_amount)
            FROM transactions_ledger
            WHERE recipient = $1
              AND block_time >= $2 AND block_time <= $3
              AND status = 'confirmed'
            "#,
        )
        .bind(merchant_wallet)
        .bind(period_start)
        .bind(period_end)
        .fetch_optional(&self.pool)
        .await?;

        let (tx_count, total_volume, total_fees) = stats.unwrap_or((0, None, None));
        let total_volume = total_volume.unwrap_or(0);
        let total_fees = total_fees.unwrap_or(0);
        let net_settlement = total_volume - total_fees;

        // Determine the primary currency (most used token)
        let primary_mint: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT token_mint
            FROM transactions_ledger
            WHERE recipient = $1
              AND block_time >= $2 AND block_time <= $3
              AND status = 'confirmed'
            GROUP BY token_mint
            ORDER BY COUNT(*) DESC
            LIMIT 1
            "#,
        )
        .bind(merchant_wallet)
        .bind(period_start)
        .bind(period_end)
        .fetch_optional(&self.pool)
        .await?;

        let currency_mint = primary_mint
            .map(|(m,)| m)
            .unwrap_or_else(|| "So11111111111111111111111111111111111111112".to_string());

        // Insert the batch
        let batch = sqlx::query_as::<_, SettlementBatch>(
            r#"
            INSERT INTO settlement_batches (
                id, merchant_id, period_start, period_end,
                total_transactions, total_volume, total_fees, net_settlement,
                currency_mint, status
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending')
            RETURNING *
            "#,
        )
        .bind(batch_id)
        .bind(merchant_id)
        .bind(period_start)
        .bind(period_end)
        .bind(tx_count as i32)
        .bind(total_volume)
        .bind(total_fees)
        .bind(net_settlement)
        .bind(&currency_mint)
        .fetch_one(&self.pool)
        .await?;

        // Generate per-token breakdown
        let token_rows: Vec<(String, i64, Option<i64>, Option<i64>)> = sqlx::query_as(
            r#"
            SELECT token_mint, COUNT(*), SUM(amount), SUM(fee_amount)
            FROM transactions_ledger
            WHERE recipient = $1
              AND block_time >= $2 AND block_time <= $3
              AND status = 'confirmed'
            GROUP BY token_mint
            "#,
        )
        .bind(merchant_wallet)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await?;

        for (mint, count, vol, fees) in token_rows {
            sqlx::query(
                r#"
                INSERT INTO settlement_token_breakdown (id, batch_id, token_mint, transaction_count, volume, fees)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(batch_id)
            .bind(&mint)
            .bind(count as i32)
            .bind(vol.unwrap_or(0))
            .bind(fees.unwrap_or(0))
            .execute(&self.pool)
            .await?;
        }

        info!(
            batch_id = %batch_id,
            merchant_id = %merchant_id,
            transactions = tx_count,
            volume = total_volume,
            "Settlement batch generated"
        );

        Ok(batch)
    }

    /// Get a settlement batch by ID.
    pub async fn get_batch(&self, batch_id: Uuid) -> Result<Option<SettlementBatch>> {
        let batch = sqlx::query_as::<_, SettlementBatch>(
            "SELECT * FROM settlement_batches WHERE id = $1"
        )
        .bind(batch_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(batch)
    }

    /// Get token breakdown for a settlement batch.
    pub async fn get_token_breakdown(&self, batch_id: Uuid) -> Result<Vec<TokenBreakdown>> {
        let breakdown = sqlx::query_as::<_, TokenBreakdown>(
            "SELECT * FROM settlement_token_breakdown WHERE batch_id = $1 ORDER BY volume DESC"
        )
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(breakdown)
    }

    /// List settlement batches for a merchant.
    pub async fn list_batches(
        &self,
        merchant_id: Uuid,
        limit: i64,
    ) -> Result<Vec<SettlementBatch>> {
        let batches = sqlx::query_as::<_, SettlementBatch>(
            r#"
            SELECT * FROM settlement_batches
            WHERE merchant_id = $1
            ORDER BY period_end DESC
            LIMIT $2
            "#,
        )
        .bind(merchant_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(batches)
    }

    /// Generate CSV report for a settlement batch.
    pub fn generate_csv(
        batch: &SettlementBatch,
        breakdown: &[TokenBreakdown],
    ) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Summary header
        wtr.write_record(&["Settlement Report"])?;
        wtr.write_record(&["Batch ID", &batch.id.to_string()])?;
        wtr.write_record(&["Merchant ID", &batch.merchant_id.to_string()])?;
        wtr.write_record(&["Period", &format!("{} to {}", batch.period_start, batch.period_end)])?;
        wtr.write_record(&["Total Transactions", &batch.total_transactions.to_string()])?;
        wtr.write_record(&["Total Volume", &batch.total_volume.to_string()])?;
        wtr.write_record(&["Total Fees", &batch.total_fees.to_string()])?;
        wtr.write_record(&["Net Settlement", &batch.net_settlement.to_string()])?;
        wtr.write_record(&["Status", &batch.status])?;
        wtr.write_record(&[""])?;

        // Token breakdown
        wtr.write_record(&["Token Breakdown"])?;
        wtr.write_record(&["Token Mint", "Transactions", "Volume", "Fees"])?;
        for t in breakdown {
            wtr.write_record(&[
                &t.token_mint,
                &t.transaction_count.to_string(),
                &t.volume.to_string(),
                &t.fees.to_string(),
            ])?;
        }

        let data = String::from_utf8(wtr.into_inner()?)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_generate_csv_empty() {
        let batch = SettlementBatch {
            id: Uuid::new_v4(),
            merchant_id: Uuid::new_v4(),
            period_start: Utc::now(),
            period_end: Utc::now(),
            total_transactions: 0,
            total_volume: 0,
            total_fees: 0,
            net_settlement: 0,
            currency_mint: "SOL".to_string(),
            status: "pending".to_string(),
            created_at: Utc::now(),
            finalized_at: None,
        };

        let csv = SettlementEngine::generate_csv(&batch, &[]).unwrap();
        assert!(csv.contains("Settlement Report"));
        assert!(csv.contains("Net Settlement"));
        assert!(csv.contains("Token Breakdown"));
    }

    #[test]
    fn test_generate_csv_with_tokens() {
        let batch_id = Uuid::new_v4();
        let batch = SettlementBatch {
            id: batch_id,
            merchant_id: Uuid::new_v4(),
            period_start: Utc::now(),
            period_end: Utc::now(),
            total_transactions: 150,
            total_volume: 5_000_000,
            total_fees: 25_000,
            net_settlement: 4_975_000,
            currency_mint: "SOL".to_string(),
            status: "confirmed".to_string(),
            created_at: Utc::now(),
            finalized_at: None,
        };

        let breakdown = vec![
            TokenBreakdown {
                id: Uuid::new_v4(),
                batch_id,
                token_mint: "SOL".to_string(),
                transaction_count: 100,
                volume: 3_000_000,
                fees: 15_000,
            },
            TokenBreakdown {
                id: Uuid::new_v4(),
                batch_id,
                token_mint: "USDC".to_string(),
                transaction_count: 50,
                volume: 2_000_000,
                fees: 10_000,
            },
        ];

        let csv = SettlementEngine::generate_csv(&batch, &breakdown).unwrap();
        assert!(csv.contains("5000000"));
        assert!(csv.contains("SOL"));
        assert!(csv.contains("USDC"));
        assert!(csv.contains("150"));
    }
}
