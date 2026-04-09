use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};

use crate::models::{AliasResponse, CreateAliasRequest};
use crate::services;
use crate::state::AppState;
use solupg_common::error::AppError;

pub fn alias_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_alias))
        .route("/{alias}", get(resolve_alias).delete(delete_alias))
}

async fn create_alias(
    State(state): State<AppState>,
    Json(req): Json<CreateAliasRequest>,
) -> Result<Json<AliasResponse>, AppError> {
    let alias = services::create_alias(&state.db, req).await?;
    Ok(Json(alias.into()))
}

async fn resolve_alias(
    State(state): State<AppState>,
    Path(alias_value): Path<String>,
) -> Result<Json<AliasResponse>, AppError> {
    let alias = services::resolve_alias(&state.db, &state.redis, &alias_value).await?;
    Ok(Json(alias.into()))
}

async fn delete_alias(
    State(state): State<AppState>,
    Path(alias_value): Path<String>,
) -> Result<(), AppError> {
    services::delete_alias(&state.db, &state.redis, &alias_value).await
}
