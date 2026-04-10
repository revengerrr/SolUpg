#[test]
fn test_api_key_generation_format() {
    let key = api_gateway::auth::generate_api_key();
    assert!(
        key.starts_with("solupg_live_"),
        "key should start with solupg_live_"
    );
    assert!(key.len() > 20, "key should be sufficiently long");
}

#[test]
fn test_api_key_generation_uniqueness() {
    let key1 = api_gateway::auth::generate_api_key();
    let key2 = api_gateway::auth::generate_api_key();
    assert_ne!(key1, key2, "generated keys should be unique");
}

#[test]
fn test_api_key_hash_deterministic() {
    let key = "solupg_live_test123";
    let hash1 = api_gateway::auth::hash_api_key(key);
    let hash2 = api_gateway::auth::hash_api_key(key);
    assert_eq!(hash1, hash2, "same key should produce same hash");
}

#[test]
fn test_api_key_hash_different_keys() {
    let hash1 = api_gateway::auth::hash_api_key("key_a");
    let hash2 = api_gateway::auth::hash_api_key("key_b");
    assert_ne!(
        hash1, hash2,
        "different keys should produce different hashes"
    );
}

#[test]
fn test_jwt_create_and_verify() {
    let secret = "test_secret_key";
    let merchant_uuid = uuid::Uuid::new_v4();
    let merchant_id = "test_merchant";

    let token = api_gateway::auth::create_token(secret, merchant_uuid, merchant_id).unwrap();
    assert!(!token.is_empty());

    // Token should have 3 parts (header.payload.signature)
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts");

    let claims = api_gateway::auth::verify_token(secret, &token).unwrap();
    assert_eq!(claims.sub, merchant_uuid.to_string());
    assert_eq!(claims.merchant_id, merchant_id);
}

#[test]
fn test_jwt_verify_wrong_secret() {
    let merchant_uuid = uuid::Uuid::new_v4();
    let token = api_gateway::auth::create_token("secret_a", merchant_uuid, "test").unwrap();
    let result = api_gateway::auth::verify_token("secret_b", &token);
    assert!(result.is_err(), "wrong secret should fail verification");
}

#[test]
fn test_jwt_verify_tampered_token() {
    let secret = "test_secret";
    let merchant_uuid = uuid::Uuid::new_v4();
    let token = api_gateway::auth::create_token(secret, merchant_uuid, "test").unwrap();

    // Tamper with the payload
    let mut parts: Vec<&str> = token.split('.').collect();
    parts[1] = "dGFtcGVyZWQ"; // base64("tampered")
    let tampered = parts.join(".");

    let result = api_gateway::auth::verify_token(secret, &tampered);
    assert!(result.is_err(), "tampered token should fail");
}

#[test]
fn test_jwt_invalid_format() {
    let result = api_gateway::auth::verify_token("secret", "not.a.valid.jwt.too.many.parts");
    assert!(result.is_err());

    let result = api_gateway::auth::verify_token("secret", "nodotsatall");
    assert!(result.is_err());
}
