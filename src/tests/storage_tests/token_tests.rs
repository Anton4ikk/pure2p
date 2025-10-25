// Token Tests - Testing contact token generation and parsing (with signature verification)

use crate::crypto::KeyPair;
use crate::storage::{generate_contact_token, parse_contact_token};
use crate::Error;
use chrono::{Duration, Utc};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

#[test]
fn test_generate_contact_token() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let ip = "192.168.1.100:8080";
    let expiry = Utc::now() + Duration::days(30);

    let token = generate_contact_token(
        ip,
        &keypair.public_key,
        &keypair.private_key,
        &keypair.x25519_public,
        expiry
    ).expect("Failed to generate token");

    // Token should not be empty
    assert!(!token.is_empty());

    // Token should be valid base64 URL-safe
    assert!(token.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
}

#[test]
fn test_parse_contact_token_roundtrip() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let ip = "10.0.0.1:9000";
    let expiry = Utc::now() + Duration::days(7);

    // Generate signed token
    let token = generate_contact_token(
        ip,
        &keypair.public_key,
        &keypair.private_key,
        &keypair.x25519_public,
        expiry
    ).expect("Failed to generate token");

    // Parse token (signature verification happens here)
    let contact = parse_contact_token(&token).expect("Failed to parse valid token");

    // Verify fields
    assert_eq!(contact.ip, ip);
    assert_eq!(contact.pubkey, keypair.public_key);
    assert_eq!(contact.x25519_pubkey, keypair.x25519_public);
    assert_eq!(contact.expiry, expiry);
    assert!(contact.is_active); // Should be active by default

    // UID should match the one derived from pubkey
    assert_eq!(contact.uid, keypair.uid.to_string());
}

#[test]
fn test_parse_contact_token_expired() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let ip = "127.0.0.1:8080";
    let expiry = Utc::now() - Duration::days(1); // Expired yesterday

    // Generate token with expired timestamp
    let token = generate_contact_token(
        ip,
        &keypair.public_key,
        &keypair.private_key,
        &keypair.x25519_public,
        expiry
    ).expect("Failed to generate token");

    // Parsing should fail due to expiry (after signature verification passes)
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

    // Generate signed token with real keys
    let token = generate_contact_token(
        ip,
        &keypair.public_key,
        &keypair.private_key,
        &keypair.x25519_public,
        expiry
    ).expect("Failed to generate token");

    // Parse token (with signature verification)
    let contact = parse_contact_token(&token).expect("Failed to parse token with real keys");

    // Verify fields
    assert_eq!(contact.ip, ip);
    assert_eq!(contact.pubkey, keypair.public_key);
    assert_eq!(contact.x25519_pubkey, keypair.x25519_public);
    assert_eq!(contact.uid, keypair.uid.to_string());
    assert!(contact.is_active);
}

#[test]
fn test_contact_token_different_inputs_different_tokens() {
    let keypair1 = KeyPair::generate().expect("Failed to generate keypair 1");
    let keypair2 = KeyPair::generate().expect("Failed to generate keypair 2");
    let expiry = Utc::now() + Duration::days(30);

    let token1 = generate_contact_token(
        "192.168.1.1:8080",
        &keypair1.public_key,
        &keypair1.private_key,
        &keypair1.x25519_public,
        expiry
    ).expect("Failed to generate token 1");

    let token2 = generate_contact_token(
        "192.168.1.2:8080",
        &keypair1.public_key,
        &keypair1.private_key,
        &keypair1.x25519_public,
        expiry
    ).expect("Failed to generate token 2");

    let token3 = generate_contact_token(
        "192.168.1.1:8080",
        &keypair2.public_key,
        &keypair2.private_key,
        &keypair2.x25519_public,
        expiry
    ).expect("Failed to generate token 3");

    // Different IPs should produce different tokens
    assert_ne!(token1, token2);

    // Different keypairs should produce different tokens
    assert_ne!(token1, token3);
}

#[test]
fn test_contact_token_deterministic() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let ip = "10.20.30.40:5000";
    let expiry = Utc::now() + Duration::days(15);

    // Generate token twice with same inputs
    let token1 = generate_contact_token(
        ip,
        &keypair.public_key,
        &keypair.private_key,
        &keypair.x25519_public,
        expiry
    ).expect("Failed to generate token 1");

    let token2 = generate_contact_token(
        ip,
        &keypair.public_key,
        &keypair.private_key,
        &keypair.x25519_public,
        expiry
    ).expect("Failed to generate token 2");

    // Should produce identical tokens (Ed25519 signatures are deterministic)
    assert_eq!(token1, token2);
}

#[test]
fn test_contact_token_tampered_signature() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let ip = "192.168.1.100:8080";
    let expiry = Utc::now() + Duration::days(30);

    // Generate a valid signed token
    let token = generate_contact_token(
        ip,
        &keypair.public_key,
        &keypair.private_key,
        &keypair.x25519_public,
        expiry
    ).expect("Failed to generate token");

    // Decode and tamper with the token
    let mut cbor = URL_SAFE_NO_PAD.decode(&token).expect("Failed to decode token");

    // Flip a bit in the signature (signature is at the end of the CBOR data)
    if let Some(byte) = cbor.last_mut() {
        *byte ^= 0xFF;
    }

    // Re-encode the tampered token
    let tampered_token = URL_SAFE_NO_PAD.encode(cbor);

    // Parsing should fail due to invalid signature
    let result = parse_contact_token(&tampered_token);
    assert!(result.is_err());

    match result {
        Err(Error::Crypto(msg)) => {
            assert!(msg.contains("signature verification failed") || msg.contains("tampered"));
        }
        Err(Error::CborSerialization(_)) => {
            // CBOR deserialization might fail first if structure is corrupted
        }
        _ => panic!("Expected Crypto or CborSerialization error for tampered token"),
    }
}

#[test]
fn test_contact_token_tampered_payload() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let ip = "10.0.0.1:8080";
    let expiry = Utc::now() + Duration::days(30);

    // Generate a valid signed token
    let token = generate_contact_token(
        ip,
        &keypair.public_key,
        &keypair.private_key,
        &keypair.x25519_public,
        expiry
    ).expect("Failed to generate token");

    // Parse as CBOR to tamper with payload
    let cbor = URL_SAFE_NO_PAD.decode(&token).expect("Failed to decode");

    // Try to tamper by flipping a bit in the middle (likely in the payload)
    let mut tampered_cbor = cbor.clone();
    if tampered_cbor.len() > 10 {
        tampered_cbor[10] ^= 0x01; // Flip a bit
    }

    let tampered_token = URL_SAFE_NO_PAD.encode(tampered_cbor);

    // Parsing should fail (either CBOR deserialization or signature verification)
    let result = parse_contact_token(&tampered_token);
    assert!(result.is_err());
}

#[test]
fn test_contact_token_wrong_signer() {
    let keypair1 = KeyPair::generate().expect("Failed to generate keypair 1");
    let keypair2 = KeyPair::generate().expect("Failed to generate keypair 2");
    let ip = "192.168.1.100:8080";
    let expiry = Utc::now() + Duration::days(30);

    // Create token with keypair1's public key but sign with keypair2's private key
    let result = generate_contact_token(
        ip,
        &keypair1.public_key,  // Public key from keypair1
        &keypair2.private_key, // Private key from keypair2 (wrong!)
        &keypair1.x25519_public,
        expiry
    );

    // Token generation succeeds (signing works)
    let token = result.expect("Token generation should succeed");

    // But parsing should fail because the signature won't match the public key
    let parse_result = parse_contact_token(&token);
    assert!(parse_result.is_err());

    if let Err(Error::Crypto(msg)) = parse_result {
        assert!(msg.contains("signature verification failed"));
    } else {
        panic!("Expected Crypto error for mismatched signer");
    }
}
