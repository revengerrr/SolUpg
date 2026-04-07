use solana_sdk::pubkey::Pubkey;
use solupg_common::error::AppError;
use solupg_common::types::RecipientIdentifier;

/// Resolve a recipient identifier to a Solana wallet address.
pub async fn resolve_recipient(
    http: &reqwest::Client,
    directory_url: &str,
    recipient: &RecipientIdentifier,
) -> Result<Pubkey, AppError> {
    match recipient {
        RecipientIdentifier::Wallet(addr) => {
            addr.parse::<Pubkey>()
                .map_err(|_| AppError::BadRequest(format!("invalid wallet address: {addr}")))
        }

        RecipientIdentifier::Email(email) => {
            resolve_alias(http, directory_url, email).await
        }

        RecipientIdentifier::Phone(phone) => {
            resolve_alias(http, directory_url, phone).await
        }

        RecipientIdentifier::SolDomain(domain) => {
            // TODO: Resolve via SNS on-chain lookup
            Err(AppError::BadRequest(format!("SNS resolution not yet implemented for: {domain}")))
        }

        RecipientIdentifier::Merchant(merchant_id) => {
            resolve_merchant(http, directory_url, merchant_id).await
        }
    }
}

async fn resolve_alias(
    http: &reqwest::Client,
    directory_url: &str,
    alias_value: &str,
) -> Result<Pubkey, AppError> {
    let url = format!("{directory_url}/aliases/{alias_value}");
    let resp = http.get(&url).send().await
        .map_err(|e| AppError::Internal(format!("directory service request failed: {e}")))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::NotFound(format!("alias '{alias_value}' not found")));
    }

    let body: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("failed to parse directory response: {e}")))?;

    let wallet = body["wallet_address"]
        .as_str()
        .ok_or_else(|| AppError::Internal("missing wallet_address in alias response".into()))?;

    wallet.parse::<Pubkey>()
        .map_err(|_| AppError::Internal(format!("invalid wallet in alias: {wallet}")))
}

async fn resolve_merchant(
    http: &reqwest::Client,
    directory_url: &str,
    merchant_id: &str,
) -> Result<Pubkey, AppError> {
    let url = format!("{directory_url}/merchants/{merchant_id}");
    let resp = http.get(&url).send().await
        .map_err(|e| AppError::Internal(format!("directory service request failed: {e}")))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::NotFound(format!("merchant '{merchant_id}' not found")));
    }

    let body: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("failed to parse directory response: {e}")))?;

    let wallet = body["wallet_address"]
        .as_str()
        .ok_or_else(|| AppError::Internal("missing wallet_address in merchant response".into()))?;

    wallet.parse::<Pubkey>()
        .map_err(|_| AppError::Internal(format!("invalid wallet in merchant: {wallet}")))
}
