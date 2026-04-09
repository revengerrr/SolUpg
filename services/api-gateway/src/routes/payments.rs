use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;
use solupg_common::error::AppError;

pub fn payment_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_payment).get(list_payments))
        .route("/{id}", get(get_payment))
        .route("/{id}/cancel", post(cancel_payment))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePaymentRequest {
    pub payer: String,
    pub recipient: serde_json::Value,
    pub source_token: Option<String>,
    pub destination_token: Option<String>,
    pub amount: u64,
    pub metadata: Option<String>,
    pub route_type: Option<String>,
    pub slippage_bps: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub id: Uuid,
    pub status: String,
    pub route_type: Option<String>,
    pub tx_signature: Option<String>,
    pub error: Option<String>,
}

async fn create_payment(
    State(state): State<AppState>,
    Json(req): Json<CreatePaymentRequest>,
) -> Result<Json<PaymentResponse>, AppError> {
    let resp = state
        .http
        .post(format!("{}/intents", state.routing_engine_url))
        .json(&req)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("routing engine error: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("routing engine: {body}")));
    }

    let result: PaymentResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("parse error: {e}")))?;

    Ok(Json(result))
}

async fn get_payment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PaymentResponse>, AppError> {
    let resp = state
        .http
        .get(format!("{}/intents/{id}", state.routing_engine_url))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("routing engine error: {e}")))?;

    if resp.status().as_u16() == 404 {
        return Err(AppError::NotFound(format!("payment {id} not found")));
    }

    let result: PaymentResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("parse error: {e}")))?;

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct ListPaymentsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListPaymentsResponse {
    pub payments: Vec<PaymentSummary>,
    pub total: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PaymentSummary {
    pub intent_id: Uuid,
    pub payer: String,
    pub recipient_wallet: String,
    pub amount: i64,
    pub status: String,
    pub route_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn list_payments(
    State(state): State<AppState>,
    Query(q): Query<ListPaymentsQuery>,
) -> Result<Json<ListPaymentsResponse>, AppError> {
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);

    let payments = if let Some(ref status) = q.status {
        sqlx::query_as::<_, PaymentSummary>(
            "SELECT intent_id, payer, recipient_wallet, amount, status, route_type, created_at \
             FROM payment_intents WHERE status = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, PaymentSummary>(
            "SELECT intent_id, payer, recipient_wallet, amount, status, route_type, created_at \
             FROM payment_intents ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    };

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM payment_intents")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(ListPaymentsResponse {
        payments,
        total: total.0,
    }))
}

async fn cancel_payment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PaymentResponse>, AppError> {
    let result = sqlx::query_as::<_, CancelRow>(
        "UPDATE payment_intents SET status = 'cancelled', updated_at = NOW() \
         WHERE intent_id = $1 AND status = 'pending' \
         RETURNING intent_id, status, route_type, tx_signature, error_message",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::BadRequest("payment not found or cannot be cancelled".to_string()))?;

    Ok(Json(PaymentResponse {
        id: result.intent_id,
        status: result.status,
        route_type: result.route_type,
        tx_signature: result.tx_signature,
        error: result.error_message,
    }))
}

#[derive(sqlx::FromRow)]
struct CancelRow {
    intent_id: Uuid,
    status: String,
    route_type: Option<String>,
    tx_signature: Option<String>,
    error_message: Option<String>,
}
