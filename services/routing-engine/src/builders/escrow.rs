use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use super::anchor_ix::anchor_instruction;
use solupg_common::error::AppError;
use solupg_common::pda::{escrow_state_pda, escrow_vault_pda};
use solupg_common::program_ids::escrow_program_id;
use solupg_common::types::{PaymentRoute, ReleaseCondition};

const TOKEN_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Build an Escrow transaction: createEscrow.
pub fn build(route: &PaymentRoute) -> Result<Transaction, AppError> {
    let program_id = escrow_program_id();
    let escrow_id = *route.intent_id.as_bytes();
    let (escrow_state, _) = escrow_state_pda(&route.payer, &escrow_id);
    let (escrow_vault, _) = escrow_vault_pda(&route.payer, &escrow_id);

    let release_condition = route.escrow_condition
        .ok_or_else(|| AppError::BadRequest("escrow_condition required for escrow route".into()))?;
    let expiry = route.escrow_expiry
        .ok_or_else(|| AppError::BadRequest("escrow_expiry required for escrow route".into()))?;

    // Borsh-serialize args: escrow_id: [u8;16], amount: u64, release_condition: enum(u8), expiry: i64
    let mut data = Vec::new();
    data.extend_from_slice(&escrow_id);
    data.extend_from_slice(&route.amount.to_le_bytes());
    // Anchor enum variant index
    let condition_idx: u8 = match release_condition {
        ReleaseCondition::TimeBased => 0,
        ReleaseCondition::AuthorityApproval => 1,
        ReleaseCondition::MutualApproval => 2,
    };
    data.push(condition_idx);
    data.extend_from_slice(&expiry.to_le_bytes());

    let payer_token_account = spl_associated_token_account(&route.payer, &route.source_mint);

    let accounts = vec![
        AccountMeta::new(route.payer, true),                      // payer
        AccountMeta::new_readonly(route.recipient_wallet, false),  // recipient
        AccountMeta::new_readonly(route.source_mint, false),       // token_mint
        AccountMeta::new(escrow_state, false),                     // escrow_state (init)
        AccountMeta::new(escrow_vault, false),                     // escrow_vault (init)
        AccountMeta::new(payer_token_account, false),              // payer_token_account
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),        // token_program
        AccountMeta::new_readonly(solana_sdk::pubkey!("11111111111111111111111111111111"), false), // system_program
    ];

    let ix = anchor_instruction(program_id, "create_escrow", data, accounts);

    let tx = Transaction::new_with_payer(&[ix], Some(&route.payer));
    Ok(tx)
}

fn spl_associated_token_account(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let ata_program = solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    Pubkey::find_program_address(
        &[wallet.as_ref(), TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &ata_program,
    ).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use solupg_common::types::{PaymentRoute, RouteType};
    use uuid::Uuid;

    fn test_escrow_route() -> PaymentRoute {
        PaymentRoute {
            intent_id: Uuid::new_v4(),
            route_type: RouteType::Escrow,
            payer: Pubkey::new_from_array([1u8; 32]),
            recipient_wallet: Pubkey::new_from_array([2u8; 32]),
            source_mint: Pubkey::new_from_array([3u8; 32]),
            destination_mint: Pubkey::new_from_array([3u8; 32]),
            amount: 500_000,
            estimated_fee_lamports: 0,
            metadata: None,
            escrow_condition: Some(ReleaseCondition::TimeBased),
            escrow_expiry: Some(1_700_000_000),
            split_config_pda: None,
            slippage_bps: 100,
        }
    }

    #[test]
    fn builds_one_instruction() {
        let route = test_escrow_route();
        let tx = build(&route).unwrap();
        assert_eq!(tx.message.instructions.len(), 1);
    }

    #[test]
    fn missing_condition_rejected() {
        let mut route = test_escrow_route();
        route.escrow_condition = None;
        assert!(build(&route).is_err());
    }

    #[test]
    fn missing_expiry_rejected() {
        let mut route = test_escrow_route();
        route.escrow_expiry = None;
        assert!(build(&route).is_err());
    }

    #[test]
    fn has_expected_accounts() {
        let route = test_escrow_route();
        let tx = build(&route).unwrap();
        // Account keys are deduplicated in the message; at least 8 unique accounts
        assert!(tx.message.account_keys.len() >= 8);
    }
}
