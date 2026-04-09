use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;

use super::anchor_ix::anchor_instruction;
use solupg_common::error::AppError;
use solupg_common::types::PaymentRoute;

const TOKEN_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Build a SplitPay transaction: executeSplit.
///
/// The `remaining_accounts` field on `PaymentRoute` supplies recipient ATAs
/// discovered by the caller (e.g. by fetching the SplitConfig account
/// on-chain).  When empty, the transaction is still well-formed but will
/// fail on-chain because the program expects at least one recipient ATA
/// in remaining_accounts.
pub fn build(route: &PaymentRoute) -> Result<Transaction, AppError> {
    let program_id = solupg_common::program_ids::splitter_program_id();

    let split_config_pda = route
        .split_config_pda
        .ok_or_else(|| AppError::BadRequest("split_config PDA required for split route".into()))?;

    let sender_token_account = spl_associated_token_account(&route.payer, &route.source_mint);

    // Borsh-serialize args: amount: u64
    let mut data = Vec::new();
    data.extend_from_slice(&route.amount.to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(route.payer, true),                // sender
        AccountMeta::new_readonly(split_config_pda, false), // split_config
        AccountMeta::new(sender_token_account, false),      // sender_token_account
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_program
    ];

    // Append recipient ATAs as remaining_accounts
    for ata in &route.remaining_accounts {
        accounts.push(AccountMeta::new(*ata, false));
    }

    let ix = anchor_instruction(program_id, "execute_split", data, accounts);

    let tx = Transaction::new_with_payer(&[ix], Some(&route.payer));
    Ok(tx)
}

/// Fetch a SplitConfig account on-chain and return the recipient wallet
/// addresses stored inside it.
///
/// The on-chain layout (after 8-byte Anchor discriminator):
///   authority: Pubkey (32)
///   token_mint: Pubkey (32)
///   recipients: Vec<SplitRecipient>  (4-byte len + entries)
///     each entry: wallet: Pubkey (32), share_bps: u16 (2)
pub fn fetch_split_recipients(
    client: &solana_client::rpc_client::RpcClient,
    split_config_pda: &Pubkey,
    mint: &Pubkey,
) -> Result<Vec<Pubkey>, AppError> {
    let account = client
        .get_account(split_config_pda)
        .map_err(|e| AppError::SolanaRpc(format!("failed to fetch SplitConfig: {e}")))?;

    let data = &account.data;
    // Skip: 8 (discriminator) + 32 (authority) + 32 (token_mint) = 72
    if data.len() < 76 {
        return Err(AppError::Internal(
            "SplitConfig account data too short".into(),
        ));
    }

    let recipient_count = u32::from_le_bytes(data[72..76].try_into().unwrap()) as usize;

    let mut recipients = Vec::with_capacity(recipient_count);
    let mut offset = 76;
    for _ in 0..recipient_count {
        if offset + 34 > data.len() {
            return Err(AppError::Internal("SplitConfig data truncated".into()));
        }
        let wallet = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| AppError::Internal("invalid pubkey in SplitConfig".into()))?;
        // Derive ATA for this recipient
        let ata = spl_associated_token_account(&wallet, mint);
        recipients.push(ata);
        offset += 34; // 32 (pubkey) + 2 (share_bps)
    }

    Ok(recipients)
}

fn spl_associated_token_account(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let ata_program = solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    Pubkey::find_program_address(
        &[wallet.as_ref(), TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &ata_program,
    )
    .0
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
            remaining_accounts: vec![],
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
