use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use clearing_engine::dashboard::{self, AppState};
use clearing_engine::indexer::TransactionIndexer;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("clearing_engine=debug,tower_http=debug")
        }))
        .init();

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://solupg:solupg_dev@localhost:5432/solupg".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    info!("Connected to database");

    // Initialize the transaction indexer
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());

    let program_ids = vec![
        "SoLuPGPaym111111111111111111111111111111111".to_string(),
        "SoLuPGEscr111111111111111111111111111111111".to_string(),
        "SoLuPGSwap111111111111111111111111111111111".to_string(),
        "SoLuPGSplt111111111111111111111111111111111".to_string(),
    ];

    let indexer = TransactionIndexer::new(pool.clone(), rpc_url, program_ids);
    indexer.start_realtime().await?;

    // Build the dashboard API
    let state = AppState { pool };
    let app = dashboard::router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // Start server
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3003".to_string());
    let addr = format!("{}:{}", host, port);

    info!("Clearing engine listening on {}", addr);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
