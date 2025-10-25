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

