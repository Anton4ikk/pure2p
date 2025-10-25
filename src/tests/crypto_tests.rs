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

// AEAD Encryption Tests (XChaCha20-Poly1305)

#[test]
fn test_encrypt_decrypt_roundtrip() {
    // Generate a shared secret
    let alice = KeyPair::generate().expect("Failed to generate Alice's keypair");
    let bob = KeyPair::generate().expect("Failed to generate Bob's keypair");

    let bob_x25519_pub: [u8; 32] = bob.x25519_public.as_slice().try_into().unwrap();
    let shared_secret = alice
        .derive_shared_secret(&bob_x25519_pub)
        .expect("Failed to derive shared secret");

    // Test with various plaintexts
    let test_cases = vec![
        b"Hello, World!".as_slice(),
        b"".as_slice(), // Empty message
        b"A".as_slice(), // Single byte
        &[0u8; 1000], // Large message
        b"Unicode: \xf0\x9f\x94\x92\xf0\x9f\x94\x91".as_slice(), // Emojis
    ];

    for plaintext in test_cases {
        let envelope = encrypt_message(&shared_secret, plaintext)
            .expect("Failed to encrypt message");

        let decrypted = decrypt_message(&shared_secret, &envelope)
            .expect("Failed to decrypt message");

        assert_eq!(plaintext, decrypted.as_slice());
    }
}

#[test]
fn test_encrypted_envelope_structure() {
    let secret = [0u8; 32];
    let plaintext = b"Test message";

    let envelope = encrypt_message(&secret, plaintext)
        .expect("Failed to encrypt message");

    // Nonce should be 24 bytes
    assert_eq!(envelope.nonce.len(), 24);

    // Ciphertext should be plaintext length + 16 bytes (Poly1305 tag)
    assert_eq!(envelope.ciphertext.len(), plaintext.len() + 16);
}

#[test]
fn test_different_nonces_produce_different_ciphertext() {
    let secret = [0u8; 32];
    let plaintext = b"Same message";

    let envelope1 = encrypt_message(&secret, plaintext)
        .expect("Failed to encrypt message 1");
    let envelope2 = encrypt_message(&secret, plaintext)
        .expect("Failed to encrypt message 2");

    // Nonces should be different (random)
    assert_ne!(envelope1.nonce, envelope2.nonce);

    // Ciphertexts should be different due to different nonces
    assert_ne!(envelope1.ciphertext, envelope2.ciphertext);

    // But both should decrypt to the same plaintext
    let decrypted1 = decrypt_message(&secret, &envelope1)
        .expect("Failed to decrypt message 1");
    let decrypted2 = decrypt_message(&secret, &envelope2)
        .expect("Failed to decrypt message 2");

    assert_eq!(decrypted1, decrypted2);
    assert_eq!(decrypted1, plaintext);
}

#[test]
fn test_wrong_key_fails_decryption() {
    let secret1 = [1u8; 32];
    let secret2 = [2u8; 32];
    let plaintext = b"Secret message";

    let envelope = encrypt_message(&secret1, plaintext)
        .expect("Failed to encrypt message");

    // Attempting to decrypt with wrong key should fail
    let result = decrypt_message(&secret2, &envelope);
    assert!(result.is_err());

    // Error message should indicate auth failure
    let err = result.unwrap_err();
    assert!(err.to_string().contains("auth tag mismatch") ||
            err.to_string().contains("Decryption failed"));
}

#[test]
fn test_tampered_ciphertext_fails_decryption() {
    let secret = [0u8; 32];
    let plaintext = b"Authentic message";

    let mut envelope = encrypt_message(&secret, plaintext)
        .expect("Failed to encrypt message");

    // Tamper with the ciphertext
    if let Some(byte) = envelope.ciphertext.first_mut() {
        *byte ^= 0xFF;
    }

    // Decryption should fail due to authentication tag mismatch
    let result = decrypt_message(&secret, &envelope);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("auth tag mismatch") ||
            err.to_string().contains("Decryption failed"));
}

#[test]
fn test_tampered_nonce_fails_decryption() {
    let secret = [0u8; 32];
    let plaintext = b"Message with nonce";

    let mut envelope = encrypt_message(&secret, plaintext)
        .expect("Failed to encrypt message");

    // Tamper with the nonce
    envelope.nonce[0] ^= 0xFF;

    // Decryption should fail (wrong nonce produces wrong plaintext and tag mismatch)
    let result = decrypt_message(&secret, &envelope);
    assert!(result.is_err());
}

#[test]
fn test_envelope_serialization() {
    let secret = [0u8; 32];
    let plaintext = b"Serializable message";

    let envelope = encrypt_message(&secret, plaintext)
        .expect("Failed to encrypt message");

    // Test JSON serialization
    let json = serde_json::to_string(&envelope)
        .expect("Failed to serialize to JSON");
    let deserialized: EncryptedEnvelope = serde_json::from_str(&json)
        .expect("Failed to deserialize from JSON");

    assert_eq!(envelope, deserialized);

    // Verify decryption still works after serialization roundtrip
    let decrypted = decrypt_message(&secret, &deserialized)
        .expect("Failed to decrypt after serialization");
    assert_eq!(plaintext, decrypted.as_slice());
}

#[test]
fn test_envelope_cbor_serialization() {
    let secret = [0u8; 32];
    let plaintext = b"CBOR message";

    let envelope = encrypt_message(&secret, plaintext)
        .expect("Failed to encrypt message");

    // Test CBOR serialization
    let cbor = serde_cbor::to_vec(&envelope)
        .expect("Failed to serialize to CBOR");
    let deserialized: EncryptedEnvelope = serde_cbor::from_slice(&cbor)
        .expect("Failed to deserialize from CBOR");

    assert_eq!(envelope, deserialized);

    // Verify decryption still works
    let decrypted = decrypt_message(&secret, &deserialized)
        .expect("Failed to decrypt after CBOR serialization");
    assert_eq!(plaintext, decrypted.as_slice());
}

#[test]
fn test_encrypt_large_message() {
    let secret = [0u8; 32];
    let plaintext = vec![42u8; 1_000_000]; // 1 MB message

    let envelope = encrypt_message(&secret, &plaintext)
        .expect("Failed to encrypt large message");

    let decrypted = decrypt_message(&secret, &envelope)
        .expect("Failed to decrypt large message");

    assert_eq!(plaintext, decrypted);
}

#[test]
fn test_encrypt_empty_message() {
    let secret = [0u8; 32];
    let plaintext = b"";

    let envelope = encrypt_message(&secret, plaintext)
        .expect("Failed to encrypt empty message");

    // Even empty messages get a 16-byte auth tag
    assert_eq!(envelope.ciphertext.len(), 16);

    let decrypted = decrypt_message(&secret, &envelope)
        .expect("Failed to decrypt empty message");

    assert_eq!(plaintext, decrypted.as_slice());
}

#[test]
fn test_truncated_ciphertext_fails() {
    let secret = [0u8; 32];
    let plaintext = b"Message to truncate";

    let mut envelope = encrypt_message(&secret, plaintext)
        .expect("Failed to encrypt message");

    // Truncate ciphertext (remove last byte of auth tag)
    envelope.ciphertext.pop();

    // Decryption should fail
    let result = decrypt_message(&secret, &envelope);
    assert!(result.is_err());
}

#[test]
fn test_end_to_end_encrypted_communication() {
    // Simulate Alice and Bob exchanging encrypted messages
    let alice = KeyPair::generate().expect("Failed to generate Alice's keypair");
    let bob = KeyPair::generate().expect("Failed to generate Bob's keypair");

    // Both derive the same shared secret
    let alice_x25519_pub: [u8; 32] = alice.x25519_public.as_slice().try_into().unwrap();
    let bob_x25519_pub: [u8; 32] = bob.x25519_public.as_slice().try_into().unwrap();

    let alice_shared_secret = alice
        .derive_shared_secret(&bob_x25519_pub)
        .expect("Alice failed to derive shared secret");
    let bob_shared_secret = bob
        .derive_shared_secret(&alice_x25519_pub)
        .expect("Bob failed to derive shared secret");

    // Verify they derived the same secret
    assert_eq!(alice_shared_secret, bob_shared_secret);

    // Alice sends encrypted message to Bob
    let alice_message = b"Hello Bob, this is Alice!";
    let encrypted_to_bob = encrypt_message(&alice_shared_secret, alice_message)
        .expect("Alice failed to encrypt");

    // Bob decrypts Alice's message
    let decrypted_by_bob = decrypt_message(&bob_shared_secret, &encrypted_to_bob)
        .expect("Bob failed to decrypt");
    assert_eq!(alice_message, decrypted_by_bob.as_slice());

    // Bob sends encrypted reply to Alice
    let bob_message = b"Hi Alice, message received!";
    let encrypted_to_alice = encrypt_message(&bob_shared_secret, bob_message)
        .expect("Bob failed to encrypt");

    // Alice decrypts Bob's message
    let decrypted_by_alice = decrypt_message(&alice_shared_secret, &encrypted_to_alice)
        .expect("Alice failed to decrypt");
    assert_eq!(bob_message, decrypted_by_alice.as_slice());
}

// Contact Token Signing Tests (Ed25519)

#[test]
fn test_sign_and_verify_contact_token() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let token_data = b"contact token test data";

    let privkey: [u8; 32] = keypair.private_key.as_slice().try_into().unwrap();
    let pubkey: [u8; 32] = keypair.public_key.as_slice().try_into().unwrap();

    // Sign the token data
    let signature = sign_contact_token(&privkey, token_data)
        .expect("Failed to sign token");

    // Signature should be 64 bytes
    assert_eq!(signature.len(), 64);

    // Verify the signature
    let is_valid = verify_contact_token(&pubkey, token_data, &signature)
        .expect("Failed to verify token");
    assert!(is_valid);
}

#[test]
fn test_verify_invalid_contact_token_signature() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let token_data = b"original token data";
    let tampered_data = b"tampered token data";

    let privkey: [u8; 32] = keypair.private_key.as_slice().try_into().unwrap();
    let pubkey: [u8; 32] = keypair.public_key.as_slice().try_into().unwrap();

    // Sign the original data
    let signature = sign_contact_token(&privkey, token_data)
        .expect("Failed to sign token");

    // Verify with tampered data should fail
    let is_valid = verify_contact_token(&pubkey, tampered_data, &signature)
        .expect("Failed to verify token");
    assert!(!is_valid);
}

#[test]
fn test_verify_corrupted_contact_token_signature() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let token_data = b"token data";

    let privkey: [u8; 32] = keypair.private_key.as_slice().try_into().unwrap();
    let pubkey: [u8; 32] = keypair.public_key.as_slice().try_into().unwrap();

    // Sign the token
    let mut signature = sign_contact_token(&privkey, token_data)
        .expect("Failed to sign token");

    // Corrupt the signature
    signature[0] ^= 0xFF;
    signature[32] ^= 0xAA;

    // Verification should fail
    let is_valid = verify_contact_token(&pubkey, token_data, &signature)
        .expect("Failed to verify token");
    assert!(!is_valid);
}

#[test]
fn test_verify_with_wrong_public_key() {
    let keypair1 = KeyPair::generate().expect("Failed to generate keypair 1");
    let keypair2 = KeyPair::generate().expect("Failed to generate keypair 2");
    let token_data = b"token data";

    let privkey1: [u8; 32] = keypair1.private_key.as_slice().try_into().unwrap();
    let pubkey2: [u8; 32] = keypair2.public_key.as_slice().try_into().unwrap();

    // Sign with keypair1's private key
    let signature = sign_contact_token(&privkey1, token_data)
        .expect("Failed to sign token");

    // Verify with keypair2's public key should fail
    let is_valid = verify_contact_token(&pubkey2, token_data, &signature)
        .expect("Failed to verify token");
    assert!(!is_valid);
}

#[test]
fn test_contact_token_signature_deterministic() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let token_data = b"deterministic test";

    let privkey: [u8; 32] = keypair.private_key.as_slice().try_into().unwrap();

    // Sign the same data twice
    let signature1 = sign_contact_token(&privkey, token_data)
        .expect("Failed to sign token 1");
    let signature2 = sign_contact_token(&privkey, token_data)
        .expect("Failed to sign token 2");

    // Ed25519 signatures are deterministic (same input -> same output)
    assert_eq!(signature1, signature2);
}

#[test]
fn test_contact_token_signature_different_data() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let data1 = b"token data 1";
    let data2 = b"token data 2";

    let privkey: [u8; 32] = keypair.private_key.as_slice().try_into().unwrap();

    // Sign different data
    let signature1 = sign_contact_token(&privkey, data1)
        .expect("Failed to sign token 1");
    let signature2 = sign_contact_token(&privkey, data2)
        .expect("Failed to sign token 2");

    // Signatures should be different
    assert_ne!(signature1, signature2);
}

#[test]
fn test_contact_token_empty_data() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let empty_data = b"";

    let privkey: [u8; 32] = keypair.private_key.as_slice().try_into().unwrap();
    let pubkey: [u8; 32] = keypair.public_key.as_slice().try_into().unwrap();

    // Sign empty data (should work)
    let signature = sign_contact_token(&privkey, empty_data)
        .expect("Failed to sign empty token");

    // Verify should succeed
    let is_valid = verify_contact_token(&pubkey, empty_data, &signature)
        .expect("Failed to verify empty token");
    assert!(is_valid);
}

#[test]
fn test_contact_token_large_data() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let large_data = vec![42u8; 10_000]; // 10 KB

    let privkey: [u8; 32] = keypair.private_key.as_slice().try_into().unwrap();
    let pubkey: [u8; 32] = keypair.public_key.as_slice().try_into().unwrap();

    // Sign large data
    let signature = sign_contact_token(&privkey, &large_data)
        .expect("Failed to sign large token");

    // Verify should succeed
    let is_valid = verify_contact_token(&pubkey, &large_data, &signature)
        .expect("Failed to verify large token");
    assert!(is_valid);
}
