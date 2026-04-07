use solana_sdk::pubkey::Pubkey;
use solupg_common::error::AppError;
use solupg_common::types::{PaymentIntent, PaymentRoute, RouteType};

/// Native SOL mint (all zeros = native SOL in SPL convention).
const NATIVE_SOL_MINT: Pubkey = Pubkey::new_from_array([0u8; 32]);

/// Determine the best route for a payment intent.
pub fn plan_route(
    intent: &PaymentIntent,
    recipient_wallet: &Pubkey,
) -> Result<PaymentRoute, AppError> {
    let payer = intent.payer.parse::<Pubkey>()
        .map_err(|_| AppError::BadRequest("invalid payer pubkey".into()))?;

    let source_mint = intent.source_token.as_ref()
        .map(|s| s.parse::<Pubkey>())
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid source_token mint".into()))?
        .unwrap_or(NATIVE_SOL_MINT);

    let destination_mint = intent.destination_token.as_ref()
        .map(|s| s.parse::<Pubkey>())
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid destination_token mint".into()))?
        .unwrap_or(source_mint);

    // Determine route type
    let route_type = if let Some(forced) = intent.route_type {
        forced
    } else if intent.split_config.is_some() {
        RouteType::SplitPay
    } else if intent.escrow_condition.is_some() {
        RouteType::Escrow
    } else if source_mint != destination_mint {
        RouteType::SwapPay
    } else {
        RouteType::DirectPay
    };

    let split_config_pda = intent.split_config.as_ref()
        .map(|s| s.parse::<Pubkey>())
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid split_config PDA".into()))?;

    Ok(PaymentRoute {
        intent_id: intent.intent_id,
        route_type,
        payer,
        recipient_wallet: *recipient_wallet,
        source_mint,
        destination_mint,
        amount: intent.amount,
        estimated_fee_lamports: 0, // filled by fee calculator
        metadata: intent.metadata.clone(),
        escrow_condition: intent.escrow_condition,
        escrow_expiry: intent.escrow_expiry,
        split_config_pda,
        slippage_bps: intent.slippage_bps.unwrap_or(100), // default 1%
        remaining_accounts: vec![], // populated later by builder if needed
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solupg_common::types::{RecipientIdentifier, ReleaseCondition};
    use uuid::Uuid;

    fn test_recipient() -> Pubkey {
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap()
    }

    fn base_intent() -> PaymentIntent {
        PaymentIntent {
            intent_id: Uuid::new_v4(),
            payer: "11111111111111111111111111111112".to_string(),
            recipient: RecipientIdentifier::Wallet(test_recipient().to_string()),
            source_token: None,
            destination_token: None,
            amount: 1_000_000,
            metadata: None,
            route_type: None,
            escrow_condition: None,
            escrow_expiry: None,
            split_config: None,
            slippage_bps: None,
        }
    }

    #[test]
    fn auto_detects_direct_pay() {
        let intent = base_intent();
        let route = plan_route(&intent, &test_recipient()).unwrap();
        assert_eq!(route.route_type, RouteType::DirectPay);
        assert_eq!(route.source_mint, NATIVE_SOL_MINT);
        assert_eq!(route.destination_mint, NATIVE_SOL_MINT);
    }

    #[test]
    fn auto_detects_swap_pay() {
        let mut intent = base_intent();
        let usdc = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        intent.source_token = Some("11111111111111111111111111111112".to_string()); // SOL-like
        intent.destination_token = Some(usdc.to_string());
        let route = plan_route(&intent, &test_recipient()).unwrap();
        assert_eq!(route.route_type, RouteType::SwapPay);
    }

    #[test]
    fn auto_detects_escrow() {
        let mut intent = base_intent();
        intent.escrow_condition = Some(ReleaseCondition::TimeBased);
        intent.escrow_expiry = Some(1_700_000_000);
        let route = plan_route(&intent, &test_recipient()).unwrap();
        assert_eq!(route.route_type, RouteType::Escrow);
    }

    #[test]
    fn auto_detects_split_pay() {
        let mut intent = base_intent();
        intent.split_config = Some("11111111111111111111111111111112".to_string());
        let route = plan_route(&intent, &test_recipient()).unwrap();
        assert_eq!(route.route_type, RouteType::SplitPay);
    }

    #[test]
    fn forced_route_type_overrides_auto() {
        let mut intent = base_intent();
        intent.route_type = Some(RouteType::Escrow);
        intent.escrow_condition = Some(ReleaseCondition::AuthorityApproval);
        intent.escrow_expiry = Some(1_700_000_000);
        let route = plan_route(&intent, &test_recipient()).unwrap();
        assert_eq!(route.route_type, RouteType::Escrow);
    }

    #[test]
    fn default_slippage_is_100_bps() {
        let intent = base_intent();
        let route = plan_route(&intent, &test_recipient()).unwrap();
        assert_eq!(route.slippage_bps, 100);
    }

    #[test]
    fn custom_slippage_used() {
        let mut intent = base_intent();
        intent.slippage_bps = Some(50);
        let route = plan_route(&intent, &test_recipient()).unwrap();
        assert_eq!(route.slippage_bps, 50);
    }

    #[test]
    fn invalid_payer_rejected() {
        let mut intent = base_intent();
        intent.payer = "bad_pubkey".to_string();
        assert!(plan_route(&intent, &test_recipient()).is_err());
    }

    #[test]
    fn same_source_and_dest_token_is_direct_pay() {
        let mut intent = base_intent();
        let mint = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
        intent.source_token = Some(mint.to_string());
        intent.destination_token = Some(mint.to_string());
        let route = plan_route(&intent, &test_recipient()).unwrap();
        assert_eq!(route.route_type, RouteType::DirectPay);
    }
}
