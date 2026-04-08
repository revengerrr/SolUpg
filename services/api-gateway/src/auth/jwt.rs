use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use solupg_common::error::AppError;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub merchant_id: String,
    pub exp: i64,
    pub iat: i64,
}

/// Create a simple HMAC-signed JWT (HS256).
pub fn create_token(
    secret: &str,
    merchant_uuid: Uuid,
    merchant_id: &str,
) -> Result<String, AppError> {
    let now = Utc::now();
    let exp = now + Duration::hours(24);

    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);

    let claims = Claims {
        sub: merchant_uuid.to_string(),
        merchant_id: merchant_id.to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_string(&claims)
            .map_err(|e| AppError::Internal(format!("JSON error: {e}")))?,
    );

    let message = format!("{header}.{payload}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("HMAC error: {e}")))?;
    mac.update(message.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    Ok(format!("{message}.{signature}"))
}

/// Verify and decode a JWT.
#[allow(dead_code)]
pub fn verify_token(secret: &str, token: &str) -> Result<Claims, AppError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AppError::BadRequest("invalid token format".to_string()));
    }

    let message = format!("{}.{}", parts[0], parts[1]);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("HMAC error: {e}")))?;
    mac.update(message.as_bytes());

    let sig_bytes = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| AppError::BadRequest("invalid token signature".to_string()))?;

    mac.verify_slice(&sig_bytes)
        .map_err(|_| AppError::BadRequest("invalid token signature".to_string()))?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| AppError::BadRequest("invalid token payload".to_string()))?;

    let claims: Claims = serde_json::from_slice(&payload_bytes)
        .map_err(|e| AppError::BadRequest(format!("invalid token claims: {e}")))?;

    if claims.exp < Utc::now().timestamp() {
        return Err(AppError::BadRequest("token expired".to_string()));
    }

    Ok(claims)
}
