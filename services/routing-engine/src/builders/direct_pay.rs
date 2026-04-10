use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use uuid::Uuid;

use super::anchor_ix::anchor_instruction;
use solupg_common::error::AppError;
use solupg_common::pda::payment_state_pda;
use solupg_common::program_ids::payment_program_id;
use solupg_common::types::PaymentRoute;

/// SPL Token Program ID
const TOKEN_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Build a DirectPay transaction: createPayment + executePayment in one TX.
pub fn build(route: &PaymentRoute) -> Result<Transaction, AppError> {
    let program_id = payment_program_id();
    let payment_id = uuid_to_16bytes(&route.intent_id);
    let (payment_state_pda, _bump) = payment_state_pda(&route.payer, &payment_id);

    let metadata = route.metadata.clone().unwrap_or_default();

    // --- Instruction 1: create_payment(payment_id, amount, metadata) ---
    let mut create_data = Vec::new();
    create_data.extend_from_slice(&payment_id); // payment_id: [u8; 16]
    create_data.extend_from_slice(&route.amount.to_le_bytes()); // amount: u64
                                                                // Borsh string: 4-byte len + utf8 bytes
    create_data.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
    create_data.extend_from_slice(metadata.as_bytes());

    let create_accounts = vec![
        AccountMeta::new(route.payer, true), // payer (signer, mut)
        AccountMeta::new_readonly(route.recipient_wallet, false), // recipient
        AccountMeta::new_readonly(route.source_mint, false), // token_mint
        AccountMeta::new(payment_state_pda, false), // payment_state (init)
        AccountMeta::new_readonly(
            solana_sdk::pubkey!("11111111111111111111111111111111"),
            false,
        ), // system_program
    ];

    let create_ix = anchor_instruction(program_id, "create_payment", create_data, create_accounts);

    // --- Instruction 2: execute_payment() ---
    // No additional args beyond discriminator

    // Derive associated token accounts (simplified: using payer/recipient + mint)
    let payer_token_account = spl_associated_token_account(&route.payer, &route.source_mint);
    let recipient_token_account =
        spl_associated_token_account(&route.recipient_wallet, &route.source_mint);

    let execute_accounts = vec![
        AccountMeta::new(route.payer, true),        // payer (signer, mut)
        AccountMeta::new(payment_state_pda, false), // payment_state (mut)
        AccountMeta::new_readonly(route.source_mint, false), // token_mint
        AccountMeta::new(payer_token_account, false), // payer_token_account (mut)
        AccountMeta::new(recipient_token_account, false), // recipient_token_account (mut)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_program
    ];

    let execute_ix = anchor_instruction(program_id, "execute_payment", vec![], execute_accounts);

    let tx = Transaction::new_with_payer(&[create_ix, execute_ix], Some(&route.payer));

    Ok(tx)
}

/// Derive an Associated Token Account address.
fn spl_associated_token_account(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let ata_program = solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    Pubkey::find_program_address(
        &[wallet.as_ref(), TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &ata_program,
    )
    .0
}

/// Convert UUID to 16-byte array for on-chain payment_id.
fn uuid_to_16bytes(id: &Uuid) -> [u8; 16] {
    *id.as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use solupg_common::types::{PaymentRoute, RouteType};

    fn test_direct_route() -> PaymentRoute {
        PaymentRoute {
            intent_id: Uuid::new_v4(),
            route_type: RouteType::DirectPay,
            payer: Pubkey::new_from_array([1u8; 32]),
            recipient_wallet: Pubkey::new_from_array([2u8; 32]),
            source_mint: Pubkey::new_from_array([3u8; 32]),
            destination_mint: Pubkey::new_from_array([3u8; 32]),
            amount: 1_000_000,
            estimated_fee_lamports: 0,
            metadata: Some("test payment".to_string()),
            escrow_condition: None,
            escrow_expiry: None,
            split_config_pda: None,
            slippage_bps: 100,
            remaining_accounts: vec![],
        }
    }

    #[test]
    fn builds_two_instructions() {
        let route = test_direct_route();
        let tx = build(&route).unwrap();
        assert_eq!(
            tx.message.instructions.len(),
            2,
            "DirectPay TX should have create + execute"
        );
    }

    #[test]
    fn payer_is_fee_payer() {
        let route = test_direct_route();
        let tx = build(&route).unwrap();
        // First account key in message should be fee payer
        assert_eq!(tx.message.account_keys[0], route.payer);
    }

    #[test]
    fn uuid_roundtrip() {
        let id = Uuid::new_v4();
        let bytes = uuid_to_16bytes(&id);
        assert_eq!(&bytes, id.as_bytes());
    }

    #[test]
    fn ata_derivation_is_deterministic() {
        let wallet = Pubkey::new_from_array([1u8; 32]);
        let mint = Pubkey::new_from_array([3u8; 32]);
        let ata1 = spl_associated_token_account(&wallet, &mint);
        let ata2 = spl_associated_token_account(&wallet, &mint);
        assert_eq!(ata1, ata2);
    }

    #[test]
    fn different_wallets_different_atas() {
        let mint = Pubkey::new_from_array([3u8; 32]);
        let ata1 = spl_associated_token_account(&Pubkey::new_from_array([1u8; 32]), &mint);
        let ata2 = spl_associated_token_account(&Pubkey::new_from_array([2u8; 32]), &mint);
        assert_ne!(ata1, ata2);
    }
}
