//! DB-backed tests for directory-service.
//!
//! These tests require a running Postgres instance and are marked `#[ignore]`
//! so they don't run by default. Run with:
//!
//!   cd services
//!   DATABASE_URL=postgres://solupg:solupg_dev@localhost:5433/solupg \
//!     cargo test -p directory-service --test db_test -- --ignored
//!
//! In CI, the rust.yml `test` job provides a Postgres service and can run
//! these via a scheduled workflow or manual dispatch.

use directory_service::models::{CreateAliasRequest, CreateMerchantRequest, UpdateMerchantRequest};
use directory_service::services;
use sqlx::PgPool;

async fn setup_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    // Run migrations (no-op if already applied).
    sqlx::migrate!("../migrations").run(&pool).await.ok()?;
    Some(pool)
}

fn unique_value(prefix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}-{ts}")
}

const SAMPLE_WALLET: &str = "9xQeWvG816bUx9EPjHmaT2aRzZ4vWQ9w7rGkmM2qAqQ";

// ---------------------------------------------------------------------------
// Alias CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn create_alias_happy_path() {
    let Some(pool) = setup_pool().await else {
        eprintln!("DATABASE_URL not set — skipping");
        return;
    };

    let value = unique_value("test@example.com");
    let req = CreateAliasRequest {
        alias_type: "email".to_string(),
        alias_value: value.clone(),
        wallet_address: SAMPLE_WALLET.to_string(),
        preferred_token: Some("USDC".to_string()),
    };

    let created = services::create_alias(&pool, req).await.unwrap();
    assert_eq!(created.alias_type, "email");
    assert_eq!(created.alias_value, value);
    assert!(!created.verified, "new aliases start unverified");
}

#[tokio::test]
#[ignore]
async fn create_alias_rejects_invalid_type() {
    let Some(pool) = setup_pool().await else {
        eprintln!("DATABASE_URL not set — skipping");
        return;
    };

    let req = CreateAliasRequest {
        alias_type: "twitter".to_string(),
        alias_value: unique_value("bad"),
        wallet_address: SAMPLE_WALLET.to_string(),
        preferred_token: None,
    };

    let err = services::create_alias(&pool, req).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("alias_type"),
        "error should mention alias_type: {msg}"
    );
}

#[tokio::test]
#[ignore]
async fn create_alias_conflict_on_duplicate() {
    let Some(pool) = setup_pool().await else {
        eprintln!("DATABASE_URL not set — skipping");
        return;
    };

    let value = unique_value("dup@example.com");

    let first = CreateAliasRequest {
        alias_type: "email".to_string(),
        alias_value: value.clone(),
        wallet_address: SAMPLE_WALLET.to_string(),
        preferred_token: None,
    };
    services::create_alias(&pool, first).await.unwrap();

    let second = CreateAliasRequest {
        alias_type: "email".to_string(),
        alias_value: value,
        wallet_address: SAMPLE_WALLET.to_string(),
        preferred_token: None,
    };
    let err = services::create_alias(&pool, second).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.to_lowercase().contains("already") || msg.to_lowercase().contains("conflict"),
        "expected duplicate error, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Merchant CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn create_merchant_auto_generates_id() {
    let Some(pool) = setup_pool().await else {
        eprintln!("DATABASE_URL not set — skipping");
        return;
    };

    let req = CreateMerchantRequest {
        merchant_id: None,
        name: "Test Merchant".to_string(),
        wallet_address: SAMPLE_WALLET.to_string(),
        preferred_token: Some("USDC".to_string()),
        split_config: None,
        webhook_url: None,
    };

    let merchant = services::create_merchant(&pool, req).await.unwrap();
    assert!(merchant.merchant_id.starts_with("MER-"));
    assert_eq!(merchant.kyc_status, "pending");
}

#[tokio::test]
#[ignore]
async fn update_merchant_round_trip() {
    let Some(pool) = setup_pool().await else {
        eprintln!("DATABASE_URL not set — skipping");
        return;
    };

    let explicit_id = unique_value("MER-upd");
    services::create_merchant(
        &pool,
        CreateMerchantRequest {
            merchant_id: Some(explicit_id.clone()),
            name: "Initial Name".to_string(),
            wallet_address: SAMPLE_WALLET.to_string(),
            preferred_token: None,
            split_config: None,
            webhook_url: None,
        },
    )
    .await
    .unwrap();

    let updated = services::update_merchant(
        &pool,
        &explicit_id,
        UpdateMerchantRequest {
            name: Some("Updated Name".to_string()),
            wallet_address: None,
            preferred_token: Some("SOL".to_string()),
            split_config: None,
            webhook_url: Some("https://new.example.com/hook".to_string()),
        },
    )
    .await
    .unwrap();

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.preferred_token.as_deref(), Some("SOL"));
    assert_eq!(
        updated.webhook_url.as_deref(),
        Some("https://new.example.com/hook")
    );
    assert_eq!(updated.wallet_address, SAMPLE_WALLET);
}

#[tokio::test]
#[ignore]
async fn get_merchant_not_found_returns_error() {
    let Some(pool) = setup_pool().await else {
        eprintln!("DATABASE_URL not set — skipping");
        return;
    };
    let err = services::get_merchant(&pool, "MER-does-not-exist")
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.to_lowercase().contains("not found"),
        "expected not-found error, got: {msg}"
    );
}
