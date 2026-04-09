//! Pure unit tests for directory-service — no DB required.
//!
//! These cover model conversions, request validation, and serde round-trips.
//! DB-backed tests live in `db_test.rs` (ignored by default).

use chrono::{TimeZone, Utc};
use directory_service::models::{
    Alias, AliasResponse, CreateAliasRequest, CreateMerchantRequest, Merchant, MerchantResponse,
    UpdateMerchantRequest,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Alias model
// ---------------------------------------------------------------------------

fn sample_alias() -> Alias {
    Alias {
        id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        alias_type: "email".to_string(),
        alias_value: "alice@example.com".to_string(),
        wallet_address: "7Np4FdvXdXm3G8kP1cT2vwL6o2X8RzH9sLXJs2wQfYbA".to_string(),
        preferred_token: Some("USDC".to_string()),
        verified: false,
        created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
    }
}

#[test]
fn alias_response_preserves_public_fields() {
    let alias = sample_alias();
    let response: AliasResponse = alias.clone().into();

    assert_eq!(response.id, alias.id);
    assert_eq!(response.alias_type, alias.alias_type);
    assert_eq!(response.alias_value, alias.alias_value);
    assert_eq!(response.wallet_address, alias.wallet_address);
    assert_eq!(response.preferred_token, alias.preferred_token);
    assert_eq!(response.verified, alias.verified);
    assert_eq!(response.created_at, alias.created_at);
}

#[test]
fn alias_response_omits_updated_at() {
    // Public response struct should only expose created_at, not updated_at.
    let response: AliasResponse = sample_alias().into();
    let json = serde_json::to_value(&response).unwrap();
    assert!(json.get("updated_at").is_none(), "updated_at must not be exposed on public responses");
    assert!(json.get("created_at").is_some());
}

#[test]
fn create_alias_request_deserializes() {
    let body = serde_json::json!({
        "alias_type": "email",
        "alias_value": "bob@example.com",
        "wallet_address": "9xQeWvG816bUx9EPjHmaT2aRzZ4vWQ9w7rGkmM2qAqQ",
        "preferred_token": "SOL"
    });
    let req: CreateAliasRequest = serde_json::from_value(body).unwrap();
    assert_eq!(req.alias_type, "email");
    assert_eq!(req.alias_value, "bob@example.com");
    assert_eq!(req.preferred_token.as_deref(), Some("SOL"));
}

#[test]
fn create_alias_request_allows_missing_preferred_token() {
    let body = serde_json::json!({
        "alias_type": "phone",
        "alias_value": "+15551234567",
        "wallet_address": "9xQeWvG816bUx9EPjHmaT2aRzZ4vWQ9w7rGkmM2qAqQ"
    });
    let req: CreateAliasRequest = serde_json::from_value(body).unwrap();
    assert_eq!(req.alias_type, "phone");
    assert!(req.preferred_token.is_none());
}

#[test]
fn alias_roundtrips_through_json() {
    let alias = sample_alias();
    let json = serde_json::to_string(&alias).unwrap();
    let back: Alias = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, alias.id);
    assert_eq!(back.alias_value, alias.alias_value);
    assert_eq!(back.verified, alias.verified);
}

// ---------------------------------------------------------------------------
// Merchant model
// ---------------------------------------------------------------------------

fn sample_merchant() -> Merchant {
    Merchant {
        id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        merchant_id: "MER-abcdef12".to_string(),
        name: "Acme Coffee".to_string(),
        wallet_address: "7Np4FdvXdXm3G8kP1cT2vwL6o2X8RzH9sLXJs2wQfYbA".to_string(),
        preferred_token: Some("USDC".to_string()),
        split_config: None,
        webhook_url: Some("https://merchant.example.com/webhook".to_string()),
        kyc_status: "pending".to_string(),
        created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
    }
}

#[test]
fn merchant_response_preserves_public_fields() {
    let merchant = sample_merchant();
    let response: MerchantResponse = merchant.clone().into();

    assert_eq!(response.merchant_id, merchant.merchant_id);
    assert_eq!(response.name, merchant.name);
    assert_eq!(response.wallet_address, merchant.wallet_address);
    assert_eq!(response.kyc_status, "pending");
    assert_eq!(response.webhook_url, merchant.webhook_url);
}

#[test]
fn create_merchant_request_optional_id() {
    let body = serde_json::json!({
        "name": "Acme Coffee",
        "wallet_address": "9xQeWvG816bUx9EPjHmaT2aRzZ4vWQ9w7rGkmM2qAqQ"
    });
    let req: CreateMerchantRequest = serde_json::from_value(body).unwrap();
    assert!(req.merchant_id.is_none(), "merchant_id is optional (auto-generated server-side)");
    assert_eq!(req.name, "Acme Coffee");
}

#[test]
fn create_merchant_request_preserves_explicit_id() {
    let body = serde_json::json!({
        "merchant_id": "MER-custom",
        "name": "Acme Coffee",
        "wallet_address": "9xQeWvG816bUx9EPjHmaT2aRzZ4vWQ9w7rGkmM2qAqQ",
        "preferred_token": "USDC",
        "webhook_url": "https://merchant.example.com/webhook"
    });
    let req: CreateMerchantRequest = serde_json::from_value(body).unwrap();
    assert_eq!(req.merchant_id.as_deref(), Some("MER-custom"));
    assert_eq!(req.preferred_token.as_deref(), Some("USDC"));
}

#[test]
fn update_merchant_request_all_fields_optional() {
    let empty = serde_json::json!({});
    let req: UpdateMerchantRequest = serde_json::from_value(empty).unwrap();
    assert!(req.name.is_none());
    assert!(req.wallet_address.is_none());
    assert!(req.preferred_token.is_none());
    assert!(req.split_config.is_none());
    assert!(req.webhook_url.is_none());
}

#[test]
fn update_merchant_request_partial_update() {
    let body = serde_json::json!({ "webhook_url": "https://new.example.com/hook" });
    let req: UpdateMerchantRequest = serde_json::from_value(body).unwrap();
    assert_eq!(req.webhook_url.as_deref(), Some("https://new.example.com/hook"));
    assert!(req.name.is_none());
}

#[test]
fn merchant_roundtrips_through_json() {
    let merchant = sample_merchant();
    let json = serde_json::to_string(&merchant).unwrap();
    let back: Merchant = serde_json::from_str(&json).unwrap();
    assert_eq!(back.merchant_id, merchant.merchant_id);
    assert_eq!(back.kyc_status, merchant.kyc_status);
}

// ---------------------------------------------------------------------------
// Alias-type validation contract
// ---------------------------------------------------------------------------
//
// The service layer validates alias_type against a fixed set
// (email/phone/username) before hitting the DB. This test documents that
// contract so any future expansion is intentional.

#[test]
fn supported_alias_types_are_documented() {
    let supported = ["email", "phone", "username"];
    for t in supported {
        // Just verify the API request struct accepts it.
        let body = serde_json::json!({
            "alias_type": t,
            "alias_value": format!("test-{t}"),
            "wallet_address": "9xQeWvG816bUx9EPjHmaT2aRzZ4vWQ9w7rGkmM2qAqQ"
        });
        let req: CreateAliasRequest = serde_json::from_value(body).unwrap();
        assert_eq!(req.alias_type, t);
    }
}
