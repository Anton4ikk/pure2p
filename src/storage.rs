//! Local storage module
//!
//! This module handles persistent storage including:
//! - Message history
//! - Peer information
//! - User data
//! - Configuration

use crate::{crypto::UID, Error, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents a contact/peer in the P2P network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// Unique identifier (derived from public key)
    pub uid: String,
    /// IP address and port (e.g., "192.168.1.100:8080")
    pub ip: String,
    /// Ed25519 public key bytes
    pub pubkey: Vec<u8>,
    /// Expiration timestamp for this contact entry
    pub expiry: DateTime<Utc>,
    /// Whether this contact is currently active
    pub is_active: bool,
}

impl Contact {
    /// Create a new contact
    pub fn new(uid: String, ip: String, pubkey: Vec<u8>, expiry: DateTime<Utc>) -> Self {
        Self {
            uid,
            ip,
            pubkey,
            expiry,
            is_active: true, // New contacts are active by default
        }
    }

    /// Check if the contact has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expiry
    }

    /// Activate this contact
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// Deactivate this contact
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

/// Internal struct for contact token serialization
#[derive(Debug, Serialize, Deserialize)]
struct ContactTokenData {
    ip: String,
    pubkey: Vec<u8>,
    expiry: DateTime<Utc>,
}

/// Generate a contact token from IP, public key, and expiry
///
/// The token is serialized using CBOR and encoded as base64 URL-safe without padding.
///
/// # Arguments
/// * `ip` - IP address and port (e.g., "192.168.1.100:8080")
/// * `pubkey` - Ed25519 public key bytes
/// * `expiry` - Expiration timestamp
///
/// # Returns
/// A base64-encoded contact token string
pub fn generate_contact_token(ip: &str, pubkey: &[u8], expiry: DateTime<Utc>) -> String {
    let data = ContactTokenData {
        ip: ip.to_string(),
        pubkey: pubkey.to_vec(),
        expiry,
    };

    // Serialize to CBOR
    let cbor = serde_cbor::to_vec(&data).expect("Failed to serialize contact token data");

    // Encode as base64 URL-safe
    URL_SAFE_NO_PAD.encode(cbor)
}

/// Parse a contact token and validate expiry
///
/// Decodes a base64 URL-safe token, deserializes CBOR data, and validates the expiry.
///
/// # Arguments
/// * `token` - Base64-encoded contact token string
///
/// # Returns
/// A `Contact` struct if the token is valid and not expired
///
/// # Errors
/// Returns an error if:
/// - Token decoding fails
/// - CBOR deserialization fails
/// - Contact has expired
pub fn parse_contact_token(token: &str) -> Result<Contact> {
    // Decode from base64
    let cbor = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|e| Error::Storage(format!("Invalid base64 token: {}", e)))?;

    // Deserialize from CBOR
    let data: ContactTokenData = serde_cbor::from_slice(&cbor)
        .map_err(|e| Error::CborSerialization(format!("Invalid token data: {}", e)))?;

    // Validate expiry
    if Utc::now() > data.expiry {
        return Err(Error::Storage("Contact token has expired".to_string()));
    }

    // Generate UID from public key
    let uid = UID::from_public_key(&data.pubkey);

    // Create contact
    Ok(Contact::new(uid.to_string(), data.ip, data.pubkey, data.expiry))
}

/// Represents a stored message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID
    pub id: String,
    /// Sender peer ID
    pub sender: String,
    /// Recipient peer ID
    pub recipient: String,
    /// Message content (encrypted)
    pub content: Vec<u8>,
    /// Timestamp
    pub timestamp: i64,
    /// Delivery status
    pub delivered: bool,
}

/// Local storage manager
pub struct Storage {
    conn: Option<Connection>,
}

impl Storage {
    /// Create a new storage instance
    pub fn new() -> Self {
        Self { conn: None }
    }

    /// Initialize storage with a database file
    pub fn init<P: AsRef<Path>>(&mut self, _path: P) -> Result<()> {
        // TODO: Implement database initialization
        // - Open SQLite connection
        // - Create tables if they don't exist
        // - Run migrations if needed
        Err(Error::Storage("Not yet implemented".to_string()))
    }

    /// Store a message
    pub fn store_message(&self, _message: &Message) -> Result<()> {
        // TODO: Implement message storage
        Err(Error::Storage("Not yet implemented".to_string()))
    }

    /// Retrieve a message by ID
    pub fn get_message(&self, _id: &str) -> Result<Option<Message>> {
        // TODO: Implement message retrieval
        Err(Error::Storage("Not yet implemented".to_string()))
    }

    /// Get all messages for a conversation
    pub fn get_conversation(&self, _peer_id: &str) -> Result<Vec<Message>> {
        // TODO: Implement conversation retrieval
        Err(Error::Storage("Not yet implemented".to_string()))
    }

    /// Delete a message
    pub fn delete_message(&self, _id: &str) -> Result<()> {
        // TODO: Implement message deletion
        Err(Error::Storage("Not yet implemented".to_string()))
    }

    /// Mark a message as delivered
    pub fn mark_delivered(&self, _id: &str) -> Result<()> {
        // TODO: Implement delivery status update
        Err(Error::Storage("Not yet implemented".to_string()))
    }

    /// Get undelivered messages
    pub fn get_undelivered(&self) -> Result<Vec<Message>> {
        // TODO: Implement undelivered message retrieval
        Err(Error::Storage("Not yet implemented".to_string()))
    }

    /// Clear all storage
    pub fn clear(&self) -> Result<()> {
        // TODO: Implement storage clearing
        Err(Error::Storage("Not yet implemented".to_string()))
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_storage_creation() {
        let storage = Storage::new();
        assert!(storage.conn.is_none());
    }

    #[test]
    fn test_contact_creation() {
        let uid = "a1b2c3d4e5f6".to_string();
        let ip = "192.168.1.100:8080".to_string();
        let pubkey = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let expiry = Utc::now() + Duration::days(30);

        let contact = Contact::new(uid.clone(), ip.clone(), pubkey.clone(), expiry);

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

    #[test]
    fn test_generate_contact_token() {
        let ip = "192.168.1.100:8080";
        let pubkey = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let expiry = Utc::now() + Duration::days(30);

        let token = generate_contact_token(ip, &pubkey, expiry);

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
        let token = generate_contact_token(ip, &pubkey, expiry);

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
        let token = generate_contact_token(ip, &pubkey, expiry);

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
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
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
        use crate::crypto::KeyPair;

        // Generate a real keypair
        let keypair = KeyPair::generate().expect("Failed to generate keypair");
        let ip = "203.0.113.10:8080";
        let expiry = Utc::now() + Duration::days(90);

        // Generate token with real public key
        let token = generate_contact_token(ip, &keypair.public_key, expiry);

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

        let token1 = generate_contact_token("192.168.1.1:8080", &[1, 2, 3], expiry);
        let token2 = generate_contact_token("192.168.1.2:8080", &[1, 2, 3], expiry);
        let token3 = generate_contact_token("192.168.1.1:8080", &[4, 5, 6], expiry);

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
        let token1 = generate_contact_token(ip, &pubkey, expiry);
        let token2 = generate_contact_token(ip, &pubkey, expiry);

        // Should produce identical tokens
        assert_eq!(token1, token2);
    }
}
