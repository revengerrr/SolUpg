use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Alias {
    pub id: Uuid,
    pub alias_type: String,
    pub alias_value: String,
    pub wallet_address: String,
    pub preferred_token: Option<String>,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAliasRequest {
    pub alias_type: String,
    pub alias_value: String,
    pub wallet_address: String,
    pub preferred_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AliasResponse {
    pub id: Uuid,
    pub alias_type: String,
    pub alias_value: String,
    pub wallet_address: String,
    pub preferred_token: Option<String>,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Alias> for AliasResponse {
    fn from(a: Alias) -> Self {
        Self {
            id: a.id,
            alias_type: a.alias_type,
            alias_value: a.alias_value,
            wallet_address: a.wallet_address,
            preferred_token: a.preferred_token,
            verified: a.verified,
            created_at: a.created_at,
        }
    }
}
