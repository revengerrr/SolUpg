use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CreateMerchantRequest, Merchant, UpdateMerchantRequest};
use solupg_common::error::AppError;

pub async fn create_merchant(
    pool: &PgPool,
    req: CreateMerchantRequest,
) -> Result<Merchant, AppError> {
    let merchant_id = req.merchant_id.unwrap_or_else(|| {
        format!("MER-{}", &Uuid::new_v4().to_string()[..8])
    });
    let merchant = sqlx::query_as::<_, Merchant>(
        r#"
        INSERT INTO merchants (id, merchant_id, name, wallet_address, preferred_token, split_config, webhook_url, kyc_status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&merchant_id)
    .bind(&req.name)
    .bind(&req.wallet_address)
    .bind(&req.preferred_token)
    .bind(&req.split_config)
    .bind(&req.webhook_url)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.constraint() == Some("merchants_merchant_id_key") => {
            AppError::Conflict("merchant_id already exists".into())
        }
        _ => AppError::Database(e),
    })?;

    Ok(merchant)
}

pub async fn get_merchant(
    pool: &PgPool,
    merchant_id: &str,
) -> Result<Merchant, AppError> {
    sqlx::query_as::<_, Merchant>(
        "SELECT * FROM merchants WHERE merchant_id = $1",
    )
    .bind(merchant_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("merchant '{merchant_id}' not found")))
}

pub async fn update_merchant(
    pool: &PgPool,
    merchant_id: &str,
    req: UpdateMerchantRequest,
) -> Result<Merchant, AppError> {
    // First check it exists
    let existing = get_merchant(pool, merchant_id).await?;

    let name = req.name.unwrap_or(existing.name);
    let wallet = req.wallet_address.unwrap_or(existing.wallet_address);
    let token = req.preferred_token.or(existing.preferred_token);
    let split = req.split_config.or(existing.split_config);
    let webhook = req.webhook_url.or(existing.webhook_url);

    let merchant = sqlx::query_as::<_, Merchant>(
        r#"
        UPDATE merchants
        SET name = $1, wallet_address = $2, preferred_token = $3, split_config = $4, webhook_url = $5, updated_at = NOW()
        WHERE merchant_id = $6
        RETURNING *
        "#,
    )
    .bind(&name)
    .bind(&wallet)
    .bind(&token)
    .bind(&split)
    .bind(&webhook)
    .bind(merchant_id)
    .fetch_one(pool)
    .await?;

    Ok(merchant)
}
