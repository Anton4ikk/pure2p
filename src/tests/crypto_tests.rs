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

// X25519 Key Exchange Tests

#[test]
fn test_x25519_keypair_generation() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");

    // X25519 keys are 32 bytes
    assert_eq!(keypair.x25519_public.len(), 32);
    assert_eq!(keypair.x25519_secret.len(), 32);
}

#[test]
fn test_x25519_keys_are_different_from_ed25519() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");

    // X25519 and Ed25519 keys should be different (independently generated)
    assert_ne!(keypair.x25519_public, keypair.public_key);
    assert_ne!(keypair.x25519_secret, keypair.private_key);
}

#[test]
fn test_derive_shared_secret_symmetric() {
    // Generate two keypairs
    let alice = KeyPair::generate().expect("Failed to generate Alice's keypair");
    let bob = KeyPair::generate().expect("Failed to generate Bob's keypair");

    // Derive shared secrets
    let alice_x25519_pub: [u8; 32] = alice.x25519_public.as_slice().try_into().unwrap();
    let bob_x25519_pub: [u8; 32] = bob.x25519_public.as_slice().try_into().unwrap();

    let secret_ab = alice
        .derive_shared_secret(&bob_x25519_pub)
        .expect("Alice failed to derive shared secret");
    let secret_ba = bob
        .derive_shared_secret(&alice_x25519_pub)
        .expect("Bob failed to derive shared secret");

    // Shared secrets should be identical (symmetric)
    assert_eq!(secret_ab, secret_ba);
}

#[test]
fn test_derive_shared_secret_different_for_different_pairs() {
    let alice = KeyPair::generate().expect("Failed to generate Alice's keypair");
    let bob = KeyPair::generate().expect("Failed to generate Bob's keypair");
    let charlie = KeyPair::generate().expect("Failed to generate Charlie's keypair");

    let bob_x25519_pub: [u8; 32] = bob.x25519_public.as_slice().try_into().unwrap();
    let charlie_x25519_pub: [u8; 32] = charlie.x25519_public.as_slice().try_into().unwrap();

    let secret_ab = alice
        .derive_shared_secret(&bob_x25519_pub)
        .expect("Failed to derive Alice-Bob secret");
    let secret_ac = alice
        .derive_shared_secret(&charlie_x25519_pub)
        .expect("Failed to derive Alice-Charlie secret");

    // Different pairs should have different secrets
    assert_ne!(secret_ab, secret_ac);
}

#[test]
fn test_standalone_derive_shared_secret() {
    // Generate two keypairs
    let alice = KeyPair::generate().expect("Failed to generate Alice's keypair");
    let bob = KeyPair::generate().expect("Failed to generate Bob's keypair");

    // Use standalone function
    let alice_secret: [u8; 32] = alice.x25519_secret.as_slice().try_into().unwrap();
    let bob_secret: [u8; 32] = bob.x25519_secret.as_slice().try_into().unwrap();
    let alice_public: [u8; 32] = alice.x25519_public.as_slice().try_into().unwrap();
    let bob_public: [u8; 32] = bob.x25519_public.as_slice().try_into().unwrap();

    let secret_ab = crate::crypto::derive_shared_secret(&alice_secret, &bob_public);
    let secret_ba = crate::crypto::derive_shared_secret(&bob_secret, &alice_public);

    // Should be symmetric
    assert_eq!(secret_ab, secret_ba);
    assert_eq!(secret_ab.len(), 32);
}

#[test]
fn test_derive_shared_secret_deterministic() {
    let alice = KeyPair::generate().expect("Failed to generate Alice's keypair");
    let bob = KeyPair::generate().expect("Failed to generate Bob's keypair");

    let bob_x25519_pub: [u8; 32] = bob.x25519_public.as_slice().try_into().unwrap();

    // Derive secret multiple times
    let secret1 = alice
        .derive_shared_secret(&bob_x25519_pub)
        .expect("Failed to derive secret 1");
    let secret2 = alice
        .derive_shared_secret(&bob_x25519_pub)
        .expect("Failed to derive secret 2");

    // Should always produce the same result
    assert_eq!(secret1, secret2);
}
