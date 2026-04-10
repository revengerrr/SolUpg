use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::alerts::AlertDispatcher;
use crate::audit::AuditTrail;
use crate::fraud::FraudEngine;
use crate::metrics::MetricsCollector;
use crate::models::*;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub metrics: std::sync::Arc<MetricsCollector>,
}

/// Build the monitoring API router.
pub fn router(state: AppState) -> Router {
    Router::new()
        // Fraud screening
        .route("/v1/screening", post(screen_transaction))
        .route("/v1/screening/wallet/{address}", get(get_wallet_risk))
        .route("/v1/screening/sanctions/{address}", get(check_sanctions))
        // Fraud alerts
        .route("/v1/alerts", get(list_alerts))
        .route("/v1/alerts/{id}", get(get_alert))
        .route("/v1/alerts/{id}/resolve", put(resolve_alert))
        // Fraud rules
        .route("/v1/rules", get(list_rules))
        // Audit trail
        .route("/v1/audit", get(query_audit))
        .route("/v1/audit/export", get(export_audit))
        // Alert channels
        .route("/v1/channels", get(list_channels))
        // Metrics
        .route("/v1/metrics", get(get_metrics))
        .route("/v1/metrics/prometheus", get(prometheus_metrics))
        .route("/v1/metrics/transactions", get(transaction_metrics))
        // Health
        .route("/health", get(health_check))
        .with_state(state)
}

// ── Health ──

async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "monitoring" }))
}

// ── Fraud Screening ──

async fn screen_transaction(
    State(state): State<AppState>,
    Json(req): Json<ScreeningRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine = FraudEngine::new(state.pool.clone());
    let result = engine
        .screen_transaction(&req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Dispatch alerts if any were triggered
    if !result.alerts.is_empty() {
        let dispatcher = AlertDispatcher::new(state.pool);
        for alert in &result.alerts {
            let _ = dispatcher.dispatch(alert).await;
        }
    }

    let status = match result.action {
        ScreeningAction::Block => StatusCode::FORBIDDEN,
        _ => StatusCode::OK,
    };

    Ok((status, Json(json!(result))))
}

async fn get_wallet_risk(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine = FraudEngine::new(state.pool);
    let score = engine
        .get_risk_score(&address)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match score {
        Some(s) => Ok(Json(json!(s))),
        None => Ok(Json(json!({
            "wallet_address": address,
            "score": 0,
            "factors": [],
            "source": "internal",
            "message": "No risk data available"
        }))),
    }
}

async fn check_sanctions(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine = FraudEngine::new(state.pool);
    let is_sanctioned = engine
        .is_sanctioned(&address)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "wallet_address": address,
        "sanctioned": is_sanctioned
    })))
}

// ── Alerts ──

async fn list_alerts(
    State(state): State<AppState>,
    Query(params): Query<AlertListParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine = FraudEngine::new(state.pool);
    let limit = params.limit.unwrap_or(50).min(200) as i64;
    let alerts = engine
        .list_open_alerts(limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "alerts": alerts, "count": alerts.len() })))
}

async fn get_alert(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let alert = sqlx::query_as::<_, FraudAlert>("SELECT * FROM fraud_alerts WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match alert {
        Some(a) => Ok(Json(json!(a))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn resolve_alert(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveAlertRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine = FraudEngine::new(state.pool.clone());
    let status = match req.status.as_str() {
        "resolved" => AlertStatus::Resolved,
        "dismissed" => AlertStatus::Dismissed,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let alert = engine
        .resolve_alert(id, &req.resolved_by, status)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match alert {
        Some(a) => {
            // Record in audit trail
            let audit = AuditTrail::new(state.pool);
            let _ = audit
                .record_admin_action(
                    &req.resolved_by,
                    "fraud_alert",
                    &id.to_string(),
                    "resolve",
                    json!({ "new_status": req.status }),
                )
                .await;

            Ok(Json(json!(a)))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ── Rules ──

async fn list_rules(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    let engine = FraudEngine::new(state.pool);
    let rules = engine
        .get_active_rules()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "rules": rules })))
}

// ── Audit Trail ──

async fn query_audit(
    State(state): State<AppState>,
    Query(params): Query<AuditQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let audit = AuditTrail::new(state.pool);
    let limit = params.limit.unwrap_or(50).min(200) as i64;
    let offset = params.offset.unwrap_or(0) as i64;

    let entries = audit
        .query(
            params.event_type.as_deref(),
            params.actor_id.as_deref(),
            params.resource_type.as_deref(),
            params.resource_id.as_deref(),
            params.from,
            params.to,
            limit,
            offset,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "entries": entries, "count": entries.len() })))
}

async fn export_audit(
    State(state): State<AppState>,
    Query(params): Query<AuditExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let from = params.from.ok_or(StatusCode::BAD_REQUEST)?;
    let to = params.to.ok_or(StatusCode::BAD_REQUEST)?;

    let audit = AuditTrail::new(state.pool);
    let entries = audit
        .export(from, to)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "export": {
            "from": from,
            "to": to,
            "total_entries": entries.len(),
            "entries": entries
        }
    })))
}

// ── Alert Channels ──

async fn list_channels(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    let dispatcher = AlertDispatcher::new(state.pool);
    let channels = dispatcher
        .list_channels()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "channels": channels })))
}

// ── Metrics ──

async fn get_metrics(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    let svc_metrics = state.metrics.get_service_metrics("monitoring").await;
    Ok(Json(json!(svc_metrics)))
}

async fn prometheus_metrics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let output = state
        .metrics
        .export_prometheus("monitoring")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        output,
    ))
}

async fn transaction_metrics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let tx_metrics = state
        .metrics
        .get_transaction_metrics()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!(tx_metrics)))
}

// ── Request/Query types ──

#[derive(Debug, Deserialize)]
pub struct AlertListParams {
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveAlertRequest {
    pub resolved_by: String,
    pub status: String, // "resolved" or "dismissed"
}

#[derive(Debug, Deserialize)]
pub struct AuditQueryParams {
    pub event_type: Option<String>,
    pub actor_id: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct AuditExportParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}
