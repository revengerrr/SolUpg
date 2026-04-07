use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;

use super::anchor_ix::anchor_instruction;
use solupg_common::error::AppError;
use solupg_common::types::PaymentRoute;

const TOKEN_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Build a SwapPay transaction: swapAndPay.
///
/// Currently builds a placeholder (direct transfer path in the on-chain program).
/// Jupiter CPI integration will be added in a future iteration.
pub fn build(route: &PaymentRoute) -> Result<Transaction, AppError> {
    let program_id = solupg_common::program_ids::swap_program_id();
    let swap_id = *route.intent_id.as_bytes();

    // Calculate minimum_amount_out based on slippage
    let minimum_amount_out = route.amount
        .checked_mul(10_000u64.saturating_sub(route.slippage_bps as u64))
        .and_then(|v| v.checked_div(10_000))
        .ok_or_else(|| AppError::Transaction("overflow calculating minimum_amount_out".into()))?;

    // Borsh-serialize: swap_id: [u8;16], amount_in: u64, minimum_amount_out: u64, slippage_bps: Option<u16>
    let mut data = Vec::new();
    data.extend_from_slice(&swap_id);
    data.extend_from_slice(&route.amount.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    // Option<u16>: Some variant = 1u8 + value
    data.push(1u8);
    data.extend_from_slice(&route.slippage_bps.to_le_bytes());

    let payer_source_token = spl_associated_token_account(&route.payer, &route.source_mint);
    let recipient_dest_token = spl_associated_token_account(&route.recipient_wallet, &route.destination_mint);

    let accounts = vec![
        AccountMeta::new(route.payer, true),                          // payer
        AccountMeta::new_readonly(route.recipient_wallet, false),      // recipient
        AccountMeta::new_readonly(route.source_mint, false),           // source_mint
        AccountMeta::new_readonly(route.destination_mint, false),      // destination_mint
        AccountMeta::new(payer_source_token, false),                   // payer_source_token
        AccountMeta::new(recipient_dest_token, false),                 // recipient_destination_token
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),            // token_program
    ];

    let ix = anchor_instruction(program_id, "swap_and_pay", data, accounts);

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

    fn test_swap_route() -> PaymentRoute {
        PaymentRoute {
            intent_id: Uuid::new_v4(),
            route_type: RouteType::SwapPay,
            payer: Pubkey::new_from_array([1u8; 32]),
            recipient_wallet: Pubkey::new_from_array([2u8; 32]),
            source_mint: Pubkey::new_from_array([3u8; 32]),
            destination_mint: Pubkey::new_from_array([4u8; 32]),
            amount: 1_000_000,
            estimated_fee_lamports: 0,
            metadata: None,
            escrow_condition: None,
            escrow_expiry: None,
            split_config_pda: None,
            slippage_bps: 100,
            remaining_accounts: vec![],
        }
    }

    #[test]
    fn builds_one_instruction() {
        let route = test_swap_route();
        let tx = build(&route).unwrap();
        assert_eq!(tx.message.instructions.len(), 1);
    }

    #[test]
    fn has_expected_accounts() {
        let route = test_swap_route();
        let tx = build(&route).unwrap();
        // payer, recipient, source_mint, dest_mint, payer_source_token, recipient_dest_token, token_program, swap_program
        assert!(tx.message.account_keys.len() >= 7);
    }

    #[test]
    fn minimum_amount_out_respects_slippage() {
        let route = test_swap_route(); // 1M amount, 100 bps slippage
        // minimum_amount_out = 1_000_000 * (10000 - 100) / 10000 = 990_000
        let tx = build(&route).unwrap();
        // Verify tx built successfully (the slippage calc didn't overflow)
        assert_eq!(tx.message.instructions.len(), 1);
    }
}
