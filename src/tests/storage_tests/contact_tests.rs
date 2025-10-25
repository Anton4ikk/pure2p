// Contact Tests - Testing Contact struct and its methods

use crate::storage::*;
use chrono::{Duration, Utc};

#[test]
fn test_storage_creation() {
    let storage = Storage::new();
    assert!(storage._conn.is_none());
}

#[test]
fn test_contact_creation() {
    let uid = "a1b2c3d4e5f6".to_string();
    let ip = "192.168.1.100:8080".to_string();
    let pubkey = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let expiry = Utc::now() + Duration::days(30);

    let contact = Contact::new(uid.clone(), ip.clone(), pubkey.clone(), vec![99u8; 32], expiry);

    assert_eq!(contact.uid, uid);
    assert_eq!(contact.ip, ip);
    assert_eq!(contact.pubkey, pubkey);
    assert_eq!(contact.expiry, expiry);
    assert!(contact.is_active); // Should be active by default
}

#[test]
fn test_contact_is_expired_future() {
    let expiry = Utc::now() + Duration::days(30);
    let contact = Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        expiry,
    );

    assert!(!contact.is_expired(), "Contact with future expiry should not be expired");
}

#[test]
fn test_contact_is_expired_past() {
    let expiry = Utc::now() - Duration::days(1);
    let contact = Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        expiry,
    );

    assert!(contact.is_expired(), "Contact with past expiry should be expired");
}

#[test]
fn test_contact_activate() {
    let expiry = Utc::now() + Duration::days(30);
    let mut contact = Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        expiry,
    );

    // Deactivate first
    contact.deactivate();
    assert!(!contact.is_active);

    // Then activate
    contact.activate();
    assert!(contact.is_active);
}

#[test]
fn test_contact_deactivate() {
    let expiry = Utc::now() + Duration::days(30);
    let mut contact = Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        expiry,
    );

    assert!(contact.is_active); // Starts active

    contact.deactivate();
    assert!(!contact.is_active);
}

#[test]
fn test_contact_serialize_deserialize_json() {
    let expiry = Utc::now() + Duration::days(30);
    let original = Contact::new(
        "a1b2c3d4e5f6".to_string(),
        "192.168.1.100:8080".to_string(),
        vec![10, 20, 30, 40, 50],
        vec![99u8; 32], // x25519_pubkey placeholder
        expiry,
    );

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("Failed to serialize to JSON");

    // Deserialize from JSON
    let deserialized: Contact = serde_json::from_str(&json).expect("Failed to deserialize from JSON");

    // Verify all fields match
    assert_eq!(deserialized.uid, original.uid);
    assert_eq!(deserialized.ip, original.ip);
    assert_eq!(deserialized.pubkey, original.pubkey);
    assert_eq!(deserialized.expiry, original.expiry);
    assert_eq!(deserialized.is_active, original.is_active);
}

#[test]
fn test_contact_serialize_deserialize_cbor() {
    let expiry = Utc::now() + Duration::days(30);
    let original = Contact::new(
        "x9y8z7w6v5u4".to_string(),
        "10.0.0.1:9000".to_string(),
        vec![100, 101, 102, 103],
        vec![99u8; 32], // x25519_pubkey placeholder
        expiry,
    );

    // Serialize to CBOR
    let cbor = serde_cbor::to_vec(&original).expect("Failed to serialize to CBOR");

    // Deserialize from CBOR
    let deserialized: Contact = serde_cbor::from_slice(&cbor).expect("Failed to deserialize from CBOR");

    // Verify all fields match
    assert_eq!(deserialized.uid, original.uid);
    assert_eq!(deserialized.ip, original.ip);
    assert_eq!(deserialized.pubkey, original.pubkey);
    assert_eq!(deserialized.expiry, original.expiry);
    assert_eq!(deserialized.is_active, original.is_active);
}

#[test]
fn test_contact_clone() {
    let expiry = Utc::now() + Duration::days(30);
    let original = Contact::new(
        "clone_test".to_string(),
        "localhost:8080".to_string(),
        vec![1, 2, 3, 4],
        vec![99u8; 32], // x25519_pubkey placeholder
        expiry,
    );

    let cloned = original.clone();

    assert_eq!(cloned.uid, original.uid);
    assert_eq!(cloned.ip, original.ip);
    assert_eq!(cloned.pubkey, original.pubkey);
    assert_eq!(cloned.expiry, original.expiry);
    assert_eq!(cloned.is_active, original.is_active);
}

#[test]
fn test_contact_multiple_activate_deactivate() {
    let expiry = Utc::now() + Duration::days(30);
    let mut contact = Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        expiry,
    );

    // Multiple activate/deactivate cycles
    contact.deactivate();
    assert!(!contact.is_active);

    contact.activate();
    assert!(contact.is_active);

    contact.activate(); // Double activate should be idempotent
    assert!(contact.is_active);

    contact.deactivate();
    assert!(!contact.is_active);

    contact.deactivate(); // Double deactivate should be idempotent
    assert!(!contact.is_active);
}
