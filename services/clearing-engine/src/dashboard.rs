use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::*;
use crate::reconciliation::ReconciliationEngine;
use crate::settlement::SettlementEngine;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

/// Build the dashboard API router.
pub fn router(state: AppState) -> Router {
    Router::new()
        // Transaction feed
        .route("/v1/dashboard/transactions", get(list_transactions))
        .route("/v1/dashboard/transactions/{id}", get(get_transaction))
        // Revenue analytics
        .route("/v1/dashboard/analytics/revenue", get(revenue_analytics))
        .route("/v1/dashboard/analytics/tokens", get(token_analytics))
        .route("/v1/dashboard/analytics/summary", get(summary_analytics))
        // Settlement
        .route("/v1/dashboard/settlements", get(list_settlements))
        .route("/v1/dashboard/settlements/{id}", get(get_settlement))
        .route("/v1/dashboard/settlements/{id}/csv", get(export_settlement_csv))
        // Reconciliation
        .route("/v1/dashboard/reconciliation/runs", get(list_recon_runs))
        .route("/v1/dashboard/reconciliation/runs/{id}", get(get_recon_report))
        // Health
        .route("/health", get(health_check))
        .with_state(state)
}

// ── Health ──

async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "clearing-engine" }))
}

// ── Transaction Feed ──

async fn list_transactions(
    State(state): State<AppState>,
    Query(params): Query<TransactionFilterParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let pagination = params.pagination();
    let date_range = params.date_range();
    let limit = pagination.limit() as i64;
    let offset = pagination.offset() as i64;
    let from = date_range.from.unwrap_or_else(|| Utc::now() - Duration::days(30));
    let to = date_range.to.unwrap_or_else(Utc::now);

    let items: Vec<TransactionFeedItem> = sqlx::query_as(
        r#"
        SELECT id, tx_signature, payer, recipient, amount, token_mint,
               fee_amount, status, block_time, instruction_type
        FROM transactions_ledger
        WHERE block_time >= $1 AND block_time <= $2
        ORDER BY block_time DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(from)
    .bind(to)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM transactions_ledger
        WHERE block_time >= $1 AND block_time <= $2
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PaginatedResponse {
        data: items,
        page: pagination.page.unwrap_or(1),
        limit: pagination.limit(),
        total,
    }))
}

async fn get_transaction(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let record = sqlx::query_as::<_, TransactionRecord>(
        "SELECT * FROM transactions_ledger WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match record {
        Some(tx) => Ok(Json(json!(tx))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ── Revenue Analytics ──

async fn revenue_analytics(
    State(state): State<AppState>,
    Query(params): Query<DateRangeParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let from = params.from.unwrap_or_else(|| Utc::now() - Duration::days(30));
    let to = params.to.unwrap_or_else(Utc::now);

    // Daily revenue stats
    let stats: Vec<RevenueStats> = sqlx::query_as(
        r#"
        SELECT
            TO_CHAR(block_time, 'YYYY-MM-DD') AS period,
            COALESCE(SUM(amount), 0) AS total_volume,
            COALESCE(SUM(fee_amount), 0) AS total_fees,
            COUNT(*) AS transaction_count
        FROM transactions_ledger
        WHERE block_time >= $1 AND block_time <= $2
          AND status = 'confirmed'
        GROUP BY TO_CHAR(block_time, 'YYYY-MM-DD')
        ORDER BY period DESC
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "from": from,
        "to": to,
        "daily": stats
    })))
}

async fn token_analytics(
    State(state): State<AppState>,
    Query(params): Query<DateRangeParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let from = params.from.unwrap_or_else(|| Utc::now() - Duration::days(30));
    let to = params.to.unwrap_or_else(Utc::now);

    // Per-token stats
    let rows: Vec<(String, i64, Option<i64>)> = sqlx::query_as(
        r#"
        SELECT token_mint, COUNT(*) AS tx_count, SUM(amount) AS total_vol
        FROM transactions_ledger
        WHERE block_time >= $1 AND block_time <= $2
          AND status = 'confirmed'
        GROUP BY token_mint
        ORDER BY tx_count DESC
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total_tx: i64 = rows.iter().map(|(_, c, _)| *c).sum();
    let stats: Vec<TokenStats> = rows
        .into_iter()
        .map(|(mint, count, vol)| TokenStats {
            token_mint: mint,
            transaction_count: count,
            total_volume: vol.unwrap_or(0),
            percentage: if total_tx > 0 {
                (count as f64 / total_tx as f64) * 100.0
            } else {
                0.0
            },
        })
        .collect();

    Ok(Json(json!({
        "from": from,
        "to": to,
        "tokens": stats
    })))
}

async fn summary_analytics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    // Overall summary stats
    let (total_tx,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM transactions_ledger WHERE status = 'confirmed'"
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (total_volume,): (Option<i64>,) = sqlx::query_as(
        "SELECT SUM(amount) FROM transactions_ledger WHERE status = 'confirmed'"
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (total_fees,): (Option<i64>,) = sqlx::query_as(
        "SELECT SUM(fee_amount) FROM transactions_ledger WHERE status = 'confirmed'"
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (unique_payers,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT payer) FROM transactions_ledger"
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (unique_recipients,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT recipient) FROM transactions_ledger"
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "total_transactions": total_tx,
        "total_volume": total_volume.unwrap_or(0),
        "total_fees": total_fees.unwrap_or(0),
        "unique_payers": unique_payers,
        "unique_recipients": unique_recipients
    })))
}

// ── Settlement ──

async fn list_settlements(
    State(state): State<AppState>,
    Query(params): Query<MerchantQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let merchant_id = params.merchant_id
        .ok_or(StatusCode::BAD_REQUEST)?;

    let engine = SettlementEngine::new(state.pool);
    let batches = engine
        .list_batches(merchant_id, 50)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "settlements": batches })))
}

async fn get_settlement(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine = SettlementEngine::new(state.pool);
    let batch = engine
        .get_batch(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let breakdown = engine
        .get_token_breakdown(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "settlement": batch,
        "token_breakdown": breakdown
    })))
}

async fn export_settlement_csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine = SettlementEngine::new(state.pool);
    let batch = engine
        .get_batch(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let breakdown = engine
        .get_token_breakdown(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let csv = SettlementEngine::generate_csv(&batch, &breakdown)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::OK,
        [
            ("content-type", "text/csv"),
            ("content-disposition", "attachment; filename=\"settlement.csv\""),
        ],
        csv,
    ))
}

// ── Reconciliation ──

async fn list_recon_runs(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine = ReconciliationEngine::new(state.pool);
    let runs = engine
        .list_runs(50)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "runs": runs })))
}

async fn get_recon_report(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine = ReconciliationEngine::new(state.pool);
    let report = engine
        .get_report(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(json!(report)))
}

// ── Helper query params ──

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MerchantQueryParams {
    pub merchant_id: Option<Uuid>,
}

// ── sqlx impls for query_as ──

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for TransactionFeedItem {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            tx_signature: row.try_get("tx_signature")?,
            payer: row.try_get("payer")?,
            recipient: row.try_get("recipient")?,
            amount: row.try_get("amount")?,
            token_mint: row.try_get("token_mint")?,
            fee_amount: row.try_get("fee_amount")?,
            status: row.try_get("status")?,
            block_time: row.try_get("block_time")?,
            instruction_type: row.try_get("instruction_type")?,
        })
    }
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for RevenueStats {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            period: row.try_get("period")?,
            total_volume: row.try_get("total_volume")?,
            total_fees: row.try_get("total_fees")?,
            transaction_count: row.try_get("transaction_count")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_defaults() {
        let params = PaginationParams {
            page: None,
            limit: None,
        };
        assert_eq!(params.offset(), 0);
        assert_eq!(params.limit(), 50);
    }

    #[test]
    fn test_pagination_custom() {
        let params = PaginationParams {
            page: Some(3),
            limit: Some(20),
        };
        assert_eq!(params.offset(), 40); // (3-1) * 20
        assert_eq!(params.limit(), 20);
    }

    #[test]
    fn test_pagination_limit_cap() {
        let params = PaginationParams {
            page: Some(1),
            limit: Some(500),
        };
        assert_eq!(params.limit(), 200); // capped at 200
    }

    #[test]
    fn test_pagination_zero_page() {
        let params = PaginationParams {
            page: Some(0),
            limit: Some(10),
        };
        assert_eq!(params.offset(), 0); // page 0 treated as page 1
    }

    #[test]
    fn test_router_creation() {
        // Verify router builds without panic (no DB needed)
        // We can't actually test this without a pool, but we verify the function compiles
        // and route definitions are valid.
    }
}
