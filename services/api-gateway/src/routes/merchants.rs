use axum::{
    Router,
    extract::State,
    routing::{get, post},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth;
use crate::state::AppState;
use solupg_common::error::AppError;

pub fn merchant_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register_merchant))
        .route("/login", post(login_merchant))
        .route("/dashboard", get(merchant_dashboard))
        .route("/api-keys", post(create_api_key).get(list_api_keys))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterMerchantRequest {
    pub merchant_id: Option<String>,
    pub name: String,
    pub wallet_address: String,
    pub preferred_token: Option<String>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterMerchantResponse {
    pub merchant: serde_json::Value,
    pub api_key: String,
}

async fn register_merchant(
    State(state): State<AppState>,
    Json(req): Json<RegisterMerchantRequest>,
) -> Result<Json<RegisterMerchantResponse>, AppError> {
    // Create merchant via directory service
    let resp = state
        .http
        .post(format!("{}/merchants", state.directory_service_url))
        .json(&req)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("directory service error: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("directory service: {body}")));
    }

    let merchant: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("parse error: {e}")))?;

    let merchant_uuid: Uuid = merchant["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| AppError::Internal("invalid merchant id".to_string()))?;

    // Generate API key
    let api_key = auth::generate_api_key();
    let key_hash = auth::hash_api_key(&api_key);
    let key_prefix = &api_key[..12.min(api_key.len())];

    sqlx::query(
        "INSERT INTO api_keys (key_prefix, key_hash, merchant_id, name, tier) \
         VALUES ($1, $2, $3, $4, 'free')",
    )
    .bind(key_prefix)
    .bind(&key_hash)
    .bind(merchant_uuid)
    .bind(format!("{} default key", req.name))
    .execute(&state.db)
    .await?;

    Ok(Json(RegisterMerchantResponse {
        merchant,
        api_key,
    }))
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub merchant_id: String,
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64,
}

async fn login_merchant(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    // Verify merchant exists
    let resp = state
        .http
        .get(format!(
            "{}/merchants/{}",
            state.directory_service_url, req.merchant_id
        ))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("directory service error: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::NotFound("merchant not found".to_string()));
    }

    let merchant: serde_json::Value = resp.json().await.map_err(|e| {
        AppError::Internal(format!("parse error: {e}"))
    })?;

    let wallet = merchant["wallet_address"].as_str().unwrap_or_default();
    if wallet != req.wallet_address {
        return Err(AppError::BadRequest("wallet mismatch".to_string()));
    }

    let merchant_uuid: Uuid = merchant["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| AppError::Internal("invalid merchant id".to_string()))?;

    let token = auth::create_token(&state.jwt_secret, merchant_uuid, &req.merchant_id)?;

    Ok(Json(LoginResponse {
        token,
        expires_in: 86400,
    }))
}

#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub total_payments: i64,
    pub total_volume: i64,
    pub pending_payments: i64,
    pub recent_payments: Vec<DashboardPayment>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DashboardPayment {
    pub intent_id: Uuid,
    pub amount: i64,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn merchant_dashboard(
    State(state): State<AppState>,
) -> Result<Json<DashboardResponse>, AppError> {
    // For MVP, return aggregate stats from payment_intents
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM payment_intents")
        .fetch_one(&state.db)
        .await?;

    let volume: (Option<i64>,) =
        sqlx::query_as("SELECT SUM(amount) FROM payment_intents WHERE status = 'confirmed'")
            .fetch_one(&state.db)
            .await?;

    let pending: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM payment_intents WHERE status = 'pending'")
            .fetch_one(&state.db)
            .await?;

    let recent = sqlx::query_as::<_, DashboardPayment>(
        "SELECT intent_id, amount, status, created_at \
         FROM payment_intents ORDER BY created_at DESC LIMIT 10",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(DashboardResponse {
        total_payments: total.0,
        total_volume: volume.0.unwrap_or(0),
        pending_payments: pending.0,
        recent_payments: recent,
    }))
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub key_prefix: String,
    pub name: String,
    pub tier: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn create_api_key(
    State(state): State<AppState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyCreatedResponse>, AppError> {
    let key_info = req.merchant_id.as_ref().ok_or_else(|| {
        AppError::BadRequest("merchant_id required".to_string())
    })?;
    let merchant_uuid: Uuid = key_info
        .parse()
        .map_err(|_| AppError::BadRequest("invalid merchant_id".to_string()))?;

    let api_key = auth::generate_api_key();
    let key_hash = auth::hash_api_key(&api_key);
    let key_prefix = &api_key[..12.min(api_key.len())];

    sqlx::query(
        "INSERT INTO api_keys (key_prefix, key_hash, merchant_id, name, tier) \
         VALUES ($1, $2, $3, $4, 'free')",
    )
    .bind(key_prefix)
    .bind(&key_hash)
    .bind(merchant_uuid)
    .bind(&req.name)
    .execute(&state.db)
    .await?;

    Ok(Json(ApiKeyCreatedResponse {
        api_key,
        name: req.name,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub merchant_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyCreatedResponse {
    pub api_key: String,
    pub name: String,
}

async fn list_api_keys(
    State(state): State<AppState>,
) -> Result<Json<Vec<ApiKeyResponse>>, AppError> {
    let keys = sqlx::query_as::<_, ApiKeyListRow>(
        "SELECT id, key_prefix, name, tier, created_at FROM api_keys WHERE is_active = true ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        keys.into_iter()
            .map(|k| ApiKeyResponse {
                id: k.id,
                key_prefix: k.key_prefix,
                name: k.name,
                tier: k.tier,
                created_at: k.created_at,
            })
            .collect(),
    ))
}

#[derive(sqlx::FromRow)]
struct ApiKeyListRow {
    id: Uuid,
    key_prefix: String,
    name: String,
    tier: String,
    created_at: chrono::DateTime<chrono::Utc>,
}
