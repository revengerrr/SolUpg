use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use solupg_common::error::AppError;

pub fn directory_routes() -> Router<AppState> {
    Router::new()
        .route("/aliases", post(create_alias))
        .route("/resolve/{alias}", get(resolve_alias))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAliasRequest {
    pub alias_type: String,
    pub alias_value: String,
    pub wallet_address: String,
    pub preferred_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AliasResponse {
    pub id: String,
    pub alias_type: String,
    pub alias_value: String,
    pub wallet_address: String,
    pub preferred_token: Option<String>,
    pub verified: bool,
}

async fn create_alias(
    State(state): State<AppState>,
    Json(req): Json<CreateAliasRequest>,
) -> Result<Json<AliasResponse>, AppError> {
    let resp = state
        .http
        .post(format!("{}/aliases", state.directory_service_url))
        .json(&req)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("directory service error: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("directory service: {body}")));
    }

    let result: AliasResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("parse error: {e}")))?;

    Ok(Json(result))
}

async fn resolve_alias(
    State(state): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Json<AliasResponse>, AppError> {
    let resp = state
        .http
        .get(format!("{}/aliases/{alias}", state.directory_service_url))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("directory service error: {e}")))?;

    if resp.status().as_u16() == 404 {
        return Err(AppError::NotFound(format!("alias '{alias}' not found")));
    }

    let result: AliasResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("parse error: {e}")))?;

    Ok(Json(result))
}
