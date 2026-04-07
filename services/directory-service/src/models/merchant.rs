use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Merchant {
    pub id: Uuid,
    pub merchant_id: String,
    pub name: String,
    pub wallet_address: String,
    pub preferred_token: Option<String>,
    pub split_config: Option<String>,
    pub webhook_url: Option<String>,
    pub kyc_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMerchantRequest {
    pub merchant_id: String,
    pub name: String,
    pub wallet_address: String,
    pub preferred_token: Option<String>,
    pub split_config: Option<String>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMerchantRequest {
    pub name: Option<String>,
    pub wallet_address: Option<String>,
    pub preferred_token: Option<String>,
    pub split_config: Option<String>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MerchantResponse {
    pub id: Uuid,
    pub merchant_id: String,
    pub name: String,
    pub wallet_address: String,
    pub preferred_token: Option<String>,
    pub split_config: Option<String>,
    pub webhook_url: Option<String>,
    pub kyc_status: String,
    pub created_at: DateTime<Utc>,
}

impl From<Merchant> for MerchantResponse {
    fn from(m: Merchant) -> Self {
        Self {
            id: m.id,
            merchant_id: m.merchant_id,
            name: m.name,
            wallet_address: m.wallet_address,
            preferred_token: m.preferred_token,
            split_config: m.split_config,
            webhook_url: m.webhook_url,
            kyc_status: m.kyc_status,
            created_at: m.created_at,
        }
    }
}
