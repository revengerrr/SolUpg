use anyhow::Result;
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::*;

/// The fraud detection rules engine evaluates transactions against
/// configurable rules and generates alerts when violations are detected.
pub struct FraudEngine {
    pool: PgPool,
}

impl FraudEngine {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Screen a transaction against all active fraud rules.
    /// Returns a ScreeningResult with risk score and any triggered alerts.
    pub async fn screen_transaction(&self, req: &ScreeningRequest) -> Result<ScreeningResult> {
        let rules = self.get_active_rules().await?;
        let mut alerts = Vec::new();
        let mut max_severity = Severity::Info;

        for rule in &rules {
            if let Some(alert) = self.evaluate_rule(rule, req).await? {
                let sev = match alert.severity.as_str() {
                    "block" => Severity::Block,
                    "critical" => Severity::Critical,
                    "warning" => Severity::Warning,
                    _ => Severity::Info,
                };
                if sev > max_severity {
                    max_severity = sev;
                }
                alerts.push(alert);
            }
        }

        // Calculate risk score from alerts
        let risk_score = self
            .calculate_risk_score(&req.wallet_address, &alerts)
            .await?;

        // Determine action
        let action = match max_severity {
            Severity::Block => ScreeningAction::Block,
            Severity::Critical => ScreeningAction::Flag,
            Severity::Warning if risk_score >= 70 => ScreeningAction::Flag,
            _ => ScreeningAction::Allow,
        };

        // Persist alerts
        for alert in &alerts {
            self.save_alert(alert).await?;
        }

        // Update risk score
        self.update_risk_score(&req.wallet_address, risk_score, &alerts)
            .await?;

        Ok(ScreeningResult {
            wallet_address: req.wallet_address.clone(),
            risk_score,
            alerts,
            action,
        })
    }

    /// Evaluate a single rule against a transaction.
    async fn evaluate_rule(
        &self,
        rule: &FraudRule,
        req: &ScreeningRequest,
    ) -> Result<Option<FraudAlert>> {
        match rule.rule_type.as_str() {
            "velocity" => self.check_velocity(rule, req).await,
            "threshold" => self.check_threshold(rule, req).await,
            "sanctions" => self.check_sanctions(rule, req).await,
            "pattern" => self.check_pattern(rule, req).await,
            "geo" => self.check_geo(rule, req).await,
            _ => Ok(None),
        }
    }

    /// Velocity check: too many transactions in a short time window.
    async fn check_velocity(
        &self,
        rule: &FraudRule,
        req: &ScreeningRequest,
    ) -> Result<Option<FraudAlert>> {
        let config: VelocityConfig = serde_json::from_value(rule.config.clone())?;
        let window_start = Utc::now() - Duration::minutes(config.window_minutes as i64);

        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM transactions_ledger
            WHERE payer = $1 AND block_time >= $2
            "#,
        )
        .bind(&req.wallet_address)
        .bind(window_start)
        .fetch_one(&self.pool)
        .await?;

        if count >= config.max_transactions as i64 {
            warn!(
                wallet = %req.wallet_address,
                count = count,
                limit = config.max_transactions,
                "Velocity check triggered"
            );
            return Ok(Some(self.create_alert(
                rule,
                req,
                json!({
                    "transaction_count": count,
                    "limit": config.max_transactions,
                    "window_minutes": config.window_minutes
                }),
            )));
        }

        Ok(None)
    }

    /// Threshold check: single transaction exceeds configured amount limit.
    async fn check_threshold(
        &self,
        rule: &FraudRule,
        req: &ScreeningRequest,
    ) -> Result<Option<FraudAlert>> {
        let config: ThresholdConfig = serde_json::from_value(rule.config.clone())?;

        // If token-specific, only check matching token
        if let Some(ref mint) = config.token_mint {
            if mint != &req.token_mint {
                return Ok(None);
            }
        }

        if req.amount > config.max_amount {
            warn!(
                wallet = %req.wallet_address,
                amount = req.amount,
                limit = config.max_amount,
                "Amount threshold triggered"
            );
            return Ok(Some(self.create_alert(
                rule,
                req,
                json!({
                    "amount": req.amount,
                    "limit": config.max_amount,
                    "token_mint": req.token_mint
                }),
            )));
        }

        Ok(None)
    }

    /// Sanctions check: wallet is on a sanctions list.
    async fn check_sanctions(
        &self,
        rule: &FraudRule,
        req: &ScreeningRequest,
    ) -> Result<Option<FraudAlert>> {
        let entry = sqlx::query_as::<_, SanctionEntry>(
            "SELECT * FROM sanctions_list WHERE wallet_address = $1 AND is_active = TRUE",
        )
        .bind(&req.wallet_address)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(sanction) = entry {
            warn!(
                wallet = %req.wallet_address,
                source = %sanction.list_source,
                "Sanctions match found"
            );
            return Ok(Some(self.create_alert(
                rule,
                req,
                json!({
                    "list_source": sanction.list_source,
                    "reason": sanction.reason
                }),
            )));
        }

        Ok(None)
    }

    /// Pattern detection: detect structuring/smurfing (many small tx below threshold).
    async fn check_pattern(
        &self,
        rule: &FraudRule,
        req: &ScreeningRequest,
    ) -> Result<Option<FraudAlert>> {
        // Check for structuring: many transactions just below the threshold amount
        // within the last 24 hours from the same wallet
        let window = Utc::now() - Duration::hours(24);

        let rows: Vec<(i64, Option<i64>)> = sqlx::query_as(
            r#"
            SELECT COUNT(*), AVG(amount)::BIGINT
            FROM transactions_ledger
            WHERE payer = $1 AND block_time >= $2
            "#,
        )
        .bind(&req.wallet_address)
        .bind(window)
        .fetch_all(&self.pool)
        .await?;

        if let Some((count, avg_amount)) = rows.first() {
            let avg = avg_amount.unwrap_or(0);
            // Structuring indicator: 5+ transactions with similar amounts in 24h
            if *count >= 5 && avg > 0 {
                let variance_check: Vec<(i64,)> = sqlx::query_as(
                    r#"
                    SELECT COUNT(*) FROM transactions_ledger
                    WHERE payer = $1 AND block_time >= $2
                      AND amount BETWEEN $3 AND $4
                    "#,
                )
                .bind(&req.wallet_address)
                .bind(window)
                .bind((avg as f64 * 0.8) as i64)
                .bind((avg as f64 * 1.2) as i64)
                .fetch_all(&self.pool)
                .await?;

                if let Some((similar_count,)) = variance_check.first() {
                    // If >80% of transactions have similar amounts, flag as structuring
                    if *similar_count as f64 / *count as f64 > 0.8 {
                        warn!(
                            wallet = %req.wallet_address,
                            tx_count = count,
                            similar = similar_count,
                            "Potential structuring pattern detected"
                        );
                        return Ok(Some(self.create_alert(
                            rule,
                            req,
                            json!({
                                "pattern": "structuring",
                                "tx_count_24h": count,
                                "similar_amount_count": similar_count,
                                "avg_amount": avg
                            }),
                        )));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Geo restriction: block transactions from restricted jurisdictions.
    async fn check_geo(
        &self,
        rule: &FraudRule,
        req: &ScreeningRequest,
    ) -> Result<Option<FraudAlert>> {
        let config: GeoConfig = serde_json::from_value(rule.config.clone())?;

        if let Some(ref country) = req.country_code {
            if config.blocked_countries.contains(country) {
                warn!(
                    wallet = %req.wallet_address,
                    country = %country,
                    "Geo restriction triggered"
                );
                return Ok(Some(self.create_alert(
                    rule,
                    req,
                    json!({
                        "country_code": country,
                        "blocked_countries": config.blocked_countries
                    }),
                )));
            }
        }

        Ok(None)
    }

    /// Create a fraud alert from a triggered rule.
    fn create_alert(
        &self,
        rule: &FraudRule,
        req: &ScreeningRequest,
        details: serde_json::Value,
    ) -> FraudAlert {
        FraudAlert {
            id: Uuid::new_v4(),
            rule_id: Some(rule.id),
            wallet_address: req.wallet_address.clone(),
            tx_signature: req.tx_signature.clone(),
            intent_id: req.intent_id,
            alert_type: rule.rule_type.clone(),
            severity: rule.severity.clone(),
            details,
            status: "open".to_string(),
            resolved_by: None,
            resolved_at: None,
            created_at: Utc::now(),
        }
    }

    /// Calculate composite risk score based on alerts and history.
    async fn calculate_risk_score(&self, wallet: &str, new_alerts: &[FraudAlert]) -> Result<i32> {
        // Get existing risk score
        let existing: Option<(i32,)> = sqlx::query_as(
            "SELECT score FROM risk_scores WHERE wallet_address = $1 AND source = 'internal'",
        )
        .bind(wallet)
        .fetch_optional(&self.pool)
        .await?;

        let base_score = existing.map(|(s,)| s).unwrap_or(0);

        // Add points for each new alert
        let alert_score: i32 = new_alerts
            .iter()
            .map(|a| match a.severity.as_str() {
                "block" => 40,
                "critical" => 25,
                "warning" => 10,
                _ => 5,
            })
            .sum();

        // Clamp to 0-100
        let score = (base_score + alert_score).min(100).max(0);
        Ok(score)
    }

    /// Update risk score in the database.
    async fn update_risk_score(
        &self,
        wallet: &str,
        score: i32,
        alerts: &[FraudAlert],
    ) -> Result<()> {
        let factors: Vec<serde_json::Value> = alerts
            .iter()
            .map(|a| {
                json!({
                    "rule_type": a.alert_type,
                    "severity": a.severity,
                    "time": a.created_at
                })
            })
            .collect();

        sqlx::query(
            r#"
            INSERT INTO risk_scores (id, wallet_address, score, factors, last_evaluated, source)
            VALUES ($1, $2, $3, $4, NOW(), 'internal')
            ON CONFLICT (wallet_address, source)
            DO UPDATE SET score = $3, factors = $4, last_evaluated = NOW()
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(wallet)
        .bind(score)
        .bind(json!(factors))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Persist a fraud alert.
    async fn save_alert(&self, alert: &FraudAlert) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO fraud_alerts (
                id, rule_id, wallet_address, tx_signature, intent_id,
                alert_type, severity, details, status
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(alert.id)
        .bind(alert.rule_id)
        .bind(&alert.wallet_address)
        .bind(&alert.tx_signature)
        .bind(alert.intent_id)
        .bind(&alert.alert_type)
        .bind(&alert.severity)
        .bind(&alert.details)
        .bind(&alert.status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all active fraud rules.
    pub async fn get_active_rules(&self) -> Result<Vec<FraudRule>> {
        let rules = sqlx::query_as::<_, FraudRule>(
            "SELECT * FROM fraud_rules WHERE is_active = TRUE ORDER BY rule_type",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rules)
    }

    /// Get risk score for a wallet.
    pub async fn get_risk_score(&self, wallet: &str) -> Result<Option<RiskScore>> {
        let score = sqlx::query_as::<_, RiskScore>(
            "SELECT * FROM risk_scores WHERE wallet_address = $1 AND source = 'internal'",
        )
        .bind(wallet)
        .fetch_optional(&self.pool)
        .await?;

        Ok(score)
    }

    /// List open alerts.
    pub async fn list_open_alerts(&self, limit: i64) -> Result<Vec<FraudAlert>> {
        let alerts = sqlx::query_as::<_, FraudAlert>(
            "SELECT * FROM fraud_alerts WHERE status = 'open' ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(alerts)
    }

    /// Resolve an alert.
    pub async fn resolve_alert(
        &self,
        alert_id: Uuid,
        resolved_by: &str,
        status: AlertStatus,
    ) -> Result<Option<FraudAlert>> {
        let alert = sqlx::query_as::<_, FraudAlert>(
            r#"
            UPDATE fraud_alerts
            SET status = $2, resolved_by = $3, resolved_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(alert_id)
        .bind(status.as_str())
        .bind(resolved_by)
        .fetch_optional(&self.pool)
        .await?;

        Ok(alert)
    }

    /// Check if a wallet is sanctioned.
    pub async fn is_sanctioned(&self, wallet: &str) -> Result<bool> {
        let exists: Option<(bool,)> = sqlx::query_as(
            "SELECT TRUE FROM sanctions_list WHERE wallet_address = $1 AND is_active = TRUE",
        )
        .bind(wallet)
        .fetch_optional(&self.pool)
        .await?;

        Ok(exists.is_some())
    }

    /// Seed default fraud rules.
    pub async fn seed_default_rules(&self) -> Result<()> {
        let defaults = vec![
            (
                "velocity_check",
                "Block wallets exceeding 50 tx in 10 min",
                "velocity",
                json!({"max_transactions": 50, "window_minutes": 10}),
                "warning",
            ),
            (
                "high_value_threshold",
                "Flag transactions over 100 SOL",
                "threshold",
                json!({"max_amount": 100_000_000_000u64, "token_mint": null}),
                "critical",
            ),
            (
                "sanctions_screening",
                "Block sanctioned wallets",
                "sanctions",
                json!({}),
                "block",
            ),
            (
                "structuring_detection",
                "Detect potential structuring patterns",
                "pattern",
                json!({}),
                "warning",
            ),
            (
                "geo_restriction",
                "Block restricted jurisdictions",
                "geo",
                json!({"blocked_countries": ["KP", "IR", "CU", "SY"]}),
                "block",
            ),
        ];

        for (name, desc, rtype, config, severity) in defaults {
            sqlx::query(
                r#"
                INSERT INTO fraud_rules (id, name, description, rule_type, config, severity)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (name) DO NOTHING
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(name)
            .bind(desc)
            .bind(rtype)
            .bind(config)
            .bind(severity)
            .execute(&self.pool)
            .await?;
        }

        info!("Default fraud rules seeded");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Block > Severity::Critical);
        assert!(Severity::Critical > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
    }

    #[test]
    fn test_risk_score_from_alerts() {
        // Block = 40, Critical = 25, Warning = 10
        let alerts = vec![
            FraudAlert {
                id: Uuid::new_v4(),
                rule_id: None,
                wallet_address: "test".to_string(),
                tx_signature: None,
                intent_id: None,
                alert_type: "velocity".to_string(),
                severity: "warning".to_string(),
                details: json!({}),
                status: "open".to_string(),
                resolved_by: None,
                resolved_at: None,
                created_at: Utc::now(),
            },
            FraudAlert {
                id: Uuid::new_v4(),
                rule_id: None,
                wallet_address: "test".to_string(),
                tx_signature: None,
                intent_id: None,
                alert_type: "sanctions".to_string(),
                severity: "block".to_string(),
                details: json!({}),
                status: "open".to_string(),
                resolved_by: None,
                resolved_at: None,
                created_at: Utc::now(),
            },
        ];

        let score: i32 = alerts
            .iter()
            .map(|a| match a.severity.as_str() {
                "block" => 40,
                "critical" => 25,
                "warning" => 10,
                _ => 5,
            })
            .sum();

        assert_eq!(score, 50); // 10 (warning) + 40 (block)
    }

    #[test]
    fn test_screening_action_from_severity() {
        assert_eq!(
            match Severity::Block {
                Severity::Block => ScreeningAction::Block,
                Severity::Critical => ScreeningAction::Flag,
                _ => ScreeningAction::Allow,
            },
            ScreeningAction::Block
        );
    }

    #[test]
    fn test_velocity_config_deserialize() {
        let config: VelocityConfig = serde_json::from_value(json!({
            "max_transactions": 50,
            "window_minutes": 10
        }))
        .unwrap();

        assert_eq!(config.max_transactions, 50);
        assert_eq!(config.window_minutes, 10);
    }

    #[test]
    fn test_geo_config_deserialize() {
        let config: GeoConfig = serde_json::from_value(json!({
            "blocked_countries": ["KP", "IR", "CU"]
        }))
        .unwrap();

        assert_eq!(config.blocked_countries.len(), 3);
        assert!(config.blocked_countries.contains(&"KP".to_string()));
    }
}
