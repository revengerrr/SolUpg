use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::models::AuditEntry;

/// Immutable audit trail system.
/// Records all significant actions for compliance and regulatory reporting.
pub struct AuditTrail {
    pool: PgPool,
}

impl AuditTrail {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Record an audit event.
    pub async fn record(
        &self,
        event_type: &str,
        actor_type: &str,
        actor_id: &str,
        resource_type: &str,
        resource_id: &str,
        action: &str,
        details: serde_json::Value,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO audit_log (
                id, event_type, actor_type, actor_id,
                resource_type, resource_id, action, details,
                ip_address, user_agent
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(id)
        .bind(event_type)
        .bind(actor_type)
        .bind(actor_id)
        .bind(resource_type)
        .bind(resource_id)
        .bind(action)
        .bind(&details)
        .bind(ip_address)
        .bind(user_agent)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Convenience: record a payment event.
    pub async fn record_payment(
        &self,
        actor_id: &str,
        payment_id: &str,
        action: &str,
        details: serde_json::Value,
    ) -> Result<Uuid> {
        self.record(
            &format!("payment.{}", action),
            "system",
            actor_id,
            "payment",
            payment_id,
            action,
            details,
            None,
            None,
        ).await
    }

    /// Convenience: record a merchant action.
    pub async fn record_merchant_action(
        &self,
        actor_id: &str,
        merchant_id: &str,
        action: &str,
        details: serde_json::Value,
        ip_address: Option<&str>,
    ) -> Result<Uuid> {
        self.record(
            &format!("merchant.{}", action),
            "merchant",
            actor_id,
            "merchant",
            merchant_id,
            action,
            details,
            ip_address,
            None,
        ).await
    }

    /// Convenience: record an admin action.
    pub async fn record_admin_action(
        &self,
        admin_id: &str,
        resource_type: &str,
        resource_id: &str,
        action: &str,
        details: serde_json::Value,
    ) -> Result<Uuid> {
        self.record(
            &format!("admin.{}", action),
            "admin",
            admin_id,
            resource_type,
            resource_id,
            action,
            details,
            None,
            None,
        ).await
    }

    /// Query audit entries with filters.
    pub async fn query(
        &self,
        event_type: Option<&str>,
        actor_id: Option<&str>,
        resource_type: Option<&str>,
        resource_id: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditEntry>> {
        let entries = sqlx::query_as::<_, AuditEntry>(
            r#"
            SELECT * FROM audit_log
            WHERE ($1::VARCHAR IS NULL OR event_type = $1)
              AND ($2::VARCHAR IS NULL OR actor_id = $2)
              AND ($3::VARCHAR IS NULL OR resource_type = $3)
              AND ($4::VARCHAR IS NULL OR resource_id = $4)
              AND ($5::TIMESTAMPTZ IS NULL OR created_at >= $5)
              AND ($6::TIMESTAMPTZ IS NULL OR created_at <= $6)
            ORDER BY created_at DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(event_type)
        .bind(actor_id)
        .bind(resource_type)
        .bind(resource_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    /// Count audit entries matching filters (for pagination).
    pub async fn count(
        &self,
        event_type: Option<&str>,
        actor_id: Option<&str>,
        resource_type: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM audit_log
            WHERE ($1::VARCHAR IS NULL OR event_type = $1)
              AND ($2::VARCHAR IS NULL OR actor_id = $2)
              AND ($3::VARCHAR IS NULL OR resource_type = $3)
              AND ($4::TIMESTAMPTZ IS NULL OR created_at >= $4)
              AND ($5::TIMESTAMPTZ IS NULL OR created_at <= $5)
            "#,
        )
        .bind(event_type)
        .bind(actor_id)
        .bind(resource_type)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Export audit entries for regulatory reporting (returns all fields).
    pub async fn export(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<AuditEntry>> {
        let entries = sqlx::query_as::<_, AuditEntry>(
            r#"
            SELECT * FROM audit_log
            WHERE created_at >= $1 AND created_at <= $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        info!(
            from = %from,
            to = %to,
            count = entries.len(),
            "Audit log exported for regulatory reporting"
        );

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_format() {
        let action = "created";
        let event_type = format!("payment.{}", action);
        assert_eq!(event_type, "payment.created");
    }

    #[test]
    fn test_merchant_event_format() {
        let action = "registered";
        let event_type = format!("merchant.{}", action);
        assert_eq!(event_type, "merchant.registered");
    }
}
