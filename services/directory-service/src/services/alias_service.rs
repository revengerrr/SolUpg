use redis::AsyncCommands;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Alias, CreateAliasRequest};
use solupg_common::error::AppError;

const ALIAS_CACHE_TTL: u64 = 300; // 5 minutes

pub async fn create_alias(
    pool: &PgPool,
    req: CreateAliasRequest,
) -> Result<Alias, AppError> {
    // Validate alias_type
    match req.alias_type.as_str() {
        "email" | "phone" | "username" => {}
        _ => return Err(AppError::BadRequest("alias_type must be email, phone, or username".into())),
    }

    let alias = sqlx::query_as::<_, Alias>(
        r#"
        INSERT INTO aliases (id, alias_type, alias_value, wallet_address, verified, created_at, updated_at)
        VALUES ($1, $2, $3, $4, false, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&req.alias_type)
    .bind(&req.alias_value)
    .bind(&req.wallet_address)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.constraint() == Some("aliases_alias_type_alias_value_key") => {
            AppError::Conflict("alias already registered".into())
        }
        _ => AppError::Database(e),
    })?;

    Ok(alias)
}

pub async fn resolve_alias(
    pool: &PgPool,
    redis: &redis::Client,
    alias_value: &str,
) -> Result<Alias, AppError> {
    // Try cache first
    if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
        let cache_key = format!("alias:{alias_value}");
        if let Ok(cached) = conn.get::<_, String>(&cache_key).await {
            if let Ok(alias) = serde_json::from_str::<Alias>(&cached) {
                return Ok(alias);
            }
        }
    }

    // Query database
    let alias = sqlx::query_as::<_, Alias>(
        "SELECT * FROM aliases WHERE alias_value = $1",
    )
    .bind(alias_value)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("alias '{alias_value}' not found")))?;

    // Cache result
    if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
        let cache_key = format!("alias:{alias_value}");
        if let Ok(json) = serde_json::to_string(&alias) {
            let _: Result<(), _> = conn.set_ex(&cache_key, &json, ALIAS_CACHE_TTL).await;
        }
    }

    Ok(alias)
}

pub async fn delete_alias(
    pool: &PgPool,
    redis: &redis::Client,
    alias_value: &str,
) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM aliases WHERE alias_value = $1")
        .bind(alias_value)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("alias '{alias_value}' not found")));
    }

    // Invalidate cache
    if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
        let cache_key = format!("alias:{alias_value}");
        let _: Result<(), _> = conn.del(&cache_key).await;
    }

    Ok(())
}
