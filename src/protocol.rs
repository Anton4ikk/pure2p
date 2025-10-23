//! Protocol module
//!
//! This module defines the core message protocol for Pure2P including:
//! - Message envelope structure
//! - Serialization/deserialization (CBOR and JSON)
//! - Protocol versioning
//! - Message routing

use crate::crypto::UID;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Message type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    /// Regular text message
    Text,
    /// Delete chat message
    Delete,
}

/// Message envelope that wraps all P2P communications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageEnvelope {
    /// Unique message identifier
    pub id: Uuid,

    /// Protocol version for compatibility checking
    pub version: u8,

    /// Sender's unique identifier
    pub from_uid: String,

    /// Recipient's unique identifier
    pub to_uid: String,

    /// Unix timestamp in milliseconds
    pub timestamp: i64,

    /// Message type (Text/Delete)
    pub message_type: MessageType,

    /// Message payload (encrypted or plaintext)
    pub payload: Vec<u8>,
}

impl MessageEnvelope {
    /// Create a new message envelope with specified message type
    pub fn new(from_uid: &UID, to_uid: &UID, message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            version: PROTOCOL_VERSION,
            from_uid: from_uid.as_str().to_string(),
            to_uid: to_uid.as_str().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            message_type,
            payload,
        }
    }

    /// Create a new text message envelope (convenience method)
    pub fn new_text(from_uid: &UID, to_uid: &UID, payload: Vec<u8>) -> Self {
        Self::new(from_uid, to_uid, MessageType::Text, payload)
    }

    /// Create a new delete message envelope (convenience method)
    pub fn new_delete(from_uid: &UID, to_uid: &UID, payload: Vec<u8>) -> Self {
        Self::new(from_uid, to_uid, MessageType::Delete, payload)
    }

    /// Encode the message envelope to CBOR format
    pub fn to_cbor(&self) -> Result<Vec<u8>> {
        serde_cbor::to_vec(self)
            .map_err(|e| Error::CborSerialization(e.to_string()))
    }

    /// Decode a message envelope from CBOR format
    pub fn from_cbor(data: &[u8]) -> Result<Self> {
        serde_cbor::from_slice(data)
            .map_err(|e| Error::CborSerialization(e.to_string()))
    }

    /// Encode the message envelope to JSON format
    pub fn to_json(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(Error::JsonSerialization)
    }

    /// Decode a message envelope from JSON format
    pub fn from_json(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).map_err(Error::JsonSerialization)
    }

    /// Encode to JSON string (pretty-printed)
    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(Error::JsonSerialization)
    }

    /// Check if the message envelope has a valid version
    pub fn is_version_compatible(&self) -> bool {
        self.version <= PROTOCOL_VERSION
    }

    /// Get the message age in milliseconds
    pub fn age_ms(&self) -> i64 {
        chrono::Utc::now().timestamp_millis() - self.timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
