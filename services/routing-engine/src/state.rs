use solana_client::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;

use crate::events::EventBus;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: redis::Client,
    pub solana: Arc<RpcClient>,
    pub http: reqwest::Client,
    pub directory_url: String,
    pub event_bus: EventBus,
}
