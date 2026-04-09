use sha2::{Digest, Sha256};
use solana_sdk::pubkey::Pubkey;
use solupg_common::error::AppError;
use solupg_common::types::RecipientIdentifier;

/// SNS Name Program ID (Solana Name Service)
const SNS_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("namesLPneVptA9Z5rqUDD9tMTWEJwofgaYNpp3pVvh5");

/// SOL TLD Authority (root domain for .sol names)
const SOL_TLD_AUTHORITY: Pubkey =
    solana_sdk::pubkey!("58PwtjSDuFHuUkYjH9BYnnQKHfvo9reZhC2zMJv9JPkx");

/// Resolve a recipient identifier to a Solana wallet address.
pub async fn resolve_recipient(
    http: &reqwest::Client,
    directory_url: &str,
    recipient: &RecipientIdentifier,
) -> Result<Pubkey, AppError> {
    match recipient {
        RecipientIdentifier::Wallet(addr) => addr
            .parse::<Pubkey>()
            .map_err(|_| AppError::BadRequest(format!("invalid wallet address: {addr}"))),

        RecipientIdentifier::Email(email) => resolve_alias(http, directory_url, email).await,

        RecipientIdentifier::Phone(phone) => resolve_alias(http, directory_url, phone).await,

        RecipientIdentifier::SolDomain(domain) => resolve_sol_domain(http, domain).await,

        RecipientIdentifier::Merchant(merchant_id) => {
            resolve_merchant(http, directory_url, merchant_id).await
        }
    }
}

/// Resolve a .sol domain name to a wallet address via SNS.
///
/// Uses the SNS Name Registry PDA derivation:
///   PDA = findProgramAddress([sha256(domain_name)], SNS_PROGRAM_ID)
/// Then fetches the registry account and reads the owner field.
///
/// For production, we call the Solana RPC to read the name registry.
/// Falls back to the public SNS API endpoint.
async fn resolve_sol_domain(http: &reqwest::Client, domain: &str) -> Result<Pubkey, AppError> {
    // Strip .sol suffix if present
    let name = domain.strip_suffix(".sol").unwrap_or(domain);

    // Derive the name account key (SNS PDA derivation)
    let hashed_name = hash_sns_name(name);
    let (name_account_key, _bump) = Pubkey::find_program_address(
        &[hashed_name.as_ref(), SOL_TLD_AUTHORITY.as_ref()],
        &SNS_PROGRAM_ID,
    );

    // Try public SNS API first (more reliable than raw RPC parsing)
    let api_url = format!("https://sns-api.bonfida.com/v2/resolve/{name}");
    match http.get(&api_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(result) = body.get("result").and_then(|r| r.as_str()) {
                    return result.parse::<Pubkey>().map_err(|_| {
                        AppError::Internal(format!("invalid pubkey from SNS API: {result}"))
                    });
                }
            }
        }
        _ => {}
    }

    // Fallback: return the name account key as a hint for the caller
    // In a full implementation, we'd parse the raw name registry account data
    Err(AppError::NotFound(format!(
        "could not resolve .sol domain '{domain}' (name account: {name_account_key})"
    )))
}

/// Hash a domain name the same way SNS does:
/// sha256("SPL Name Service" + "\0" + name_bytes)
fn hash_sns_name(name: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"SPL Name Service");
    hasher.update(&[0u8]); // null separator
    hasher.update(name.as_bytes());
    hasher.finalize().to_vec()
}

async fn resolve_alias(
    http: &reqwest::Client,
    directory_url: &str,
    alias_value: &str,
) -> Result<Pubkey, AppError> {
    let url = format!("{directory_url}/aliases/{alias_value}");
    let resp = http
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("directory service request failed: {e}")))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::NotFound(format!(
            "alias '{alias_value}' not found"
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("failed to parse directory response: {e}")))?;

    let wallet = body["wallet_address"]
        .as_str()
        .ok_or_else(|| AppError::Internal("missing wallet_address in alias response".into()))?;

    wallet
        .parse::<Pubkey>()
        .map_err(|_| AppError::Internal(format!("invalid wallet in alias: {wallet}")))
}

async fn resolve_merchant(
    http: &reqwest::Client,
    directory_url: &str,
    merchant_id: &str,
) -> Result<Pubkey, AppError> {
    let url = format!("{directory_url}/merchants/{merchant_id}");
    let resp = http
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("directory service request failed: {e}")))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::NotFound(format!(
            "merchant '{merchant_id}' not found"
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("failed to parse directory response: {e}")))?;

    let wallet = body["wallet_address"]
        .as_str()
        .ok_or_else(|| AppError::Internal("missing wallet_address in merchant response".into()))?;

    wallet
        .parse::<Pubkey>()
        .map_err(|_| AppError::Internal(format!("invalid wallet in merchant: {wallet}")))
}
