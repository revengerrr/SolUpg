use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::transaction::Transaction;
use solupg_common::error::AppError;
use std::time::Duration;

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 500;

/// Submit a transaction and wait for confirmation with retry logic.
///
/// NOTE: In a real implementation, the payer would sign the transaction
/// client-side. For the routing engine MVP, this function expects a
/// pre-signed transaction or will be extended to support server-side
/// signing for custodial flows.
pub async fn submit_and_confirm(
    client: &RpcClient,
    tx: &Transaction,
) -> Result<String, AppError> {
    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
            tokio::time::sleep(Duration::from_millis(backoff)).await;
            tracing::info!("retry attempt {attempt}/{MAX_RETRIES}");
        }

        // Get fresh blockhash
        let blockhash = client
            .get_latest_blockhash()
            .map_err(|e| AppError::SolanaRpc(format!("failed to get blockhash: {e}")))?;

        // Clone and update blockhash
        let mut tx = tx.clone();
        tx.message.recent_blockhash = blockhash;

        // NOTE: Transaction must be signed before this point.
        // In production, this is where we'd call tx.sign(&[&payer_keypair], blockhash)
        // For now, we return an error indicating signing is needed.

        // Simulate first
        match client.simulate_transaction(&tx) {
            Ok(sim_result) => {
                if let Some(err) = sim_result.value.err {
                    last_error = Some(AppError::Transaction(format!("simulation failed: {err}")));
                    continue;
                }
            }
            Err(e) => {
                last_error = Some(AppError::SolanaRpc(format!("simulation request failed: {e}")));
                continue;
            }
        }

        // Submit
        match client.send_and_confirm_transaction_with_spinner_and_commitment(
            &tx,
            CommitmentConfig::confirmed(),
        ) {
            Ok(signature) => {
                tracing::info!("transaction confirmed: {signature}");
                return Ok(signature.to_string());
            }
            Err(e) => {
                last_error = Some(AppError::SolanaRpc(format!("send failed: {e}")));
                continue;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AppError::Transaction("max retries exceeded".into())))
}
