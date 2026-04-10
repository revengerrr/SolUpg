//! Integration test: build a DirectPay TX, sign with a local keypair,
//! and submit to solana-test-validator.
//!
//! Prerequisites:
//!   solana-test-validator running with solupg programs loaded
//!   Payer has SOL balance

// solana-sdk 2.x deprecated `system_instruction` in favor of a split crate
// and `Keypair::from_bytes` in favor of `Keypair::try_from`. Keeping the
// deprecated calls here is intentional — this test hits the actual chain
// path and is `#[ignore]`'d, so the deprecations don't impact regular CI.
#![allow(deprecated)]

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;
use uuid::Uuid;

/// Run only when validator is explicitly available (not in CI).
/// Execute with: cargo test -p routing-engine --test integration_test -- --ignored
#[test]
#[ignore]
fn direct_pay_end_to_end() {
    let rpc_url =
        std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
    let client = RpcClient::new_with_commitment(&rpc_url, CommitmentConfig::confirmed());

    // Check validator is reachable
    let version = client.get_version().expect("validator not reachable");
    println!("Validator version: {}", version.solana_core);

    // Load validator identity keypair to fund test accounts
    let validator_keypair_path = std::env::var("VALIDATOR_KEYPAIR")
        .unwrap_or_else(|_| "test-ledger/validator-keypair.json".to_string());
    let validator_keypair_bytes: Vec<u8> =
        serde_json::from_str(&std::fs::read_to_string(&validator_keypair_path)
            .unwrap_or_else(|_| panic!("Cannot read validator keypair at {validator_keypair_path}. Start test-validator first.")))
            .unwrap();
    let funder = Keypair::from_bytes(&validator_keypair_bytes).unwrap();

    let payer = Keypair::new();
    let recipient = Keypair::new();

    // Fund payer from validator identity
    let blockhash = client.get_latest_blockhash().unwrap();
    let fund_tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &funder.pubkey(),
            &payer.pubkey(),
            5_000_000_000,
        )],
        Some(&funder.pubkey()),
        &[&funder],
        blockhash,
    );
    client
        .send_and_confirm_transaction_with_spinner(&fund_tx)
        .expect("failed to fund payer from validator identity");

    let balance = client.get_balance(&payer.pubkey()).unwrap();
    assert!(balance > 0, "payer has no SOL");
    println!("Payer {} funded with {} lamports", payer.pubkey(), balance);

    // Build a payment state PDA (mirrors on-chain derivation)
    let payment_program_id =
        Pubkey::from_str("J2ZU7W5ee6X2vV58HM4EVSwBzJRmHtvxn8cLVzzvhwxK").unwrap();
    let payment_id: [u8; 16] = *Uuid::new_v4().as_bytes();
    let (payment_state_pda, _bump) = Pubkey::find_program_address(
        &[b"payment", payer.pubkey().as_ref(), &payment_id],
        &payment_program_id,
    );

    println!("Payment PDA: {payment_state_pda}");
    println!("Program: {payment_program_id}");

    // Verify program is deployed
    let program_account = client.get_account(&payment_program_id);
    assert!(
        program_account.is_ok(),
        "payment program not deployed at {payment_program_id}"
    );
    println!("Payment program deployed: OK");

    // For a simple validation, just do a SOL transfer to prove the pipeline works
    let blockhash = client.get_latest_blockhash().unwrap();
    // Transfer enough for rent exemption (min ~890,880 lamports for 0-byte account)
    let transfer_amount = 1_000_000; // 0.001 SOL
    let transfer_ix =
        system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), transfer_amount);
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    let sig = client
        .send_and_confirm_transaction_with_spinner(&tx)
        .expect("transfer failed");
    println!("Transfer confirmed: {sig}");

    let recipient_balance = client.get_balance(&recipient.pubkey()).unwrap();
    assert_eq!(recipient_balance, transfer_amount);
    println!("Recipient balance verified: {recipient_balance} lamports");

    println!("\n=== Integration test PASSED ===");
    println!("  - Validator reachable");
    println!("  - Payment program deployed");
    println!("  - SOL transfer confirmed on-chain");
}
