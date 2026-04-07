mod routes;
mod engine;
mod builders;
mod submitter;
mod state;
mod events;

use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://solupg:solupg_dev@localhost:5432/solupg".to_string());
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let solana_rpc = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());
    let directory_url = std::env::var("DIRECTORY_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let port: u16 = std::env::var("ROUTING_ENGINE_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("../migrations")
        .run(&pool)
        .await?;

    let redis_client = redis::Client::open(redis_url)?;
    let solana_client = solana_client::rpc_client::RpcClient::new(solana_rpc);
    let http_client = reqwest::Client::new();
    let event_bus = events::EventBus::new();

    let app_state = state::AppState {
        db: pool,
        redis: redis_client,
        solana: std::sync::Arc::new(solana_client),
        http: http_client,
        directory_url,
        event_bus,
    };

    let app = Router::new()
        .nest("/intents", routes::intent_routes())
        .route("/health", axum::routing::get(|| async { "ok" }))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("Routing Engine listening on port {port}");
    axum::serve(listener, app).await?;

    Ok(())
}
