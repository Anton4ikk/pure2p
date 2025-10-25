use crate::protocol::*;
use crate::crypto::KeyPair;

#[test]
fn test_message_envelope_creation() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Hello, Pure2P!".to_vec();

    let envelope = MessageEnvelope::new(sender.uid(), recipient.uid(), MessageType::Text, payload.clone());

    assert_eq!(envelope.version, PROTOCOL_VERSION);
    assert_eq!(envelope.from_uid, sender.uid().as_str());
    assert_eq!(envelope.to_uid, recipient.uid().as_str());
    assert_eq!(envelope.payload, payload);
    assert_eq!(envelope.message_type, MessageType::Text);
    assert!(!envelope.encrypted); // Default is plaintext
    assert!(envelope.timestamp > 0);
    assert!(!envelope.id.is_nil());
}

#[test]
fn test_message_envelope_with_type() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Delete request".to_vec();

    // Test Text type
    let text_envelope = MessageEnvelope::new_text(sender.uid(), recipient.uid(), payload.clone());
    assert_eq!(text_envelope.message_type, MessageType::Text);
    assert!(!text_envelope.id.is_nil());

    // Test Delete type
    let delete_envelope = MessageEnvelope::new_delete(sender.uid(), recipient.uid(), payload.clone());
    assert_eq!(delete_envelope.message_type, MessageType::Delete);
    assert!(!delete_envelope.id.is_nil());

    // IDs should be different
    assert_ne!(text_envelope.id, delete_envelope.id);
}

#[test]
fn test_cbor_encode_decode_roundtrip() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Test message for CBOR encoding".to_vec();

    let original = MessageEnvelope::new(sender.uid(), recipient.uid(), MessageType::Text, payload);

    // Encode to CBOR
    let encoded = original.to_cbor().expect("Failed to encode to CBOR");
    assert!(!encoded.is_empty());

    // Decode from CBOR
    let decoded = MessageEnvelope::from_cbor(&encoded)
        .expect("Failed to decode from CBOR");

    // Verify roundtrip
    assert_eq!(decoded.id, original.id);
    assert_eq!(decoded.version, original.version);
    assert_eq!(decoded.from_uid, original.from_uid);
    assert_eq!(decoded.to_uid, original.to_uid);
    assert_eq!(decoded.timestamp, original.timestamp);
    assert_eq!(decoded.message_type, original.message_type);
    assert_eq!(decoded.payload, original.payload);
    assert_eq!(decoded, original);
}

#[test]
fn test_json_encode_decode_roundtrip() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Test message for JSON encoding".to_vec();

    let original = MessageEnvelope::new(sender.uid(), recipient.uid(), MessageType::Text, payload);

    // Encode to JSON
    let encoded = original.to_json().expect("Failed to encode to JSON");
    assert!(!encoded.is_empty());

    // Decode from JSON
    let decoded = MessageEnvelope::from_json(&encoded)
        .expect("Failed to decode from JSON");

    // Verify roundtrip
    assert_eq!(decoded, original);
}

#[test]
fn test_json_string_encoding() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Human readable message".to_vec();

    let envelope = MessageEnvelope::new(sender.uid(), recipient.uid(), MessageType::Text, payload);

    let json_string = envelope.to_json_string()
        .expect("Failed to encode to JSON string");

    // Should be valid JSON
    assert!(json_string.contains("id"));
    assert!(json_string.contains("version"));
    assert!(json_string.contains("from_uid"));
    assert!(json_string.contains("to_uid"));
    assert!(json_string.contains("timestamp"));
    assert!(json_string.contains("message_type"));
    assert!(json_string.contains("payload"));

    // Should be able to parse back
    let decoded: MessageEnvelope = serde_json::from_str(&json_string)
        .expect("Failed to parse JSON string");
    assert_eq!(decoded, envelope);
}

#[test]
fn test_version_compatibility() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Version test".to_vec();

    let mut envelope = MessageEnvelope::new(sender.uid(), recipient.uid(), MessageType::Text, payload);

    // Current version should be compatible
    assert!(envelope.is_version_compatible());

    // Future version should still be compatible (backward compatible)
    envelope.version = PROTOCOL_VERSION;
    assert!(envelope.is_version_compatible());

    // Past version should be compatible
    envelope.version = PROTOCOL_VERSION - 1;
    assert!(envelope.is_version_compatible());
}

#[test]
fn test_message_age() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Age test".to_vec();

    let envelope = MessageEnvelope::new(sender.uid(), recipient.uid(), MessageType::Text, payload);

    // Age should be very small (just created)
    let age = envelope.age_ms();
    assert!(age >= 0);
    assert!(age < 1000); // Less than 1 second

    // Create envelope with old timestamp
    let mut old_envelope = envelope.clone();
    old_envelope.timestamp -= 5000; // 5 seconds ago

    let old_age = old_envelope.age_ms();
    assert!(old_age >= 5000);
}

#[test]
fn test_cbor_vs_json_size() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = vec![0u8; 100]; // 100 bytes of data

    let envelope = MessageEnvelope::new(sender.uid(), recipient.uid(), MessageType::Text, payload);

    let cbor_encoded = envelope.to_cbor().expect("Failed to encode to CBOR");
    let json_encoded = envelope.to_json().expect("Failed to encode to JSON");

    // CBOR should typically be more compact than JSON
    println!("CBOR size: {} bytes", cbor_encoded.len());
    println!("JSON size: {} bytes", json_encoded.len());

    // Both should encode the same data
    let cbor_decoded = MessageEnvelope::from_cbor(&cbor_encoded)
        .expect("Failed to decode CBOR");
    let json_decoded = MessageEnvelope::from_json(&json_encoded)
        .expect("Failed to decode JSON");

    assert_eq!(cbor_decoded, json_decoded);
    assert_eq!(cbor_decoded, envelope);
}

#[test]
fn test_empty_payload() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = Vec::new();

    let envelope = MessageEnvelope::new(sender.uid(), recipient.uid(), MessageType::Text, payload);

    // Should handle empty payload
    assert!(envelope.payload.is_empty());

    // Should encode/decode empty payload
    let cbor_encoded = envelope.to_cbor().expect("Failed to encode");
    let decoded = MessageEnvelope::from_cbor(&cbor_encoded)
        .expect("Failed to decode");

    assert_eq!(decoded.payload.len(), 0);
    assert_eq!(decoded, envelope);
}

#[test]
fn test_large_payload() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = vec![0x42u8; 10_000]; // 10KB payload

    let envelope = MessageEnvelope::new(sender.uid(), recipient.uid(), MessageType::Text, payload.clone());

    // CBOR roundtrip
    let cbor_encoded = envelope.to_cbor().expect("Failed to encode to CBOR");
    let cbor_decoded = MessageEnvelope::from_cbor(&cbor_encoded)
        .expect("Failed to decode from CBOR");
    assert_eq!(cbor_decoded.payload, payload);

    // JSON roundtrip
    let json_encoded = envelope.to_json().expect("Failed to encode to JSON");
    let json_decoded = MessageEnvelope::from_json(&json_encoded)
        .expect("Failed to decode from JSON");
    assert_eq!(json_decoded.payload, payload);
}

// Encryption Tests

#[test]
fn test_encrypted_message_envelope_creation() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Secret message!".to_vec();

    // Derive shared secret
    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let shared_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive shared secret");

    let envelope = MessageEnvelope::new_encrypted(
        sender.uid(),
        recipient.uid(),
        MessageType::Text,
        payload.clone(),
        &shared_secret,
    ).expect("Failed to create encrypted envelope");

    assert_eq!(envelope.version, PROTOCOL_VERSION);
    assert_eq!(envelope.from_uid, sender.uid().as_str());
    assert_eq!(envelope.to_uid, recipient.uid().as_str());
    assert_eq!(envelope.message_type, MessageType::Text);
    assert!(envelope.encrypted);
    assert!(!envelope.payload.is_empty());
    assert_ne!(envelope.payload, payload); // Payload should be encrypted
}

#[test]
fn test_encrypted_message_roundtrip() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Hello encrypted world!".to_vec();

    // Derive shared secret (both sender and recipient can derive the same secret)
    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let sender_x25519_pub: [u8; 32] = sender.x25519_public.as_slice().try_into().unwrap();

    let sender_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive sender secret");
    let recipient_secret = recipient.derive_shared_secret(&sender_x25519_pub)
        .expect("Failed to derive recipient secret");

    // Secrets should match
    assert_eq!(sender_secret, recipient_secret);

    // Sender creates encrypted message
    let encrypted_envelope = MessageEnvelope::new_encrypted(
        sender.uid(),
        recipient.uid(),
        MessageType::Text,
        payload.clone(),
        &sender_secret,
    ).expect("Failed to create encrypted envelope");

    // Recipient decrypts the message
    let decrypted_payload = encrypted_envelope.decrypt_payload(&recipient_secret)
        .expect("Failed to decrypt payload");

    assert_eq!(decrypted_payload, payload);
}

#[test]
fn test_encrypted_text_convenience_method() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Convenience test".to_vec();

    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let shared_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive shared secret");

    let envelope = MessageEnvelope::new_text_encrypted(
        sender.uid(),
        recipient.uid(),
        payload.clone(),
        &shared_secret,
    ).expect("Failed to create encrypted text envelope");

    assert!(envelope.encrypted);
    assert_eq!(envelope.message_type, MessageType::Text);

    let decrypted = envelope.decrypt_payload(&shared_secret)
        .expect("Failed to decrypt");
    assert_eq!(decrypted, payload);
}

#[test]
fn test_encrypted_delete_convenience_method() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Delete command".to_vec();

    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let shared_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive shared secret");

    let envelope = MessageEnvelope::new_delete_encrypted(
        sender.uid(),
        recipient.uid(),
        payload.clone(),
        &shared_secret,
    ).expect("Failed to create encrypted delete envelope");

    assert!(envelope.encrypted);
    assert_eq!(envelope.message_type, MessageType::Delete);

    let decrypted = envelope.decrypt_payload(&shared_secret)
        .expect("Failed to decrypt");
    assert_eq!(decrypted, payload);
}

#[test]
fn test_wrong_key_decryption_fails() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let attacker = KeyPair::generate().expect("Failed to generate attacker keypair");
    let payload = b"Secret message".to_vec();

    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let attacker_x25519_pub: [u8; 32] = attacker.x25519_public.as_slice().try_into().unwrap();

    let shared_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive shared secret");
    let wrong_secret = sender.derive_shared_secret(&attacker_x25519_pub)
        .expect("Failed to derive wrong secret");

    let envelope = MessageEnvelope::new_encrypted(
        sender.uid(),
        recipient.uid(),
        MessageType::Text,
        payload,
        &shared_secret,
    ).expect("Failed to create encrypted envelope");

    // Decryption with wrong key should fail
    let result = envelope.decrypt_payload(&wrong_secret);
    assert!(result.is_err());
}

#[test]
fn test_get_payload_with_encryption() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Test message".to_vec();

    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let shared_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive shared secret");

    // Test encrypted envelope
    let encrypted_envelope = MessageEnvelope::new_encrypted(
        sender.uid(),
        recipient.uid(),
        MessageType::Text,
        payload.clone(),
        &shared_secret,
    ).expect("Failed to create encrypted envelope");

    // Should fail without secret
    assert!(encrypted_envelope.get_payload(None).is_err());

    // Should succeed with secret
    let decrypted = encrypted_envelope.get_payload(Some(&shared_secret))
        .expect("Failed to get encrypted payload");
    assert_eq!(decrypted, payload);

    // Test plaintext envelope
    let plaintext_envelope = MessageEnvelope::new_text(
        sender.uid(),
        recipient.uid(),
        payload.clone(),
    );

    // Should work without secret
    let plaintext = plaintext_envelope.get_payload(None)
        .expect("Failed to get plaintext payload");
    assert_eq!(plaintext, payload);
}

#[test]
fn test_encrypted_envelope_cbor_roundtrip() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"CBOR encrypted test".to_vec();

    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let shared_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive shared secret");

    let original = MessageEnvelope::new_encrypted(
        sender.uid(),
        recipient.uid(),
        MessageType::Text,
        payload.clone(),
        &shared_secret,
    ).expect("Failed to create encrypted envelope");

    // Encode to CBOR
    let encoded = original.to_cbor().expect("Failed to encode to CBOR");

    // Decode from CBOR
    let decoded = MessageEnvelope::from_cbor(&encoded)
        .expect("Failed to decode from CBOR");

    // Verify envelope fields match
    assert_eq!(decoded.id, original.id);
    assert_eq!(decoded.encrypted, original.encrypted);
    assert!(decoded.encrypted);

    // Decrypt payload
    let decrypted = decoded.decrypt_payload(&shared_secret)
        .expect("Failed to decrypt");
    assert_eq!(decrypted, payload);
}

#[test]
fn test_encrypted_envelope_json_roundtrip() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"JSON encrypted test".to_vec();

    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let shared_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive shared secret");

    let original = MessageEnvelope::new_encrypted(
        sender.uid(),
        recipient.uid(),
        MessageType::Text,
        payload.clone(),
        &shared_secret,
    ).expect("Failed to create encrypted envelope");

    // Encode to JSON
    let encoded = original.to_json().expect("Failed to encode to JSON");

    // Decode from JSON
    let decoded = MessageEnvelope::from_json(&encoded)
        .expect("Failed to decode from JSON");

    // Verify envelope fields match
    assert_eq!(decoded.encrypted, original.encrypted);
    assert!(decoded.encrypted);

    // Decrypt payload
    let decrypted = decoded.decrypt_payload(&shared_secret)
        .expect("Failed to decrypt");
    assert_eq!(decrypted, payload);
}

#[test]
fn test_decrypt_plaintext_envelope_fails() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = b"Plaintext message".to_vec();

    let plaintext_envelope = MessageEnvelope::new_text(
        sender.uid(),
        recipient.uid(),
        payload,
    );

    let fake_secret = [0u8; 32];

    // Should fail to decrypt plaintext envelope
    let result = plaintext_envelope.decrypt_payload(&fake_secret);
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(e.to_string().contains("not encrypted"));
    }
}

#[test]
fn test_encrypted_flag_serialization() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");

    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let shared_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive shared secret");

    // Create encrypted envelope
    let encrypted = MessageEnvelope::new_encrypted(
        sender.uid(),
        recipient.uid(),
        MessageType::Text,
        b"encrypted".to_vec(),
        &shared_secret,
    ).expect("Failed to create encrypted envelope");

    // Create plaintext envelope
    let plaintext = MessageEnvelope::new_text(
        sender.uid(),
        recipient.uid(),
        b"plaintext".to_vec(),
    );

    // Serialize both
    let encrypted_cbor = encrypted.to_cbor().expect("Failed to encode encrypted");
    let plaintext_cbor = plaintext.to_cbor().expect("Failed to encode plaintext");

    // Deserialize
    let encrypted_decoded = MessageEnvelope::from_cbor(&encrypted_cbor)
        .expect("Failed to decode encrypted");
    let plaintext_decoded = MessageEnvelope::from_cbor(&plaintext_cbor)
        .expect("Failed to decode plaintext");

    // Verify encrypted flags
    assert!(encrypted_decoded.encrypted);
    assert!(!plaintext_decoded.encrypted);
}

#[test]
fn test_encrypted_empty_payload() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = Vec::new();

    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let shared_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive shared secret");

    let envelope = MessageEnvelope::new_encrypted(
        sender.uid(),
        recipient.uid(),
        MessageType::Text,
        payload.clone(),
        &shared_secret,
    ).expect("Failed to create encrypted envelope");

    let decrypted = envelope.decrypt_payload(&shared_secret)
        .expect("Failed to decrypt empty payload");

    assert_eq!(decrypted, payload);
    assert!(decrypted.is_empty());
}

#[test]
fn test_encrypted_large_payload() {
    let sender = KeyPair::generate().expect("Failed to generate sender keypair");
    let recipient = KeyPair::generate().expect("Failed to generate recipient keypair");
    let payload = vec![42u8; 50_000]; // 50 KB

    let recipient_x25519_pub: [u8; 32] = recipient.x25519_public.as_slice().try_into().unwrap();
    let shared_secret = sender.derive_shared_secret(&recipient_x25519_pub)
        .expect("Failed to derive shared secret");

    let envelope = MessageEnvelope::new_encrypted(
        sender.uid(),
        recipient.uid(),
        MessageType::Text,
        payload.clone(),
        &shared_secret,
    ).expect("Failed to create encrypted envelope");

    let decrypted = envelope.decrypt_payload(&shared_secret)
        .expect("Failed to decrypt large payload");

    assert_eq!(decrypted, payload);
}
