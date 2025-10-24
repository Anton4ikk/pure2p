use crate::crypto::*;

#[test]
fn test_keypair_generation() {
    // Test that we can generate a keypair
    let keypair = KeyPair::generate().expect("Failed to generate keypair");

    // Ed25519 public keys are 32 bytes
    assert_eq!(keypair.public_key.len(), 32);
    // Ed25519 private keys are 32 bytes
    assert_eq!(keypair.private_key.len(), 32);
    // UID should be 32 hex characters (16 bytes)
    assert_eq!(keypair.uid.as_str().len(), 32);
}

#[test]
fn test_uid_consistency() {
    // Generate a keypair
    let keypair = KeyPair::generate().expect("Failed to generate keypair");

    // Derive UID from public key multiple times
    let uid1 = UID::from_public_key(&keypair.public_key);
    let uid2 = UID::from_public_key(&keypair.public_key);

    // UIDs should be identical
    assert_eq!(uid1, uid2);
    assert_eq!(keypair.uid, uid1);
}

#[test]
fn test_uid_uniqueness() {
    // Generate two different keypairs
    let keypair1 = KeyPair::generate().expect("Failed to generate keypair 1");
    let keypair2 = KeyPair::generate().expect("Failed to generate keypair 2");

    // Their UIDs should be different
    assert_ne!(keypair1.uid, keypair2.uid);
    // Their public keys should be different
    assert_ne!(keypair1.public_key, keypair2.public_key);
}

#[test]
fn test_sign_and_verify() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let message = b"Hello, Pure2P!";

    // Sign the message
    let signature = keypair.sign(message).expect("Failed to sign message");

    // Signature should be 64 bytes (Ed25519 signature size)
    assert_eq!(signature.len(), 64);

    // Verify the signature
    let is_valid = keypair
        .verify(message, &signature)
        .expect("Failed to verify signature");
    assert!(is_valid);
}

#[test]
fn test_verify_invalid_signature() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let message = b"Hello, Pure2P!";
    let wrong_message = b"Wrong message!";

    // Sign the original message
    let signature = keypair.sign(message).expect("Failed to sign message");

    // Verify with wrong message should fail
    let is_valid = keypair
        .verify(wrong_message, &signature)
        .expect("Failed to verify signature");
    assert!(!is_valid);
}

#[test]
fn test_verify_corrupted_signature() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let message = b"Hello, Pure2P!";

    // Sign the message
    let mut signature = keypair.sign(message).expect("Failed to sign message");

    // Corrupt the signature
    signature[0] ^= 0xFF;

    // Verify should fail
    let is_valid = keypair
        .verify(message, &signature)
        .expect("Failed to verify signature");
    assert!(!is_valid);
}

#[test]
fn test_uid_display() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let uid_string = format!("{}", keypair.uid);

    // Should format as a hex string
    assert_eq!(uid_string, keypair.uid.as_str());
    // Should be all lowercase hex
    assert!(uid_string.chars().all(|c| c.is_ascii_hexdigit()));
}
