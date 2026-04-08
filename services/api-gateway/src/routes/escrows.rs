use axum::{
    Router,
    extract::{Path, State},
    routing::{get, post},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;
use solupg_common::error::AppError;

pub fn escrow_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_escrow))
        .route("/{id}", get(get_escrow))
        .route("/{id}/release", post(release_escrow))
        .route("/{id}/cancel", post(cancel_escrow))
        .route("/{id}/dispute", post(dispute_escrow))
}

#[derive(Debug, Deserialize)]
pub struct CreateEscrowRequest {
    pub payer: String,
    pub recipient: serde_json::Value,
    pub source_token: Option<String>,
    pub destination_token: Option<String>,
    pub amount: u64,
    pub condition: Option<String>,
    pub expiry: Option<i64>,
    pub metadata: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EscrowResponse {
    pub id: Uuid,
    pub status: String,
    pub tx_signature: Option<String>,
    pub error: Option<String>,
}

async fn create_escrow(
    State(state): State<AppState>,
    Json(req): Json<CreateEscrowRequest>,
) -> Result<Json<EscrowResponse>, AppError> {
    // Build a payment intent with escrow route type
    let intent = serde_json::json!({
        "intent_id": Uuid::new_v4(),
        "payer": req.payer,
        "recipient": req.recipient,
        "source_token": req.source_token,
        "destination_token": req.destination_token,
        "amount": req.amount,
        "route_type": "Escrow",
        "escrow_condition": req.condition,
        "escrow_expiry": req.expiry,
        "metadata": req.metadata,
    });

    let resp = state
        .http
        .post(format!("{}/intents", state.routing_engine_url))
        .json(&intent)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("routing engine error: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("routing engine: {body}")));
    }

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("parse error: {e}")))?;

    Ok(Json(EscrowResponse {
        id: result["intent_id"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        status: result["status"].as_str().unwrap_or("pending").to_string(),
        tx_signature: result["tx_signature"].as_str().map(String::from),
        error: result["error"].as_str().map(String::from),
    }))
}

async fn get_escrow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EscrowResponse>, AppError> {
    let resp = state
        .http
        .get(format!("{}/intents/{id}", state.routing_engine_url))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("routing engine error: {e}")))?;

    if resp.status().as_u16() == 404 {
        return Err(AppError::NotFound(format!("escrow {id} not found")));
    }

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("parse error: {e}")))?;

    Ok(Json(EscrowResponse {
        id,
        status: result["status"].as_str().unwrap_or("unknown").to_string(),
        tx_signature: result["tx_signature"].as_str().map(String::from),
        error: result["error"].as_str().map(String::from),
    }))
}

async fn release_escrow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EscrowResponse>, AppError> {
    // Update status to trigger release on-chain
    sqlx::query(
        "UPDATE payment_intents SET status = 'processing', updated_at = NOW() \
         WHERE intent_id = $1 AND route_type = 'Escrow' AND status = 'confirmed'",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(EscrowResponse {
        id,
        status: "releasing".to_string(),
        tx_signature: None,
        error: None,
    }))
}

async fn cancel_escrow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EscrowResponse>, AppError> {
    sqlx::query(
        "UPDATE payment_intents SET status = 'cancelled', updated_at = NOW() \
         WHERE intent_id = $1 AND route_type = 'Escrow' AND status IN ('pending', 'confirmed')",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(EscrowResponse {
        id,
        status: "cancelled".to_string(),
        tx_signature: None,
        error: None,
    }))
}

async fn dispute_escrow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EscrowResponse>, AppError> {
    sqlx::query(
        "UPDATE payment_intents SET status = 'processing', error_message = 'disputed', updated_at = NOW() \
         WHERE intent_id = $1 AND route_type = 'Escrow' AND status = 'confirmed'",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(EscrowResponse {
        id,
        status: "disputed".to_string(),
        tx_signature: None,
        error: None,
    }))
}
