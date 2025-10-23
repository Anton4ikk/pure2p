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

/// Represents a chat conversation with a contact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    /// Contact UID this chat is with
    pub contact_uid: String,
    /// Messages in this conversation
    pub messages: Vec<Message>,
    /// Whether this chat is active (unread messages present)
    pub is_active: bool,
}

impl Chat {
    /// Create a new chat with a contact
    pub fn new(contact_uid: String) -> Self {
        Self {
            contact_uid,
            messages: Vec::new(),
            is_active: false,
        }
    }

    /// Append a message to this chat
    pub fn append_message(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    /// Mark chat as having unread messages (active)
    pub fn mark_unread(&mut self) {
        self.is_active = true;
    }

    /// Mark chat as read (inactive)
    pub fn mark_read(&mut self) {
        self.is_active = false;
    }
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Default contact expiry duration in days
    pub default_contact_expiry_days: u32,
    /// Auto-accept contact requests
    pub auto_accept_contacts: bool,
    /// Maximum retry attempts for message delivery
    pub max_message_retries: u32,
    /// Base delay for retry backoff in milliseconds
    pub retry_base_delay_ms: u64,
    /// Enable notifications
    pub enable_notifications: bool,
    /// Global retry interval in milliseconds (default 10 minutes)
    pub global_retry_interval_ms: u64,
}

impl Settings {
    /// Update the global retry interval at runtime
    pub fn set_global_retry_interval_ms(&mut self, interval_ms: u64) {
        self.global_retry_interval_ms = interval_ms;
    }

    /// Get the global retry interval in milliseconds
    pub fn get_global_retry_interval_ms(&self) -> u64 {
        self.global_retry_interval_ms
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_contact_expiry_days: 30,
            auto_accept_contacts: false,
            max_message_retries: 5,
            retry_base_delay_ms: 1000,
            enable_notifications: true,
            global_retry_interval_ms: 600_000, // 10 minutes = 600,000 ms
        }
    }
}

/// Persistent application state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    /// List of contacts
    pub contacts: Vec<Contact>,
    /// List of chat conversations
    pub chats: Vec<Chat>,
    /// Queued messages awaiting delivery
    pub message_queue: Vec<String>, // Message IDs in queue
    /// Application settings
    pub settings: Settings,
}

impl AppState {
    /// Create a new empty application state
    pub fn new() -> Self {
        Self {
            contacts: Vec::new(),
            chats: Vec::new(),
            message_queue: Vec::new(),
            settings: Settings::default(),
        }
    }

    /// Save the application state to a file
    ///
    /// # Arguments
    /// * `path` - Path to the state file (e.g., "pure2p_state.json")
    ///
    /// # Errors
    /// Returns an error if file operations or serialization fail
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
            .map_err(|e| Error::Storage(format!("Failed to write state file: {}", e)))?;
        Ok(())
    }

    /// Load the application state from a file
    ///
    /// # Arguments
    /// * `path` - Path to the state file
    ///
    /// # Returns
    /// A loaded `AppState` or a new empty state if the file doesn't exist
    ///
    /// # Errors
    /// Returns an error if the file exists but cannot be read or deserialized
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();

        // If file doesn't exist, return a new empty state
        if !path_ref.exists() {
            return Ok(Self::new());
        }

        // Read and deserialize the file
        let json = std::fs::read_to_string(path_ref)
            .map_err(|e| Error::Storage(format!("Failed to read state file: {}", e)))?;

        let state: AppState = serde_json::from_str(&json)?;
        Ok(state)
    }

    /// Save state using CBOR format (more compact)
    ///
    /// # Arguments
    /// * `path` - Path to the state file (e.g., "pure2p_state.cbor")
    pub fn save_cbor<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let cbor = serde_cbor::to_vec(self)
            .map_err(|e| Error::CborSerialization(format!("Failed to serialize state: {}", e)))?;
        std::fs::write(path, cbor)
            .map_err(|e| Error::Storage(format!("Failed to write state file: {}", e)))?;
        Ok(())
    }

    /// Load state from CBOR format
    ///
    /// # Arguments
    /// * `path` - Path to the CBOR state file
    pub fn load_cbor<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();

        // If file doesn't exist, return a new empty state
        if !path_ref.exists() {
            return Ok(Self::new());
        }

        // Read and deserialize the file
        let cbor = std::fs::read(path_ref)
            .map_err(|e| Error::Storage(format!("Failed to read state file: {}", e)))?;

        let state: AppState = serde_cbor::from_slice(&cbor)
            .map_err(|e| Error::CborSerialization(format!("Failed to deserialize state: {}", e)))?;
        Ok(state)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
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

    #[test]
    fn test_chat_creation() {
        let chat = Chat::new("test_uid_123".to_string());

        assert_eq!(chat.contact_uid, "test_uid_123");
        assert!(chat.messages.is_empty());
        assert!(!chat.is_active);
    }

    #[test]
    fn test_chat_append_message() {
        let mut chat = Chat::new("uid_456".to_string());

        let msg1 = Message {
            id: "msg_1".to_string(),
            sender: "sender_1".to_string(),
            recipient: "uid_456".to_string(),
            content: vec![1, 2, 3],
            timestamp: 1000,
            delivered: false,
        };

        chat.append_message(msg1);
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].id, "msg_1");

        let msg2 = Message {
            id: "msg_2".to_string(),
            sender: "sender_2".to_string(),
            recipient: "uid_456".to_string(),
            content: vec![4, 5, 6],
            timestamp: 2000,
            delivered: true,
        };

        chat.append_message(msg2);
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[1].timestamp, 2000);
    }

    #[test]
    fn test_chat_active_management() {
        let mut chat = Chat::new("uid_789".to_string());

        // Initially not active
        assert!(!chat.is_active);

        // Mark as unread (active)
        chat.mark_unread();
        assert!(chat.is_active);

        // Mark as read (inactive)
        chat.mark_read();
        assert!(!chat.is_active);

        // Can mark unread multiple times
        chat.mark_unread();
        chat.mark_unread();
        assert!(chat.is_active);
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();

        assert_eq!(settings.default_contact_expiry_days, 30);
        assert!(!settings.auto_accept_contacts);
        assert_eq!(settings.max_message_retries, 5);
        assert_eq!(settings.retry_base_delay_ms, 1000);
        assert!(settings.enable_notifications);
        assert_eq!(settings.global_retry_interval_ms, 600_000); // 10 minutes
    }

    #[test]
    fn test_settings_global_retry_interval() {
        let mut settings = Settings::default();

        // Default should be 10 minutes (600,000 ms)
        assert_eq!(settings.get_global_retry_interval_ms(), 600_000);

        // Update to 5 minutes
        settings.set_global_retry_interval_ms(300_000);
        assert_eq!(settings.get_global_retry_interval_ms(), 300_000);

        // Update to 30 minutes
        settings.set_global_retry_interval_ms(1_800_000);
        assert_eq!(settings.get_global_retry_interval_ms(), 1_800_000);
    }

    #[test]
    fn test_settings_runtime_update() {
        let mut settings = Settings::default();

        // Change multiple settings at runtime
        settings.set_global_retry_interval_ms(120_000); // 2 minutes
        settings.max_message_retries = 10;
        settings.enable_notifications = false;

        assert_eq!(settings.global_retry_interval_ms, 120_000);
        assert_eq!(settings.max_message_retries, 10);
        assert!(!settings.enable_notifications);
    }

    #[test]
    fn test_app_state_creation() {
        let state = AppState::new();

        assert!(state.contacts.is_empty());
        assert!(state.chats.is_empty());
        assert!(state.message_queue.is_empty());
        assert_eq!(state.settings.default_contact_expiry_days, 30);
    }

    #[test]
    fn test_app_state_save_load_json() {
        use tempfile::NamedTempFile;

        // Create temp file
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        // Create state with some data
        let mut state = AppState::new();
        state.contacts.push(Contact::new(
            "test_uid".to_string(),
            "127.0.0.1:8080".to_string(),
            vec![1, 2, 3, 4],
            Utc::now() + Duration::days(30),
        ));
        state.chats.push(Chat::new("test_uid".to_string()));
        state.message_queue.push("msg_1".to_string());
        state.settings.enable_notifications = false;

        // Save state
        state.save(path).expect("Failed to save state");

        // Load state
        let loaded = AppState::load(path).expect("Failed to load state");

        // Verify all fields
        assert_eq!(loaded.contacts.len(), 1);
        assert_eq!(loaded.contacts[0].uid, "test_uid");
        assert_eq!(loaded.chats.len(), 1);
        assert_eq!(loaded.chats[0].contact_uid, "test_uid");
        assert_eq!(loaded.message_queue.len(), 1);
        assert_eq!(loaded.message_queue[0], "msg_1");
        assert!(!loaded.settings.enable_notifications);
    }

    #[test]
    fn test_app_state_save_load_cbor() {
        use tempfile::NamedTempFile;

        // Create temp file
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        // Create state with some data
        let mut state = AppState::new();
        state.contacts.push(Contact::new(
            "cbor_uid".to_string(),
            "192.168.1.1:9000".to_string(),
            vec![10, 20, 30],
            Utc::now() + Duration::days(60),
        ));
        state.settings.max_message_retries = 10;

        // Save state as CBOR
        state.save_cbor(path).expect("Failed to save state as CBOR");

        // Load state from CBOR
        let loaded = AppState::load_cbor(path).expect("Failed to load state from CBOR");

        // Verify fields
        assert_eq!(loaded.contacts.len(), 1);
        assert_eq!(loaded.contacts[0].uid, "cbor_uid");
        assert_eq!(loaded.settings.max_message_retries, 10);
    }

    #[test]
    fn test_app_state_load_nonexistent_file() {
        // Try to load from a file that doesn't exist
        let loaded = AppState::load("/tmp/nonexistent_pure2p_state.json")
            .expect("Should return empty state for nonexistent file");

        // Should return a new empty state
        assert!(loaded.contacts.is_empty());
        assert!(loaded.chats.is_empty());
        assert_eq!(loaded.settings.default_contact_expiry_days, 30);
    }

    #[test]
    fn test_app_state_load_cbor_nonexistent_file() {
        // Try to load from a CBOR file that doesn't exist
        let loaded = AppState::load_cbor("/tmp/nonexistent_pure2p_state.cbor")
            .expect("Should return empty state for nonexistent file");

        // Should return a new empty state
        assert!(loaded.contacts.is_empty());
        assert!(loaded.chats.is_empty());
    }

    #[test]
    fn test_app_state_with_multiple_contacts_and_chats() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        // Create state with multiple contacts and chats
        let mut state = AppState::new();

        for i in 0..5 {
            let uid = format!("uid_{}", i);
            state.contacts.push(Contact::new(
                uid.clone(),
                format!("10.0.0.{}:8080", i),
                vec![i as u8; 10],
                Utc::now() + Duration::days(30),
            ));

            let mut chat = Chat::new(uid.clone());
            let msg = Message {
                id: format!("msg_{}", i),
                sender: uid.clone(),
                recipient: "self".to_string(),
                content: vec![i as u8; 5],
                timestamp: 1000 * i as i64,
                delivered: true,
            };
            chat.append_message(msg);
            chat.mark_unread();
            state.chats.push(chat);
        }

        // Save and load
        state.save(path).expect("Failed to save state");
        let loaded = AppState::load(path).expect("Failed to load state");

        // Verify
        assert_eq!(loaded.contacts.len(), 5);
        assert_eq!(loaded.chats.len(), 5);
        assert!(loaded.chats[0].is_active);
        assert_eq!(loaded.chats[4].contact_uid, "uid_4");
        assert_eq!(loaded.chats[0].messages.len(), 1);
    }

    #[test]
    fn test_app_state_json_format_human_readable() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let mut state = AppState::new();
        state.contacts.push(Contact::new(
            "readable_uid".to_string(),
            "127.0.0.1:8080".to_string(),
            vec![1, 2, 3],
            Utc::now() + Duration::days(7),
        ));

        // Save state
        state.save(path).expect("Failed to save state");

        // Read raw file content
        let content = std::fs::read_to_string(path).expect("Failed to read file");

        // Verify it's human-readable JSON
        assert!(content.contains("readable_uid"));
        assert!(content.contains("127.0.0.1:8080"));
        assert!(content.contains("contacts"));
        assert!(content.contains("settings"));
    }

    #[test]
    fn test_settings_serialization() {
        let mut settings = Settings::default();
        settings.default_contact_expiry_days = 90;
        settings.auto_accept_contacts = true;

        // Serialize to JSON
        let json = serde_json::to_string(&settings).expect("Failed to serialize");

        // Deserialize
        let loaded: Settings = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(loaded.default_contact_expiry_days, 90);
        assert!(loaded.auto_accept_contacts);
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            id: "test_msg_123".to_string(),
            sender: "sender_uid".to_string(),
            recipient: "recipient_uid".to_string(),
            content: vec![10, 20, 30, 40, 50],
            timestamp: 1234567890,
            delivered: true,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&msg).expect("Failed to serialize message");

        // Deserialize
        let loaded: Message = serde_json::from_str(&json).expect("Failed to deserialize message");

        assert_eq!(loaded.id, "test_msg_123");
        assert_eq!(loaded.sender, "sender_uid");
        assert_eq!(loaded.recipient, "recipient_uid");
        assert_eq!(loaded.content, vec![10, 20, 30, 40, 50]);
        assert_eq!(loaded.timestamp, 1234567890);
        assert!(loaded.delivered);
    }

    #[test]
    fn test_chat_with_messages_serialization() {
        let mut chat = Chat::new("contact_123".to_string());

        // Add multiple messages
        for i in 0..3 {
            let msg = Message {
                id: format!("msg_{}", i),
                sender: "sender".to_string(),
                recipient: "contact_123".to_string(),
                content: vec![i as u8; 10],
                timestamp: 1000 * i as i64,
                delivered: i % 2 == 0,
            };
            chat.append_message(msg);
        }
        chat.mark_unread();

        // Serialize to JSON
        let json = serde_json::to_string(&chat).expect("Failed to serialize chat");

        // Deserialize
        let loaded: Chat = serde_json::from_str(&json).expect("Failed to deserialize chat");

        assert_eq!(loaded.contact_uid, "contact_123");
        assert_eq!(loaded.messages.len(), 3);
        assert!(loaded.is_active);
        assert_eq!(loaded.messages[0].id, "msg_0");
        assert_eq!(loaded.messages[2].timestamp, 2000);
    }
}
