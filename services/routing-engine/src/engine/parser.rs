use solupg_common::error::AppError;
use solupg_common::types::PaymentIntent;

/// Validate a payment intent before processing.
pub fn validate_intent(intent: &PaymentIntent) -> Result<(), AppError> {
    if intent.amount == 0 {
        return Err(AppError::BadRequest("amount must be greater than 0".into()));
    }

    if intent.payer.is_empty() {
        return Err(AppError::BadRequest("payer is required".into()));
    }

    // Validate payer is valid base58 pubkey
    if intent.payer.parse::<solana_sdk::pubkey::Pubkey>().is_err() {
        return Err(AppError::BadRequest("invalid payer pubkey".into()));
    }

    if let Some(ref meta) = intent.metadata {
        if meta.len() > 256 {
            return Err(AppError::BadRequest(
                "metadata exceeds 256 characters".into(),
            ));
        }
    }

    if let Some(bps) = intent.slippage_bps {
        if bps > 1000 {
            return Err(AppError::BadRequest(
                "slippage_bps cannot exceed 1000 (10%)".into(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solupg_common::types::RecipientIdentifier;
    use uuid::Uuid;

    fn valid_intent() -> PaymentIntent {
        PaymentIntent {
            intent_id: Uuid::new_v4(),
            payer: "11111111111111111111111111111112".to_string(),
            recipient: RecipientIdentifier::Wallet(
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            ),
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
    fn valid_intent_passes() {
        assert!(validate_intent(&valid_intent()).is_ok());
    }

    #[test]
    fn zero_amount_rejected() {
        let mut intent = valid_intent();
        intent.amount = 0;
        let err = validate_intent(&intent).unwrap_err();
        assert!(err.to_string().contains("amount"));
    }

    #[test]
    fn empty_payer_rejected() {
        let mut intent = valid_intent();
        intent.payer = String::new();
        let err = validate_intent(&intent).unwrap_err();
        assert!(err.to_string().contains("payer"));
    }

    #[test]
    fn invalid_payer_pubkey_rejected() {
        let mut intent = valid_intent();
        intent.payer = "not_a_valid_pubkey".to_string();
        let err = validate_intent(&intent).unwrap_err();
        assert!(err.to_string().contains("invalid payer"));
    }

    #[test]
    fn metadata_too_long_rejected() {
        let mut intent = valid_intent();
        intent.metadata = Some("x".repeat(257));
        let err = validate_intent(&intent).unwrap_err();
        assert!(err.to_string().contains("metadata"));
    }

    #[test]
    fn metadata_at_limit_accepted() {
        let mut intent = valid_intent();
        intent.metadata = Some("x".repeat(256));
        assert!(validate_intent(&intent).is_ok());
    }

    #[test]
    fn slippage_over_1000_rejected() {
        let mut intent = valid_intent();
        intent.slippage_bps = Some(1001);
        let err = validate_intent(&intent).unwrap_err();
        assert!(err.to_string().contains("slippage"));
    }

    #[test]
    fn slippage_at_1000_accepted() {
        let mut intent = valid_intent();
        intent.slippage_bps = Some(1000);
        assert!(validate_intent(&intent).is_ok());
    }
}
