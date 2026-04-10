use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;

use crate::auth::ApiKeyInfo;
use solupg_common::error::AppError;

/// Rate limit tiers (requests per minute).
fn tier_limit(tier: &str) -> u64 {
    match tier {
        "enterprise" => 10_000,
        "standard" => 1_000,
        _ => 100, // free
    }
}

/// Redis-backed sliding window rate limiter.
pub async fn rate_limit_middleware(
    State(state): State<crate::state::AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let key_info = req
        .extensions()
        .get::<ApiKeyInfo>()
        .cloned()
        .ok_or_else(|| AppError::Internal("missing API key context".to_string()))?;

    let limit = tier_limit(&key_info.tier);
    let redis_key = format!("rate:{}:{}", key_info.id, current_minute_bucket());

    let mut conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(AppError::Redis)?;

    let count: u64 = conn.incr(&redis_key, 1u64).await.map_err(AppError::Redis)?;

    if count == 1 {
        let _: () = conn.expire(&redis_key, 60).await.map_err(AppError::Redis)?;
    }

    if count > limit {
        return Err(AppError::BadRequest(format!(
            "rate limit exceeded: {count}/{limit} requests/minute"
        )));
    }

    let mut response = next.run(req).await;
    response
        .headers_mut()
        .insert("X-RateLimit-Limit", limit.to_string().parse().unwrap());
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        limit.saturating_sub(count).to_string().parse().unwrap(),
    );

    Ok(response)
}

fn current_minute_bucket() -> i64 {
    chrono::Utc::now().timestamp() / 60
}
