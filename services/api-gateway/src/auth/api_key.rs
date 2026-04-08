use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use solupg_common::error::AppError;

/// API key metadata extracted from the database.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub merchant_id: Option<Uuid>,
    pub tier: String,
}

/// Hash an API key for database lookup.
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Generate a new API key with prefix.
pub fn generate_api_key() -> String {
    let random_bytes: [u8; 32] = rand::random();
    format!("solupg_live_{}", hex::encode(random_bytes))
}

/// Validate an API key against the database.
pub async fn validate_api_key(pool: &PgPool, key: &str) -> Result<ApiKeyInfo, AppError> {
    let key_hash = hash_api_key(key);

    let row = sqlx::query_as::<_, ApiKeyRow>(
        "SELECT id, merchant_id, tier FROM api_keys WHERE key_hash = $1 AND is_active = true",
    )
    .bind(&key_hash)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("invalid API key".to_string()))?;

    // Update last_used_at in background (non-blocking)
    let pool = pool.clone();
    let id = row.id;
    tokio::spawn(async move {
        let _ = sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await;
    });

    Ok(ApiKeyInfo {
        id: row.id,
        merchant_id: row.merchant_id,
        tier: row.tier,
    })
}

/// Axum middleware that extracts and validates the API key from the X-API-Key header.
pub async fn api_key_middleware(
    State(state): State<crate::state::AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let api_key = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("missing X-API-Key header".to_string()))?
        .to_string();

    let info = validate_api_key(&state.db, &api_key).await?;
    req.extensions_mut().insert(info);

    Ok(next.run(req).await)
}

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: Uuid,
    merchant_id: Option<Uuid>,
    tier: String,
}
