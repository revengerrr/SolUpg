mod models;
mod routes;
mod services;
mod state;

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
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let port: u16 = std::env::var("DIRECTORY_SERVICE_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse()?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("../migrations").run(&pool).await?;

    let redis_client = redis::Client::open(redis_url)?;

    let app_state = state::AppState {
        db: pool,
        redis: redis_client,
    };

    let app = Router::new()
        .nest("/aliases", routes::alias_routes())
        .nest("/merchants", routes::merchant_routes())
        .nest("/verify", routes::verification_routes())
        .route("/health", axum::routing::get(|| async { "ok" }))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("Directory Service listening on port {port}");
    axum::serve(listener, app).await?;

    Ok(())
}
