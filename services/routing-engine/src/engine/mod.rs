mod fee_calculator;
mod parser;
mod planner;
mod resolver;

use crate::builders;
use crate::events::PaymentEvent;
use crate::routes::intent_routes::IntentResponse;
use crate::state::AppState;
use crate::submitter;
use solupg_common::error::AppError;
use solupg_common::types::{IntentStatus, PaymentIntent};
use uuid::Uuid;

/// Main payment processing pipeline.
pub async fn process_intent(
    state: &AppState,
    intent: PaymentIntent,
) -> Result<IntentResponse, AppError> {
    // 1. Validate
    parser::validate_intent(&intent)?;

    // 2. Check idempotency
    let existing =
        sqlx::query_scalar::<_, Uuid>("SELECT intent_id FROM payment_intents WHERE intent_id = $1")
            .bind(intent.intent_id)
            .fetch_optional(&state.db)
            .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(format!(
            "intent {} already exists",
            intent.intent_id
        )));
    }

    // 3. Resolve recipient wallet
    let recipient_wallet =
        resolver::resolve_recipient(&state.http, &state.directory_url, &intent.recipient).await?;

    // 4. Plan route
    let route = planner::plan_route(&intent, &recipient_wallet)?;

    // 5. Estimate fees
    let _fees = fee_calculator::estimate_fees(&route);

    // 6. Persist intent
    sqlx::query(
        r#"
        INSERT INTO payment_intents (intent_id, payer, recipient_wallet, source_mint, destination_mint, amount, route_type, status, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', NOW())
        "#,
    )
    .bind(intent.intent_id)
    .bind(&intent.payer)
    .bind(recipient_wallet.to_string())
    .bind(route.source_mint.to_string())
    .bind(route.destination_mint.to_string())
    .bind(route.amount as i64)
    .bind(format!("{:?}", route.route_type))
    .execute(&state.db)
    .await?;

    state.event_bus.publish(PaymentEvent::IntentCreated {
        intent_id: intent.intent_id,
    });

    // 7. Build transaction
    let tx = builders::build_transaction(&route)?;

    state.event_bus.publish(PaymentEvent::RouteResolved {
        intent_id: intent.intent_id,
        route_type: format!("{:?}", route.route_type),
    });

    // 8. Submit and confirm
    match submitter::submit_and_confirm(&state.solana, &tx).await {
        Ok(signature) => {
            // Update status to confirmed
            sqlx::query(
                "UPDATE payment_intents SET status = 'confirmed', tx_signature = $1, updated_at = NOW() WHERE intent_id = $2"
            )
            .bind(&signature)
            .bind(intent.intent_id)
            .execute(&state.db)
            .await?;

            state.event_bus.publish(PaymentEvent::TransactionConfirmed {
                intent_id: intent.intent_id,
                signature: signature.clone(),
            });

            Ok(IntentResponse {
                intent_id: intent.intent_id,
                status: IntentStatus::Confirmed,
                route_type: Some(format!("{:?}", route.route_type)),
                tx_signature: Some(signature),
                error: None,
            })
        }
        Err(e) => {
            // Update status to failed
            sqlx::query(
                "UPDATE payment_intents SET status = 'failed', error_message = $1, updated_at = NOW() WHERE intent_id = $2"
            )
            .bind(e.to_string())
            .bind(intent.intent_id)
            .execute(&state.db)
            .await?;

            state.event_bus.publish(PaymentEvent::TransactionFailed {
                intent_id: intent.intent_id,
                error: e.to_string(),
            });

            Ok(IntentResponse {
                intent_id: intent.intent_id,
                status: IntentStatus::Failed,
                route_type: Some(format!("{:?}", route.route_type)),
                tx_signature: None,
                error: Some(e.to_string()),
            })
        }
    }
}
