//! Protocol module
//!
//! This module defines the core message protocol for Pure2P including:
//! - Message envelope structure
//! - Serialization/deserialization (CBOR and JSON)
//! - Protocol versioning
//! - Message routing
//! - End-to-end encryption support

use crate::crypto::{encrypt_message, decrypt_message, EncryptedEnvelope, UID};
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
///
/// Supports both encrypted and plaintext payloads. When `encrypted` is true,
/// the `payload` field contains an encrypted `EncryptedEnvelope` (serialized to CBOR).
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

    /// Whether the payload is encrypted
    pub encrypted: bool,

    /// Message payload
    /// - If `encrypted` is true: Contains serialized `EncryptedEnvelope` (CBOR)
    /// - If `encrypted` is false: Contains plaintext data
    pub payload: Vec<u8>,
}

impl MessageEnvelope {
    /// Create a new plaintext message envelope with specified message type
    ///
    /// Note: This creates an unencrypted envelope. Use `new_encrypted()` for E2E encryption.
    pub fn new(from_uid: &UID, to_uid: &UID, message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            version: PROTOCOL_VERSION,
            from_uid: from_uid.as_str().to_string(),
            to_uid: to_uid.as_str().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            message_type,
            encrypted: false,
            payload,
        }
    }

    /// Create a new encrypted message envelope
    ///
    /// Encrypts the payload using XChaCha20-Poly1305 with the provided shared secret.
    ///
    /// # Arguments
    ///
    /// * `from_uid` - Sender's UID
    /// * `to_uid` - Recipient's UID
    /// * `message_type` - Type of message (Text/Delete)
    /// * `payload` - Plaintext payload to encrypt
    /// * `shared_secret` - 32-byte shared secret from X25519 key exchange
    ///
    /// # Returns
    ///
    /// An encrypted message envelope
    pub fn new_encrypted(
        from_uid: &UID,
        to_uid: &UID,
        message_type: MessageType,
        payload: Vec<u8>,
        shared_secret: &[u8; 32],
    ) -> Result<Self> {
        // Encrypt the payload
        let encrypted_envelope = encrypt_message(shared_secret, &payload)?;

        // Serialize the encrypted envelope to CBOR
        let encrypted_payload = serde_cbor::to_vec(&encrypted_envelope)
            .map_err(|e| Error::CborSerialization(format!("Failed to serialize encrypted envelope: {}", e)))?;

        Ok(Self {
            id: Uuid::new_v4(),
            version: PROTOCOL_VERSION,
            from_uid: from_uid.as_str().to_string(),
            to_uid: to_uid.as_str().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            message_type,
            encrypted: true,
            payload: encrypted_payload,
        })
    }

    /// Create a new text message envelope (plaintext, convenience method)
    pub fn new_text(from_uid: &UID, to_uid: &UID, payload: Vec<u8>) -> Self {
        Self::new(from_uid, to_uid, MessageType::Text, payload)
    }

    /// Create a new delete message envelope (plaintext, convenience method)
    pub fn new_delete(from_uid: &UID, to_uid: &UID, payload: Vec<u8>) -> Self {
        Self::new(from_uid, to_uid, MessageType::Delete, payload)
    }

    /// Create a new encrypted text message envelope (convenience method)
    pub fn new_text_encrypted(
        from_uid: &UID,
        to_uid: &UID,
        payload: Vec<u8>,
        shared_secret: &[u8; 32],
    ) -> Result<Self> {
        Self::new_encrypted(from_uid, to_uid, MessageType::Text, payload, shared_secret)
    }

    /// Create a new encrypted delete message envelope (convenience method)
    pub fn new_delete_encrypted(
        from_uid: &UID,
        to_uid: &UID,
        payload: Vec<u8>,
        shared_secret: &[u8; 32],
    ) -> Result<Self> {
        Self::new_encrypted(from_uid, to_uid, MessageType::Delete, payload, shared_secret)
    }

    /// Decrypt the payload if the message is encrypted
    ///
    /// # Arguments
    ///
    /// * `shared_secret` - 32-byte shared secret from X25519 key exchange
    ///
    /// # Returns
    ///
    /// The decrypted plaintext payload
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The message is not encrypted
    /// - Decryption fails (wrong key or tampered data)
    pub fn decrypt_payload(&self, shared_secret: &[u8; 32]) -> Result<Vec<u8>> {
        if !self.encrypted {
            return Err(Error::Crypto("Message is not encrypted".to_string()));
        }

        // Deserialize the encrypted envelope from CBOR
        let encrypted_envelope: EncryptedEnvelope = serde_cbor::from_slice(&self.payload)
            .map_err(|e| Error::CborSerialization(format!("Failed to deserialize encrypted envelope: {}", e)))?;

        // Decrypt the payload
        decrypt_message(shared_secret, &encrypted_envelope)
    }

    /// Get the payload (decrypting if necessary)
    ///
    /// # Arguments
    ///
    /// * `shared_secret` - Optional shared secret for decryption (required if message is encrypted)
    ///
    /// # Returns
    ///
    /// The plaintext payload
    pub fn get_payload(&self, shared_secret: Option<&[u8; 32]>) -> Result<Vec<u8>> {
        if self.encrypted {
            match shared_secret {
                Some(secret) => self.decrypt_payload(secret),
                None => Err(Error::Crypto("Shared secret required to decrypt message".to_string())),
            }
        } else {
            Ok(self.payload.clone())
        }
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

