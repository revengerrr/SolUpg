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

pub fn webhook_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_webhook).get(list_webhooks))
        .route("/{id}", get(get_webhook).put(update_webhook).delete(delete_webhook))
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub merchant_id: String,
    pub url: String,
    pub events: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub secret: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn create_webhook(
    State(state): State<AppState>,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<Json<WebhookResponse>, AppError> {
    let merchant_uuid: Uuid = req
        .merchant_id
        .parse()
        .map_err(|_| AppError::BadRequest("invalid merchant_id".to_string()))?;

    let secret = format!("whsec_{}", hex::encode(rand::random::<[u8; 24]>()));

    let row = sqlx::query_as::<_, WebhookRow>(
        "INSERT INTO webhooks (merchant_id, url, secret, events) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, merchant_id, url, secret, events, is_active, created_at",
    )
    .bind(merchant_uuid)
    .bind(&req.url)
    .bind(&secret)
    .bind(&req.events)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row.into()))
}

async fn list_webhooks(
    State(state): State<AppState>,
) -> Result<Json<Vec<WebhookResponse>>, AppError> {
    let rows = sqlx::query_as::<_, WebhookRow>(
        "SELECT id, merchant_id, url, secret, events, is_active, created_at \
         FROM webhooks WHERE is_active = true ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn get_webhook(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<WebhookResponse>, AppError> {
    let row = sqlx::query_as::<_, WebhookRow>(
        "SELECT id, merchant_id, url, secret, events, is_active, created_at \
         FROM webhooks WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("webhook {id} not found")))?;

    Ok(Json(row.into()))
}

#[derive(Debug, Deserialize)]
pub struct UpdateWebhookRequest {
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

async fn update_webhook(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWebhookRequest>,
) -> Result<Json<WebhookResponse>, AppError> {
    // Fetch current
    let mut current = sqlx::query_as::<_, WebhookRow>(
        "SELECT id, merchant_id, url, secret, events, is_active, created_at \
         FROM webhooks WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("webhook {id} not found")))?;

    if let Some(url) = req.url {
        current.url = url;
    }
    if let Some(events) = req.events {
        current.events = events;
    }
    if let Some(is_active) = req.is_active {
        current.is_active = is_active;
    }

    let row = sqlx::query_as::<_, WebhookRow>(
        "UPDATE webhooks SET url = $2, events = $3, is_active = $4, updated_at = NOW() \
         WHERE id = $1 \
         RETURNING id, merchant_id, url, secret, events, is_active, created_at",
    )
    .bind(id)
    .bind(&current.url)
    .bind(&current.events)
    .bind(current.is_active)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row.into()))
}

async fn delete_webhook(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE webhooks SET is_active = false, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[derive(Debug, sqlx::FromRow)]
struct WebhookRow {
    id: Uuid,
    merchant_id: Uuid,
    url: String,
    secret: String,
    events: Vec<String>,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<WebhookRow> for WebhookResponse {
    fn from(r: WebhookRow) -> Self {
        Self {
            id: r.id,
            merchant_id: r.merchant_id,
            url: r.url,
            events: r.events,
            secret: r.secret,
            is_active: r.is_active,
            created_at: r.created_at,
        }
    }
}
