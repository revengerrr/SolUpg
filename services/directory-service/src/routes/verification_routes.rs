use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use solupg_common::error::AppError;

pub fn verification_routes() -> Router<AppState> {
    Router::new()
        .route("/request", post(request_otp))
        .route("/verify", post(verify_otp))
}

#[derive(Debug, Deserialize)]
pub struct OtpRequest {
    pub alias_type: String,
    pub alias_value: String,
}

#[derive(Debug, Serialize)]
pub struct OtpResponse {
    pub message: String,
    pub expires_in_seconds: u32,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub alias_type: String,
    pub alias_value: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub verified: bool,
    pub message: String,
}

/// Request an OTP code for alias verification.
///
/// In production, this would send an email or SMS with a 6-digit code
/// and store the hash in Redis with a TTL. For the dev stub, any code works.
async fn request_otp(
    State(_state): State<AppState>,
    Json(req): Json<OtpRequest>,
) -> Result<Json<OtpResponse>, AppError> {
    match req.alias_type.as_str() {
        "email" | "phone" => {}
        _ => {
            return Err(AppError::BadRequest(
                "alias_type must be email or phone".into(),
            ))
        }
    }

    tracing::info!(
        "OTP requested for {} ({}). Dev stub: use code '123456'",
        req.alias_value,
        req.alias_type
    );

    // TODO: In production:
    // 1. Generate random 6-digit code
    // 2. Hash and store in Redis with TTL (e.g. 5 minutes)
    // 3. Send via email (SendGrid/SES) or SMS (Twilio)

    Ok(Json(OtpResponse {
        message: format!("OTP sent to {} via {}", req.alias_value, req.alias_type),
        expires_in_seconds: 300,
    }))
}

/// Verify an OTP code and mark the alias as verified.
///
/// Dev stub: accepts code "123456" for any alias.
async fn verify_otp(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError> {
    // Dev stub: accept "123456"
    if req.code != "123456" {
        return Ok(Json(VerifyResponse {
            verified: false,
            message: "invalid or expired OTP code".into(),
        }));
    }

    // TODO: In production, verify against Redis-stored hash

    // Mark alias as verified in DB
    let result = sqlx::query(
        "UPDATE aliases SET verified = true, updated_at = NOW() WHERE alias_type = $1 AND alias_value = $2"
    )
    .bind(&req.alias_type)
    .bind(&req.alias_value)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "alias '{}' ({}) not found",
            req.alias_value, req.alias_type
        )));
    }

    Ok(Json(VerifyResponse {
        verified: true,
        message: "alias verified successfully".into(),
    }))
}
