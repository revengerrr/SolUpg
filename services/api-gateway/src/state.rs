use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: redis::Client,
    pub http: reqwest::Client,
    pub routing_engine_url: String,
    pub directory_service_url: String,
    pub jwt_secret: String,
}
