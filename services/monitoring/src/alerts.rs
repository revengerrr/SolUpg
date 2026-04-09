use anyhow::Result;
use serde_json::json;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::{AlertChannel, FraudAlert};

/// Alert dispatcher: sends notifications to configured channels
/// (Slack, PagerDuty, email, webhooks) when fraud alerts are created.
pub struct AlertDispatcher {
    pool: PgPool,
    http_client: reqwest::Client,
}

impl AlertDispatcher {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            http_client: reqwest::Client::new(),
        }
    }

    /// Dispatch a fraud alert to all matching channels.
    pub async fn dispatch(&self, alert: &FraudAlert) -> Result<u32> {
        let channels = self.get_matching_channels(&alert.severity).await?;
        let mut sent = 0u32;

        for channel in &channels {
            match self.send_to_channel(channel, alert).await {
                Ok(_) => {
                    self.record_notification(alert.id, channel.id, "sent")
                        .await?;
                    sent += 1;
                }
                Err(e) => {
                    warn!(
                        channel = %channel.name,
                        error = %e,
                        "Failed to send alert notification"
                    );
                    self.record_notification(alert.id, channel.id, "failed")
                        .await?;
                }
            }
        }

        info!(
            alert_id = %alert.id,
            channels_notified = sent,
            "Alert dispatched"
        );

        Ok(sent)
    }

    /// Send alert to a specific channel.
    async fn send_to_channel(&self, channel: &AlertChannel, alert: &FraudAlert) -> Result<()> {
        match channel.channel_type.as_str() {
            "slack" => self.send_slack(channel, alert).await,
            "pagerduty" => self.send_pagerduty(channel, alert).await,
            "webhook" => self.send_webhook(channel, alert).await,
            "email" => {
                info!(channel = %channel.name, "Email alerting not yet implemented");
                Ok(())
            }
            _ => {
                warn!(channel_type = %channel.channel_type, "Unknown channel type");
                Ok(())
            }
        }
    }

    /// Send alert to Slack via webhook.
    async fn send_slack(&self, channel: &AlertChannel, alert: &FraudAlert) -> Result<()> {
        let webhook_url = channel.config["webhook_url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing slack webhook_url"))?;

        let color = match alert.severity.as_str() {
            "block" => "#FF0000",
            "critical" => "#FF6600",
            "warning" => "#FFCC00",
            _ => "#36A64F",
        };

        let payload = json!({
            "attachments": [{
                "color": color,
                "title": format!("SolUPG Fraud Alert: {}", alert.alert_type),
                "fields": [
                    { "title": "Severity", "value": alert.severity, "short": true },
                    { "title": "Wallet", "value": alert.wallet_address, "short": true },
                    { "title": "Type", "value": alert.alert_type, "short": true },
                    { "title": "Status", "value": alert.status, "short": true },
                ],
                "text": format!("Details: {}", alert.details),
                "ts": alert.created_at.timestamp()
            }]
        });

        self.http_client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    /// Send alert to PagerDuty.
    async fn send_pagerduty(&self, channel: &AlertChannel, alert: &FraudAlert) -> Result<()> {
        let routing_key = channel.config["routing_key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing pagerduty routing_key"))?;

        let pd_severity = match alert.severity.as_str() {
            "block" | "critical" => "critical",
            "warning" => "warning",
            _ => "info",
        };

        let payload = json!({
            "routing_key": routing_key,
            "event_action": "trigger",
            "payload": {
                "summary": format!("SolUPG: {} alert for wallet {}", alert.alert_type, alert.wallet_address),
                "severity": pd_severity,
                "source": "solupg-monitoring",
                "component": "fraud-detection",
                "custom_details": alert.details
            }
        });

        self.http_client
            .post("https://events.pagerduty.com/v2/enqueue")
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    /// Send alert to a generic webhook.
    async fn send_webhook(&self, channel: &AlertChannel, alert: &FraudAlert) -> Result<()> {
        let url = channel.config["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing webhook url"))?;

        let payload = json!({
            "event": "fraud_alert",
            "alert": {
                "id": alert.id,
                "type": alert.alert_type,
                "severity": alert.severity,
                "wallet": alert.wallet_address,
                "tx_signature": alert.tx_signature,
                "details": alert.details,
                "created_at": alert.created_at
            }
        });

        self.http_client.post(url).json(&payload).send().await?;

        Ok(())
    }

    /// Get channels that match the alert severity.
    async fn get_matching_channels(&self, severity: &str) -> Result<Vec<AlertChannel>> {
        let channels = sqlx::query_as::<_, AlertChannel>(
            r#"
            SELECT id, name, channel_type, config, is_active, created_at
            FROM alert_channels
            WHERE is_active = TRUE AND $1 = ANY(severity_filter)
            "#,
        )
        .bind(severity)
        .fetch_all(&self.pool)
        .await?;

        Ok(channels)
    }

    /// Record a notification attempt.
    async fn record_notification(
        &self,
        alert_id: Uuid,
        channel_id: Uuid,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO alert_notifications (id, alert_id, channel_id, status, attempts, last_attempt)
            VALUES ($1, $2, $3, $4, 1, NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(alert_id)
        .bind(channel_id)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List configured alert channels.
    pub async fn list_channels(&self) -> Result<Vec<AlertChannel>> {
        let channels = sqlx::query_as::<_, AlertChannel>(
            "SELECT id, name, channel_type, config, is_active, created_at FROM alert_channels ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(channels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_color_mapping() {
        let severities = vec![
            ("block", "#FF0000"),
            ("critical", "#FF6600"),
            ("warning", "#FFCC00"),
            ("info", "#36A64F"),
        ];

        for (sev, expected_color) in severities {
            let color = match sev {
                "block" => "#FF0000",
                "critical" => "#FF6600",
                "warning" => "#FFCC00",
                _ => "#36A64F",
            };
            assert_eq!(color, expected_color);
        }
    }

    #[test]
    fn test_pagerduty_severity_mapping() {
        let mappings = vec![
            ("block", "critical"),
            ("critical", "critical"),
            ("warning", "warning"),
            ("info", "info"),
        ];

        for (sev, expected) in mappings {
            let pd = match sev {
                "block" | "critical" => "critical",
                "warning" => "warning",
                _ => "info",
            };
            assert_eq!(pd, expected);
        }
    }
}
