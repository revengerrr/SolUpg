use axum::{
    Router,
    routing::get,
    Json,
};
use serde::Serialize;

use crate::state::AppState;

/// Public routes that don't require API key auth (health, version).
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
}

async fn health() -> &'static str {
    "ok"
}

#[derive(Serialize)]
struct VersionResponse {
    version: &'static str,
    service: &'static str,
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        service: "solupg-api-gateway",
    })
}
