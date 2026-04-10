//! Solana Pay URL and QR code generation.
//!
//! This route exposes a stateless helper that turns a recipient wallet,
//! amount, and optional metadata into a Solana Pay compatible URL and a
//! base64 encoded PNG of a scannable QR code.
//!
//! The URL format follows the Solana Pay transfer request spec:
//! `solana:<recipient>?amount=<amount>&spl-token=<mint>&reference=<ref>...`
//!
//! The endpoint is deliberately stateless: it does not touch the database,
//! create a payment intent, or call the routing engine. Callers can use it
//! for ad hoc QR generation in checkout pages, POS terminals, or invoice
//! emails without having to pre register a payment.

use axum::{extract::State, routing::post, Json, Router};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{ImageFormat, Luma};
use qrcode::QrCode;
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use solupg_common::error::AppError;

pub fn solana_pay_routes() -> Router<AppState> {
    Router::new().route("/generate", post(generate_solana_pay))
}

/// Maximum QR PNG size in pixels per side. Anything larger than this is
/// pointless for mobile camera scans and just eats bandwidth.
const MAX_QR_SIZE: u32 = 2000;

/// Default PNG size per side when the caller does not specify one.
const DEFAULT_QR_SIZE: u32 = 400;

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    /// Base58 encoded recipient wallet address.
    pub recipient: String,

    /// Amount in decimal UI units as a string (for example "1.50" for 1.5 USDC).
    /// Using a string avoids floating point rounding surprises in JSON.
    pub amount: String,

    /// Optional SPL token mint address. Omit for a native SOL transfer.
    pub spl_token: Option<String>,

    /// Optional unique reference key for tracking. Must be base58 if present.
    pub reference: Option<String>,

    /// Optional merchant label shown in the paying wallet (for example "Warung Kopi").
    pub label: Option<String>,

    /// Optional human readable message shown in the paying wallet.
    pub message: Option<String>,

    /// Optional on chain memo attached to the transfer.
    pub memo: Option<String>,

    /// Optional PNG size per side in pixels. Defaults to 400, capped at 2000.
    pub qr_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct GenerateResponse {
    /// The full Solana Pay URL that wallets understand.
    pub url: String,

    /// Base64 encoded PNG of the QR. Embed directly with `data:image/png;base64,<value>`.
    pub qr_png_base64: String,

    /// Pixel size per side of the generated PNG. Echoes back the effective value.
    pub qr_size: u32,
}

async fn generate_solana_pay(
    State(_state): State<AppState>,
    Json(req): Json<GenerateRequest>,
) -> Result<Json<GenerateResponse>, AppError> {
    validate_recipient(&req.recipient)?;
    validate_amount(&req.amount)?;

    let url = build_solana_pay_url(&req);
    let qr_size = req.qr_size.unwrap_or(DEFAULT_QR_SIZE).min(MAX_QR_SIZE).max(64);
    let qr_png_base64 = render_qr_png_base64(&url, qr_size)?;

    Ok(Json(GenerateResponse {
        url,
        qr_png_base64,
        qr_size,
    }))
}

/// Builds the canonical Solana Pay URL from the request fields. All user
/// supplied string values are percent encoded because labels and messages
/// frequently contain spaces and punctuation.
fn build_solana_pay_url(req: &GenerateRequest) -> String {
    let mut url = format!("solana:{}", req.recipient);
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("amount={}", req.amount));

    if let Some(mint) = req.spl_token.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("spl-token={mint}"));
    }
    if let Some(reference) = req.reference.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("reference={reference}"));
    }
    if let Some(label) = req.label.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("label={}", urlencoding::encode(label)));
    }
    if let Some(message) = req.message.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("message={}", urlencoding::encode(message)));
    }
    if let Some(memo) = req.memo.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("memo={}", urlencoding::encode(memo)));
    }

    if !parts.is_empty() {
        url.push('?');
        url.push_str(&parts.join("&"));
    }
    url
}

/// Renders the given Solana Pay URL as a PNG QR code and returns it base64
/// encoded. Uses a grayscale luma encoding because a QR is one bit per
/// module and grayscale keeps the PNG small.
fn render_qr_png_base64(url: &str, size: u32) -> Result<String, AppError> {
    let code = QrCode::new(url.as_bytes())
        .map_err(|e| AppError::Internal(format!("qr encode: {e}")))?;

    let image = code
        .render::<Luma<u8>>()
        .min_dimensions(size, size)
        .max_dimensions(size * 2, size * 2)
        .build();

    let mut png_bytes: Vec<u8> = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        image
            .write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| AppError::Internal(format!("png encode: {e}")))?;
    }

    Ok(STANDARD.encode(&png_bytes))
}

/// Sanity check the recipient. A full base58 verification would require
/// pulling in ed25519 key parsing, which is overkill for this endpoint.
/// Instead we reject obviously wrong input: empty strings and anything that
/// is not the expected base58 length window for a Solana public key.
fn validate_recipient(recipient: &str) -> Result<(), AppError> {
    if recipient.is_empty() {
        return Err(AppError::BadRequest("recipient is required".to_string()));
    }
    let len = recipient.len();
    if !(32..=44).contains(&len) {
        return Err(AppError::BadRequest(format!(
            "recipient length {len} is outside the valid base58 range 32 to 44"
        )));
    }
    if !recipient
        .chars()
        .all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l')
    {
        return Err(AppError::BadRequest(
            "recipient contains characters outside the base58 alphabet".to_string(),
        ));
    }
    Ok(())
}

/// Validate the amount string. The Solana Pay spec allows decimal strings
/// like "1", "1.5", "0.001". We reject empty strings, scientific notation,
/// and anything that is not strictly a positive decimal.
fn validate_amount(amount: &str) -> Result<(), AppError> {
    if amount.is_empty() {
        return Err(AppError::BadRequest("amount is required".to_string()));
    }
    let parsed: f64 = amount
        .parse()
        .map_err(|_| AppError::BadRequest(format!("amount {amount} is not a valid decimal")))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(AppError::BadRequest(
            "amount must be a positive finite decimal".to_string(),
        ));
    }
    if amount.contains(['e', 'E']) {
        return Err(AppError::BadRequest(
            "amount must not use scientific notation".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> GenerateRequest {
        GenerateRequest {
            recipient: "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU".to_string(),
            amount: "1.50".to_string(),
            spl_token: Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),
            reference: None,
            label: Some("Warung Kopi".to_string()),
            message: Some("Latte and croissant".to_string()),
            memo: None,
            qr_size: Some(256),
        }
    }

    #[test]
    fn builds_url_with_all_fields() {
        let req = sample_request();
        let url = build_solana_pay_url(&req);
        assert!(url.starts_with("solana:7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU?"));
        assert!(url.contains("amount=1.50"));
        assert!(url.contains("spl-token=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));
        assert!(url.contains("label=Warung%20Kopi"));
        assert!(url.contains("message=Latte%20and%20croissant"));
    }

    #[test]
    fn builds_url_for_native_sol() {
        let req = GenerateRequest {
            recipient: "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU".to_string(),
            amount: "0.1".to_string(),
            spl_token: None,
            reference: None,
            label: None,
            message: None,
            memo: None,
            qr_size: None,
        };
        let url = build_solana_pay_url(&req);
        assert_eq!(
            url,
            "solana:7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU?amount=0.1"
        );
    }

    #[test]
    fn renders_qr_png_base64_successfully() {
        let encoded = render_qr_png_base64("solana:7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU?amount=1", 256)
            .expect("qr renders");
        let bytes = STANDARD.decode(&encoded).expect("valid base64");
        // PNG magic header is 8 bytes: 89 50 4E 47 0D 0A 1A 0A
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn rejects_empty_recipient() {
        assert!(validate_recipient("").is_err());
    }

    #[test]
    fn rejects_short_recipient() {
        assert!(validate_recipient("abc").is_err());
    }

    #[test]
    fn rejects_recipient_with_zero() {
        assert!(validate_recipient("7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosg0sU").is_err());
    }

    #[test]
    fn accepts_valid_recipient() {
        assert!(validate_recipient("7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU").is_ok());
    }

    #[test]
    fn rejects_non_positive_amount() {
        assert!(validate_amount("0").is_err());
        assert!(validate_amount("-1").is_err());
        assert!(validate_amount("").is_err());
        assert!(validate_amount("abc").is_err());
    }

    #[test]
    fn rejects_scientific_notation_amount() {
        assert!(validate_amount("1e6").is_err());
    }

    #[test]
    fn accepts_decimal_amount() {
        assert!(validate_amount("1").is_ok());
        assert!(validate_amount("1.5").is_ok());
        assert!(validate_amount("0.001").is_ok());
    }
}
