use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::{MismatchType, ReconciliationMismatch, ReconciliationReport, ReconciliationRun};

/// The reconciliation engine cross-references on-chain transactions
/// with off-chain payment intents to ensure data consistency.
pub struct ReconciliationEngine {
    pool: PgPool,
}

impl ReconciliationEngine {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run a batch reconciliation for a given time period.
    /// Compares payment_intents against transactions_ledger.
    pub async fn run_batch(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<ReconciliationReport> {
        let run_id = Uuid::new_v4();

        // Create the reconciliation run record
        sqlx::query(
            r#"
            INSERT INTO reconciliation_runs (id, run_type, period_start, period_end, status)
            VALUES ($1, 'batch', $2, $3, 'running')
            "#,
        )
        .bind(run_id)
        .bind(period_start)
        .bind(period_end)
        .execute(&self.pool)
        .await?;

        info!(run_id = %run_id, "Starting batch reconciliation");

        let mut mismatches = Vec::new();

        // 1. Find payment intents with confirmed/submitted status but no matching transaction
        let missing_tx = self.find_missing_transactions(period_start, period_end).await?;
        mismatches.extend(missing_tx.iter().cloned());

        // 2. Find on-chain transactions with no matching payment intent
        let orphaned = self.find_orphaned_transactions(period_start, period_end).await?;
        mismatches.extend(orphaned.iter().cloned());

        // 3. Find amount mismatches between intents and transactions
        let amount_mismatches = self.find_amount_mismatches(period_start, period_end).await?;
        mismatches.extend(amount_mismatches.iter().cloned());

        // 4. Find status mismatches
        let status_mismatches = self.find_status_mismatches(period_start, period_end).await?;
        mismatches.extend(status_mismatches.iter().cloned());

        // Persist mismatches
        for m in &mismatches {
            self.save_mismatch(run_id, m).await?;
        }

        // Count totals
        let (total_intents,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM payment_intents
            WHERE created_at >= $1 AND created_at <= $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        let (total_transactions,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM transactions_ledger
            WHERE block_time >= $1 AND block_time <= $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        let matched = total_intents.min(total_transactions)
            - mismatches.iter().filter(|m| m.mismatch_type == MismatchType::Amount).count() as i64;

        // Finalize the run
        let run = self.finalize_run(
            run_id,
            total_intents as i32,
            total_transactions as i32,
            matched.max(0) as i32,
            &mismatches,
        ).await?;

        info!(
            run_id = %run_id,
            matched = run.matched,
            mismatched = run.mismatched,
            "Batch reconciliation complete"
        );

        Ok(ReconciliationReport { run, mismatches })
    }

    /// Find payment intents that were submitted/confirmed but have no matching on-chain tx.
    async fn find_missing_transactions(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<ReconciliationMismatch>> {
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT pi.intent_id, pi.status
            FROM payment_intents pi
            LEFT JOIN transactions_ledger tl ON tl.payment_id = pi.intent_id
            WHERE pi.created_at >= $1 AND pi.created_at <= $2
              AND pi.status IN ('submitted', 'confirmed')
              AND tl.id IS NULL
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(intent_id, _status)| {
                warn!(intent_id = %intent_id, "Missing on-chain transaction");
                ReconciliationMismatch {
                    intent_id: Some(intent_id),
                    tx_signature: None,
                    mismatch_type: MismatchType::MissingTx,
                    expected_value: Some(format!("on-chain tx for intent {}", intent_id)),
                    actual_value: Some("no matching transaction found".to_string()),
                    severity: "critical".to_string(),
                }
            })
            .collect())
    }

    /// Find on-chain transactions with no matching payment intent (orphaned).
    async fn find_orphaned_transactions(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<ReconciliationMismatch>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT tl.tx_signature
            FROM transactions_ledger tl
            LEFT JOIN payment_intents pi ON pi.intent_id = tl.payment_id
            WHERE tl.block_time >= $1 AND tl.block_time <= $2
              AND tl.payment_id IS NOT NULL
              AND pi.intent_id IS NULL
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(sig,)| {
                warn!(tx_sig = %sig, "Orphaned on-chain transaction");
                ReconciliationMismatch {
                    intent_id: None,
                    tx_signature: Some(sig.clone()),
                    mismatch_type: MismatchType::OrphanedTx,
                    expected_value: Some("matching payment intent".to_string()),
                    actual_value: Some(format!("no intent for tx {}", sig)),
                    severity: "warning".to_string(),
                }
            })
            .collect())
    }

    /// Find transactions where the amount doesn't match the intent amount.
    async fn find_amount_mismatches(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<ReconciliationMismatch>> {
        let rows: Vec<(Uuid, String, i64, i64)> = sqlx::query_as(
            r#"
            SELECT pi.intent_id, tl.tx_signature, pi.amount AS intent_amount, tl.amount AS tx_amount
            FROM payment_intents pi
            INNER JOIN transactions_ledger tl ON tl.payment_id = pi.intent_id
            WHERE pi.created_at >= $1 AND pi.created_at <= $2
              AND pi.amount != tl.amount
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(intent_id, sig, intent_amt, tx_amt)| {
                warn!(
                    intent_id = %intent_id,
                    expected = intent_amt,
                    actual = tx_amt,
                    "Amount mismatch"
                );
                ReconciliationMismatch {
                    intent_id: Some(intent_id),
                    tx_signature: Some(sig),
                    mismatch_type: MismatchType::Amount,
                    expected_value: Some(intent_amt.to_string()),
                    actual_value: Some(tx_amt.to_string()),
                    severity: "critical".to_string(),
                }
            })
            .collect())
    }

    /// Find where intent status says confirmed but tx says failed or vice versa.
    async fn find_status_mismatches(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<ReconciliationMismatch>> {
        let rows: Vec<(Uuid, String, String, String)> = sqlx::query_as(
            r#"
            SELECT pi.intent_id, tl.tx_signature, pi.status AS intent_status, tl.status AS tx_status
            FROM payment_intents pi
            INNER JOIN transactions_ledger tl ON tl.payment_id = pi.intent_id
            WHERE pi.created_at >= $1 AND pi.created_at <= $2
              AND (
                (pi.status = 'confirmed' AND tl.status = 'failed')
                OR (pi.status = 'failed' AND tl.status = 'confirmed')
              )
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(intent_id, sig, intent_status, tx_status)| {
                warn!(
                    intent_id = %intent_id,
                    intent_status = %intent_status,
                    tx_status = %tx_status,
                    "Status mismatch"
                );
                ReconciliationMismatch {
                    intent_id: Some(intent_id),
                    tx_signature: Some(sig),
                    mismatch_type: MismatchType::Status,
                    expected_value: Some(intent_status),
                    actual_value: Some(tx_status),
                    severity: "critical".to_string(),
                }
            })
            .collect())
    }

    /// Persist a mismatch to the database.
    async fn save_mismatch(
        &self,
        run_id: Uuid,
        m: &ReconciliationMismatch,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO reconciliation_mismatches (
                id, run_id, intent_id, tx_signature, mismatch_type,
                expected_value, actual_value, severity
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(run_id)
        .bind(m.intent_id)
        .bind(&m.tx_signature)
        .bind(m.mismatch_type.as_str())
        .bind(&m.expected_value)
        .bind(&m.actual_value)
        .bind(&m.severity)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Finalize a reconciliation run with computed totals.
    async fn finalize_run(
        &self,
        run_id: Uuid,
        total_intents: i32,
        total_transactions: i32,
        matched: i32,
        mismatches: &[ReconciliationMismatch],
    ) -> Result<ReconciliationRun> {
        let mismatched = mismatches
            .iter()
            .filter(|m| m.mismatch_type == MismatchType::Amount || m.mismatch_type == MismatchType::Status)
            .count() as i32;
        let orphaned = mismatches
            .iter()
            .filter(|m| m.mismatch_type == MismatchType::OrphanedTx)
            .count() as i32;
        let missing = mismatches
            .iter()
            .filter(|m| m.mismatch_type == MismatchType::MissingTx)
            .count() as i32;

        let run = sqlx::query_as::<_, ReconciliationRun>(
            r#"
            UPDATE reconciliation_runs
            SET completed_at = NOW(), status = 'completed',
                total_intents = $2, total_transactions = $3,
                matched = $4, mismatched = $5, orphaned_tx = $6, missing_tx = $7
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(run_id)
        .bind(total_intents)
        .bind(total_transactions)
        .bind(matched)
        .bind(mismatched)
        .bind(orphaned)
        .bind(missing)
        .fetch_one(&self.pool)
        .await?;

        Ok(run)
    }

    /// Get a reconciliation report by run ID.
    pub async fn get_report(&self, run_id: Uuid) -> Result<Option<ReconciliationReport>> {
        let run = sqlx::query_as::<_, ReconciliationRun>(
            "SELECT * FROM reconciliation_runs WHERE id = $1"
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(run) = run else {
            return Ok(None);
        };

        let rows: Vec<(Option<Uuid>, Option<String>, String, Option<String>, Option<String>, String)> =
            sqlx::query_as(
                r#"
                SELECT intent_id, tx_signature, mismatch_type, expected_value, actual_value, severity
                FROM reconciliation_mismatches
                WHERE run_id = $1
                ORDER BY created_at
                "#,
            )
            .bind(run_id)
            .fetch_all(&self.pool)
            .await?;

        let mismatches = rows
            .into_iter()
            .map(|(intent_id, tx_sig, mt, expected, actual, severity)| {
                let mismatch_type = match mt.as_str() {
                    "amount" => MismatchType::Amount,
                    "missing_tx" => MismatchType::MissingTx,
                    "orphaned_tx" => MismatchType::OrphanedTx,
                    _ => MismatchType::Status,
                };
                ReconciliationMismatch {
                    intent_id,
                    tx_signature: tx_sig,
                    mismatch_type,
                    expected_value: expected,
                    actual_value: actual,
                    severity,
                }
            })
            .collect();

        Ok(Some(ReconciliationReport { run, mismatches }))
    }

    /// List recent reconciliation runs.
    pub async fn list_runs(&self, limit: i64) -> Result<Vec<ReconciliationRun>> {
        let runs = sqlx::query_as::<_, ReconciliationRun>(
            "SELECT * FROM reconciliation_runs ORDER BY started_at DESC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(runs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mismatch_type_as_str() {
        assert_eq!(MismatchType::Amount.as_str(), "amount");
        assert_eq!(MismatchType::MissingTx.as_str(), "missing_tx");
        assert_eq!(MismatchType::OrphanedTx.as_str(), "orphaned_tx");
        assert_eq!(MismatchType::Status.as_str(), "status");
    }

    #[test]
    fn test_mismatch_severity() {
        let m = ReconciliationMismatch {
            intent_id: Some(Uuid::new_v4()),
            tx_signature: None,
            mismatch_type: MismatchType::MissingTx,
            expected_value: Some("on-chain tx".to_string()),
            actual_value: Some("none".to_string()),
            severity: "critical".to_string(),
        };
        assert_eq!(m.severity, "critical");
        assert!(m.tx_signature.is_none());
    }
}
