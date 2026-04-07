use axum::{
    Router,
    extract::{Path, State},
    routing::{get, post},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::engine;
use crate::state::AppState;
use solupg_common::error::AppError;
use solupg_common::types::{IntentStatus, PaymentIntent};

#[derive(Debug, Serialize)]
pub struct IntentResponse {
    pub intent_id: Uuid,
    pub status: IntentStatus,
    pub route_type: Option<String>,
    pub tx_signature: Option<String>,
    pub error: Option<String>,
}

pub fn intent_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_intent))
        .route("/{intent_id}", get(get_intent))
}

async fn create_intent(
    State(state): State<AppState>,
    Json(intent): Json<PaymentIntent>,
) -> Result<Json<IntentResponse>, AppError> {
    let result = engine::process_intent(&state, intent).await?;
    Ok(Json(result))
}

async fn get_intent(
    State(state): State<AppState>,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<IntentResponse>, AppError> {
    let row = sqlx::query_as::<_, IntentRow>(
        "SELECT intent_id, status, route_type, tx_signature, error_message FROM payment_intents WHERE intent_id = $1"
    )
    .bind(intent_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("intent {intent_id} not found")))?;

    Ok(Json(IntentResponse {
        intent_id: row.intent_id,
        status: row.status,
        route_type: row.route_type,
        tx_signature: row.tx_signature,
        error: row.error_message,
    }))
}

#[derive(sqlx::FromRow)]
struct IntentRow {
    intent_id: Uuid,
    status: IntentStatus,
    route_type: Option<String>,
    tx_signature: Option<String>,
    error_message: Option<String>,
}
