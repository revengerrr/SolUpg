use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Deliver a webhook event to all matching subscribers.
pub async fn deliver_event(
    pool: &PgPool,
    http: &reqwest::Client,
    event_type: &str,
    payload: &serde_json::Value,
) {
    let webhooks = match sqlx::query_as::<_, WebhookTarget>(
        "SELECT id, url, secret FROM webhooks WHERE is_active = true AND $1 = ANY(events)",
    )
    .bind(event_type)
    .fetch_all(pool)
    .await
    {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("failed to fetch webhooks: {e}");
            return;
        }
    };

    for wh in webhooks {
        let pool = pool.clone();
        let http = http.clone();
        let event_type = event_type.to_string();
        let payload = payload.clone();

        tokio::spawn(async move {
            deliver_single(&pool, &http, &wh, &event_type, &payload).await;
        });
    }
}

async fn deliver_single(
    pool: &PgPool,
    http: &reqwest::Client,
    wh: &WebhookTarget,
    event_type: &str,
    payload: &serde_json::Value,
) {
    let body = serde_json::json!({
        "event": event_type,
        "data": payload,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    let body_str = serde_json::to_string(&body).unwrap_or_default();
    let signature = sign_payload(&wh.secret, &body_str);

    // Record delivery attempt
    let delivery_id = Uuid::new_v4();
    let _ = sqlx::query(
        "INSERT INTO webhook_deliveries (id, webhook_id, event_type, payload, status) \
         VALUES ($1, $2, $3, $4, 'pending')",
    )
    .bind(delivery_id)
    .bind(wh.id)
    .bind(event_type)
    .bind(&body)
    .execute(pool)
    .await;

    // Attempt delivery with retries
    let mut attempts = 0;
    let max_attempts = 3;
    let mut last_status = 0u16;

    while attempts < max_attempts {
        attempts += 1;

        match http
            .post(&wh.url)
            .header("Content-Type", "application/json")
            .header("X-SolUPG-Signature", &signature)
            .header("X-SolUPG-Event", event_type)
            .body(body_str.clone())
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) => {
                last_status = resp.status().as_u16();
                if resp.status().is_success() {
                    let _ = sqlx::query(
                        "UPDATE webhook_deliveries SET status = 'delivered', attempts = $2, \
                         response_status = $3, last_attempt_at = NOW() WHERE id = $1",
                    )
                    .bind(delivery_id)
                    .bind(attempts)
                    .bind(last_status as i32)
                    .execute(pool)
                    .await;
                    return;
                }
            }
            Err(e) => {
                tracing::warn!("webhook delivery attempt {attempts} failed: {e}");
            }
        }

        if attempts < max_attempts {
            tokio::time::sleep(std::time::Duration::from_secs(attempts as u64 * 2)).await;
        }
    }

    let _ = sqlx::query(
        "UPDATE webhook_deliveries SET status = 'failed', attempts = $2, \
         response_status = $3, last_attempt_at = NOW() WHERE id = $1",
    )
    .bind(delivery_id)
    .bind(attempts)
    .bind(if last_status > 0 {
        Some(last_status as i32)
    } else {
        None
    })
    .execute(pool)
    .await;
}

fn sign_payload(secret: &str, payload: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct WebhookTarget {
    id: Uuid,
    url: String,
    secret: String,
}
