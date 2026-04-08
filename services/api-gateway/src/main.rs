mod auth;
mod middleware;
mod routes;
mod state;
pub mod webhook;

use axum::{middleware::from_fn_with_state, Router};
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
    let routing_engine_url = std::env::var("ROUTING_ENGINE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let directory_service_url = std::env::var("DIRECTORY_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "solupg_dev_jwt_secret_change_in_production".to_string());
    let port: u16 = std::env::var("API_GATEWAY_PORT")
        .unwrap_or_else(|_| "3002".to_string())
        .parse()?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("../migrations").run(&pool).await?;

    let redis_client = redis::Client::open(redis_url)?;
    let http_client = reqwest::Client::new();

    let app_state = state::AppState {
        db: pool,
        redis: redis_client,
        http: http_client,
        routing_engine_url,
        directory_service_url,
        jwt_secret,
    };

    // Protected routes (require API key + rate limiting)
    let protected = Router::new()
        .nest("/v1/payments", routes::payment_routes())
        .nest("/v1/escrows", routes::escrow_routes())
        .nest("/v1", routes::directory_routes())
        .nest("/v1/merchants", routes::merchant_routes())
        .nest("/v1/webhooks", routes::webhook_routes())
        .layer(from_fn_with_state(
            app_state.clone(),
            middleware::rate_limit_middleware,
        ))
        .layer(from_fn_with_state(
            app_state.clone(),
            auth::api_key_middleware,
        ));

    // Combine public + protected routes
    let app = Router::new()
        .merge(routes::auth_routes())
        .merge(protected)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("API Gateway listening on port {port}");
    axum::serve(listener, app).await?;

    Ok(())
}
