use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;

use super::anchor_ix::anchor_instruction;
use solupg_common::error::AppError;
use solupg_common::types::PaymentRoute;

const TOKEN_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Build a SplitPay transaction: executeSplit.
///
/// NOTE: In production, the routing engine would first fetch the SplitConfig
/// account on-chain to discover the recipients vector and build the
/// remaining_accounts. For now, we build a placeholder that assumes the
/// split_config_pda is provided and recipient accounts will be resolved
/// at submission time.
pub fn build(route: &PaymentRoute) -> Result<Transaction, AppError> {
    let program_id = solupg_common::program_ids::splitter_program_id();

    let split_config_pda = route.split_config_pda
        .ok_or_else(|| AppError::BadRequest("split_config PDA required for split route".into()))?;

    let sender_token_account = spl_associated_token_account(&route.payer, &route.source_mint);

    // Borsh-serialize args: amount: u64
    let mut data = Vec::new();
    data.extend_from_slice(&route.amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(route.payer, true),                // sender
        AccountMeta::new_readonly(split_config_pda, false), // split_config
        AccountMeta::new(sender_token_account, false),      // sender_token_account
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_program
    ];

    // TODO: In production, fetch SplitConfig on-chain to get recipients,
    // then append each recipient's ATA as remaining_accounts.
    // For now the transaction is built without remaining_accounts and will
    // need them injected before signing.

    let ix = anchor_instruction(program_id, "execute_split", data, accounts);

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

    fn test_split_route() -> PaymentRoute {
        PaymentRoute {
            intent_id: Uuid::new_v4(),
            route_type: RouteType::SplitPay,
            payer: Pubkey::new_from_array([1u8; 32]),
            recipient_wallet: Pubkey::new_from_array([2u8; 32]),
            source_mint: Pubkey::new_from_array([3u8; 32]),
            destination_mint: Pubkey::new_from_array([3u8; 32]),
            amount: 2_000_000,
            estimated_fee_lamports: 0,
            metadata: None,
            escrow_condition: None,
            escrow_expiry: None,
            split_config_pda: Some(Pubkey::new_from_array([4u8; 32])),
            slippage_bps: 100,
        }
    }

    #[test]
    fn builds_one_instruction() {
        let route = test_split_route();
        let tx = build(&route).unwrap();
        assert_eq!(tx.message.instructions.len(), 1);
    }

    #[test]
    fn missing_split_config_rejected() {
        let mut route = test_split_route();
        route.split_config_pda = None;
        assert!(build(&route).is_err());
    }
}
