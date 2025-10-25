// Token Tests - Testing contact token generation and parsing

use crate::crypto::{KeyPair, UID};
use crate::storage::{generate_contact_token, parse_contact_token};
use crate::Error;
use chrono::{Duration, Utc};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

#[test]
fn test_generate_contact_token() {
    let ip = "192.168.1.100:8080";
    let pubkey = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let expiry = Utc::now() + Duration::days(30);

    let token = generate_contact_token(ip, &pubkey, &vec![99u8; 32], expiry);

    // Token should not be empty
    assert!(!token.is_empty());

    // Token should be valid base64 URL-safe
    assert!(token.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
}

#[test]
fn test_parse_contact_token_roundtrip() {
    let ip = "10.0.0.1:9000";
    let pubkey = vec![100, 101, 102, 103, 104, 105, 106, 107, 108, 109];
    let expiry = Utc::now() + Duration::days(7);

    // Generate token
    let token = generate_contact_token(ip, &pubkey, &vec![99u8; 32], expiry);

    // Parse token
    let contact = parse_contact_token(&token).expect("Failed to parse valid token");

    // Verify fields
    assert_eq!(contact.ip, ip);
    assert_eq!(contact.pubkey, pubkey);
    assert_eq!(contact.expiry, expiry);
    assert!(contact.is_active); // Should be active by default

    // UID should match the one derived from pubkey
    let expected_uid = UID::from_public_key(&pubkey);
    assert_eq!(contact.uid, expected_uid.to_string());
}

#[test]
fn test_parse_contact_token_expired() {
    let ip = "127.0.0.1:8080";
    let pubkey = vec![1, 2, 3, 4, 5];
    let expiry = Utc::now() - Duration::days(1); // Expired yesterday

    // Generate token with expired timestamp
    let token = generate_contact_token(ip, &pubkey, &vec![99u8; 32], expiry);

    // Parsing should fail due to expiry
    let result = parse_contact_token(&token);
    assert!(result.is_err());

    if let Err(Error::Storage(msg)) = result {
        assert!(msg.contains("expired"));
    } else {
        panic!("Expected Storage error with 'expired' message");
    }
}

#[test]
fn test_parse_contact_token_invalid_base64() {
    let invalid_token = "not-valid-base64!!!";

    let result = parse_contact_token(invalid_token);
    assert!(result.is_err());

    if let Err(Error::Storage(msg)) = result {
        assert!(msg.contains("Invalid base64"));
    } else {
        panic!("Expected Storage error for invalid base64");
    }
}

#[test]
fn test_parse_contact_token_invalid_cbor() {
    // Create a valid base64 string but with invalid CBOR data
    let invalid_cbor = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid CBOR
    let token = URL_SAFE_NO_PAD.encode(invalid_cbor);

    let result = parse_contact_token(&token);
    assert!(result.is_err());

    if let Err(Error::CborSerialization(msg)) = result {
        assert!(msg.contains("Invalid token data"));
    } else {
        panic!("Expected CborSerialization error for invalid CBOR");
    }
}

#[test]
fn test_contact_token_with_real_crypto_keys() {
    // Generate a real keypair
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let ip = "203.0.113.10:8080";
    let expiry = Utc::now() + Duration::days(90);

    // Generate token with real public key
    let token = generate_contact_token(ip, &keypair.public_key, &keypair.x25519_public, expiry);

    // Parse token
    let contact = parse_contact_token(&token).expect("Failed to parse token with real keys");

    // Verify fields
    assert_eq!(contact.ip, ip);
    assert_eq!(contact.pubkey, keypair.public_key);
    assert_eq!(contact.uid, keypair.uid.to_string());
    assert!(contact.is_active);
}

#[test]
fn test_contact_token_different_inputs_different_tokens() {
    let expiry = Utc::now() + Duration::days(30);

    let token1 = generate_contact_token("192.168.1.1:8080", &[1, 2, 3], &vec![99u8; 32], expiry);
    let token2 = generate_contact_token("192.168.1.2:8080", &[1, 2, 3], &vec![99u8; 32], expiry);
    let token3 = generate_contact_token("192.168.1.1:8080", &[4, 5, 6], &vec![99u8; 32], expiry);

    // Different IPs should produce different tokens
    assert_ne!(token1, token2);

    // Different pubkeys should produce different tokens
    assert_ne!(token1, token3);
}

#[test]
fn test_contact_token_deterministic() {
    let ip = "10.20.30.40:5000";
    let pubkey = vec![10, 20, 30, 40, 50];
    let expiry = Utc::now() + Duration::days(15);

    // Generate token twice with same inputs
    let token1 = generate_contact_token(ip, &pubkey, &vec![99u8; 32], expiry);
    let token2 = generate_contact_token(ip, &pubkey, &vec![99u8; 32], expiry);

    // Should produce identical tokens
    assert_eq!(token1, token2);
}
