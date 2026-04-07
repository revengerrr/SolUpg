use axum::{
    Router,
    extract::{Path, State},
    routing::{get, post},
    Json,
};

use crate::models::{CreateMerchantRequest, MerchantResponse, UpdateMerchantRequest};
use crate::services;
use crate::state::AppState;
use solupg_common::error::AppError;

pub fn merchant_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_merchant))
        .route("/{id}", get(get_merchant).put(update_merchant))
}

async fn create_merchant(
    State(state): State<AppState>,
    Json(req): Json<CreateMerchantRequest>,
) -> Result<Json<MerchantResponse>, AppError> {
    let merchant = services::create_merchant(&state.db, req).await?;
    Ok(Json(merchant.into()))
}

async fn get_merchant(
    State(state): State<AppState>,
    Path(merchant_id): Path<String>,
) -> Result<Json<MerchantResponse>, AppError> {
    let merchant = services::get_merchant(&state.db, &merchant_id).await?;
    Ok(Json(merchant.into()))
}

async fn update_merchant(
    State(state): State<AppState>,
    Path(merchant_id): Path<String>,
    Json(req): Json<UpdateMerchantRequest>,
) -> Result<Json<MerchantResponse>, AppError> {
    let merchant = services::update_merchant(&state.db, &merchant_id, req).await?;
    Ok(Json(merchant.into()))
}
