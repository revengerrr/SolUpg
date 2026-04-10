use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use monitoring::api::{self, AppState};
use monitoring::fraud::FraudEngine;
use monitoring::metrics::MetricsCollector;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("monitoring=debug,tower_http=debug")),
        )
        .init();

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://solupg:solupg_dev@localhost:5432/solupg".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    info!("Connected to database");

    // Seed default fraud rules
    let fraud_engine = FraudEngine::new(pool.clone());
    fraud_engine.seed_default_rules().await?;

    // Initialize metrics collector
    let metrics = Arc::new(MetricsCollector::new(pool.clone()));

    // Build the monitoring API
    let state = AppState { pool, metrics };
    let app = api::router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // Start server
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3004".to_string());
    let addr = format!("{}:{}", host, port);

    info!("Monitoring service listening on {}", addr);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
