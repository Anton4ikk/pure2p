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
    /// Whether there are pending (queued) messages for this contact
    pub has_pending_messages: bool,
}

impl Chat {
    /// Create a new chat with a contact
    pub fn new(contact_uid: String) -> Self {
        Self {
            contact_uid,
            messages: Vec::new(),
            is_active: false,
            has_pending_messages: false,
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

    /// Mark chat as having pending messages in the queue
    pub fn mark_has_pending(&mut self) {
        self.has_pending_messages = true;
    }

    /// Mark chat as having no pending messages
    pub fn mark_no_pending(&mut self) {
        self.has_pending_messages = false;
    }

    /// Check if this chat has pending messages
    pub fn has_pending(&self) -> bool {
        self.has_pending_messages
    }
}

/// Application settings
///
/// Persistent configuration for the Pure2P application.
/// Settings are stored in JSON format and can be loaded/saved from disk.
///
/// # Example
/// ```rust,no_run
/// use pure2p::storage::Settings;
///
/// // Load settings (returns default if file doesn't exist)
/// let mut settings = Settings::load("settings.json").expect("Failed to load");
///
/// // Update retry interval and auto-save
/// settings.update_retry_interval(15, "settings.json").expect("Failed to update");
///
/// // Access values
/// println!("Retry interval: {} minutes", settings.get_retry_interval_minutes());
/// println!("Storage path: {}", settings.storage_path);
/// ```
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
    /// Retry interval in minutes (user-facing, converts to/from global_retry_interval_ms)
    pub retry_interval_minutes: u32,
    /// Storage path for application data
    pub storage_path: String,
}

impl Settings {
    /// Load settings from a JSON file
    ///
    /// # Arguments
    /// * `path` - Path to the settings file
    ///
    /// # Returns
    /// The loaded settings, or default settings if file doesn't exist
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            // Return default settings if file doesn't exist
            return Ok(Self::default());
        }

        let data = std::fs::read_to_string(path)
            .map_err(|e| Error::Storage(format!("Failed to read settings: {}", e)))?;

        // Handle empty file (return defaults)
        if data.trim().is_empty() {
            return Ok(Self::default());
        }

        let mut settings: Self = serde_json::from_str(&data)
            .map_err(|e| Error::Storage(format!("Failed to parse settings: {}", e)))?;

        // Ensure milliseconds and minutes are in sync
        settings.sync_retry_interval();

        Ok(settings)
    }

    /// Save settings to a JSON file
    ///
    /// # Arguments
    /// * `path` - Path to save the settings file
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Storage(format!("Failed to create settings directory: {}", e)))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| Error::Storage(format!("Failed to serialize settings: {}", e)))?;

        std::fs::write(path, json)
            .map_err(|e| Error::Storage(format!("Failed to write settings: {}", e)))?;

        Ok(())
    }

    /// Update the retry interval in minutes and auto-save
    ///
    /// # Arguments
    /// * `minutes` - Retry interval in minutes
    /// * `save_path` - Path to save the updated settings
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn update_retry_interval<P: AsRef<std::path::Path>>(&mut self, minutes: u32, save_path: P) -> Result<()> {
        self.retry_interval_minutes = minutes;
        self.global_retry_interval_ms = (minutes as u64) * 60 * 1000; // Convert minutes to milliseconds
        self.save(save_path)
    }

    /// Update the global retry interval at runtime (in milliseconds)
    pub fn set_global_retry_interval_ms(&mut self, interval_ms: u64) {
        self.global_retry_interval_ms = interval_ms;
        self.retry_interval_minutes = (interval_ms / (60 * 1000)) as u32;
    }

    /// Get the global retry interval in milliseconds
    pub fn get_global_retry_interval_ms(&self) -> u64 {
        self.global_retry_interval_ms
    }

    /// Get the retry interval in minutes
    pub fn get_retry_interval_minutes(&self) -> u32 {
        self.retry_interval_minutes
    }

    /// Synchronize retry interval values (ensure minutes and milliseconds match)
    fn sync_retry_interval(&mut self) {
        // Prefer milliseconds value as source of truth
        self.retry_interval_minutes = (self.global_retry_interval_ms / (60 * 1000)) as u32;
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
            retry_interval_minutes: 10, // 10 minutes
            storage_path: "./data".to_string(), // Default storage path
        }
    }
}

/// Thread-safe settings manager for UI layer access
///
/// Provides shared access to application settings with automatic persistence.
/// Designed for use with TUI/GUI layers that need concurrent access.
///
/// # Example
/// ```rust,no_run
/// use pure2p::storage::SettingsManager;
/// use std::sync::Arc;
///
/// # async fn example() -> pure2p::Result<()> {
/// // Create shared settings manager
/// let manager = SettingsManager::new("settings.json").await?;
///
/// // Read settings
/// let retry_interval = manager.get_retry_interval_minutes().await;
/// println!("Retry interval: {} minutes", retry_interval);
///
/// // Update settings (auto-saves)
/// manager.set_retry_interval_minutes(15).await?;
///
/// // Share with UI thread
/// let ui_manager = manager.clone();
/// tokio::spawn(async move {
///     let path = ui_manager.get_storage_path().await;
///     println!("Storage: {}", path);
/// });
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SettingsManager {
    /// Shared settings state
    settings: std::sync::Arc<tokio::sync::RwLock<Settings>>,
    /// Path to settings file for auto-save
    settings_path: std::sync::Arc<String>,
}

impl SettingsManager {
    /// Create a new settings manager
    ///
    /// Loads settings from the specified path, or creates default settings if the file doesn't exist.
    ///
    /// # Arguments
    /// * `path` - Path to the settings file
    ///
    /// # Returns
    /// A new SettingsManager instance
    pub async fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let settings = Settings::load(&path)?;

        Ok(Self {
            settings: std::sync::Arc::new(tokio::sync::RwLock::new(settings)),
            settings_path: std::sync::Arc::new(path_str),
        })
    }

    /// Get the current retry interval in minutes
    pub async fn get_retry_interval_minutes(&self) -> u32 {
        let settings = self.settings.read().await;
        settings.retry_interval_minutes
    }

    /// Set the retry interval in minutes and auto-save
    ///
    /// # Arguments
    /// * `minutes` - Retry interval in minutes
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn set_retry_interval_minutes(&self, minutes: u32) -> Result<()> {
        let mut settings = self.settings.write().await;
        settings.update_retry_interval(minutes, self.settings_path.as_str())
    }

    /// Get the storage path
    pub async fn get_storage_path(&self) -> String {
        let settings = self.settings.read().await;
        settings.storage_path.clone()
    }

    /// Set the storage path and auto-save
    ///
    /// # Arguments
    /// * `path` - New storage path
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn set_storage_path(&self, path: String) -> Result<()> {
        let mut settings = self.settings.write().await;
        settings.storage_path = path;
        settings.save(self.settings_path.as_str())
    }

    /// Get the maximum message retry count
    pub async fn get_max_message_retries(&self) -> u32 {
        let settings = self.settings.read().await;
        settings.max_message_retries
    }

    /// Set the maximum message retry count and auto-save
    pub async fn set_max_message_retries(&self, retries: u32) -> Result<()> {
        let mut settings = self.settings.write().await;
        settings.max_message_retries = retries;
        settings.save(self.settings_path.as_str())
    }

    /// Get whether notifications are enabled
    pub async fn get_notifications_enabled(&self) -> bool {
        let settings = self.settings.read().await;
        settings.enable_notifications
    }

    /// Set whether notifications are enabled and auto-save
    pub async fn set_notifications_enabled(&self, enabled: bool) -> Result<()> {
        let mut settings = self.settings.write().await;
        settings.enable_notifications = enabled;
        settings.save(self.settings_path.as_str())
    }

    /// Get the default contact expiry in days
    pub async fn get_default_contact_expiry_days(&self) -> u32 {
        let settings = self.settings.read().await;
        settings.default_contact_expiry_days
    }

    /// Set the default contact expiry in days and auto-save
    pub async fn set_default_contact_expiry_days(&self, days: u32) -> Result<()> {
        let mut settings = self.settings.write().await;
        settings.default_contact_expiry_days = days;
        settings.save(self.settings_path.as_str())
    }

    /// Get a clone of all settings (for reading multiple values at once)
    pub async fn get_all(&self) -> Settings {
        let settings = self.settings.read().await;
        settings.clone()
    }

    /// Update multiple settings at once and auto-save
    ///
    /// # Arguments
    /// * `update_fn` - Function that modifies the settings
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn update<F>(&self, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut Settings),
    {
        let mut settings = self.settings.write().await;
        update_fn(&mut settings);
        settings.save(self.settings_path.as_str())
    }

    /// Reload settings from disk
    ///
    /// Useful for syncing with external changes to the settings file.
    pub async fn reload(&self) -> Result<()> {
        let loaded = Settings::load(self.settings_path.as_str())?;
        let mut settings = self.settings.write().await;
        *settings = loaded;
        Ok(())
    }

    /// Save current settings to disk
    pub async fn save(&self) -> Result<()> {
        let settings = self.settings.read().await;
        settings.save(self.settings_path.as_str())
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

    /// Update chat pending message status based on queued message UIDs
    ///
    /// This method synchronizes the `has_pending_messages` flag for each chat
    /// based on the list of UIDs that have pending messages in the queue.
    ///
    /// # Arguments
    /// * `pending_uids` - Set of contact UIDs that have messages in the queue
    ///
    /// # Example
    /// ```rust,no_run
    /// use pure2p::storage::AppState;
    /// use std::collections::HashSet;
    ///
    /// let mut state = AppState::new();
    /// let mut pending_uids = HashSet::new();
    /// pending_uids.insert("contact_uid_123".to_string());
    ///
    /// state.sync_pending_status(&pending_uids);
    /// ```
    pub fn sync_pending_status(&mut self, pending_uids: &std::collections::HashSet<String>) {
        for chat in &mut self.chats {
            if pending_uids.contains(&chat.contact_uid) {
                chat.mark_has_pending();
            } else {
                chat.mark_no_pending();
            }
        }
    }

    /// Get a mutable reference to a chat by contact UID
    ///
    /// # Arguments
    /// * `contact_uid` - The UID of the contact
    ///
    /// # Returns
    /// A mutable reference to the chat if found, None otherwise
    pub fn get_chat_mut(&mut self, contact_uid: &str) -> Option<&mut Chat> {
        self.chats.iter_mut().find(|c| c.contact_uid == contact_uid)
    }

    /// Get a reference to a chat by contact UID
    ///
    /// # Arguments
    /// * `contact_uid` - The UID of the contact
    ///
    /// # Returns
    /// A reference to the chat if found, None otherwise
    pub fn get_chat(&self, contact_uid: &str) -> Option<&Chat> {
        self.chats.iter().find(|c| c.contact_uid == contact_uid)
    }

    /// Add a new chat for a contact
    ///
    /// # Arguments
    /// * `contact_uid` - The UID of the contact
    ///
    /// # Returns
    /// A mutable reference to the newly created chat
    pub fn add_chat(&mut self, contact_uid: String) -> &mut Chat {
        let chat = Chat::new(contact_uid);
        self.chats.push(chat);
        self.chats.last_mut().unwrap()
    }

    /// Get or create a chat for a contact
    ///
    /// # Arguments
    /// * `contact_uid` - The UID of the contact
    ///
    /// # Returns
    /// A mutable reference to the chat (existing or newly created)
    pub fn get_or_create_chat(&mut self, contact_uid: &str) -> &mut Chat {
        if self.get_chat(contact_uid).is_none() {
            self.add_chat(contact_uid.to_string());
        }
        self.get_chat_mut(contact_uid).unwrap()
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
        assert_eq!(settings.retry_interval_minutes, 10);
        assert_eq!(settings.storage_path, "./data");
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

    #[test]
    fn test_chat_pending_messages_flag() {
        let mut chat = Chat::new("contact_uid".to_string());

        // Initially, no pending messages
        assert!(!chat.has_pending());
        assert!(!chat.has_pending_messages);

        // Mark as having pending messages
        chat.mark_has_pending();
        assert!(chat.has_pending());
        assert!(chat.has_pending_messages);

        // Mark as no pending messages
        chat.mark_no_pending();
        assert!(!chat.has_pending());
        assert!(!chat.has_pending_messages);
    }

    #[test]
    fn test_chat_pending_independent_of_active() {
        let mut chat = Chat::new("contact_uid".to_string());

        // Set both flags independently
        chat.mark_unread();
        chat.mark_has_pending();

        assert!(chat.is_active);
        assert!(chat.has_pending_messages);

        // Clear one flag
        chat.mark_read();

        assert!(!chat.is_active);
        assert!(chat.has_pending_messages); // Should remain true

        // Clear the other flag
        chat.mark_no_pending();

        assert!(!chat.is_active);
        assert!(!chat.has_pending_messages);
    }

    #[test]
    fn test_appstate_sync_pending_status() {
        use std::collections::HashSet;

        let mut state = AppState::new();

        // Add some chats
        state.add_chat("alice".to_string());
        state.add_chat("bob".to_string());
        state.add_chat("charlie".to_string());

        // Create pending UIDs set
        let mut pending_uids = HashSet::new();
        pending_uids.insert("alice".to_string());
        pending_uids.insert("charlie".to_string());

        // Sync pending status
        state.sync_pending_status(&pending_uids);

        // Verify flags
        assert!(state.get_chat("alice").unwrap().has_pending_messages);
        assert!(!state.get_chat("bob").unwrap().has_pending_messages);
        assert!(state.get_chat("charlie").unwrap().has_pending_messages);
    }

    #[test]
    fn test_appstate_sync_pending_status_empty() {
        use std::collections::HashSet;

        let mut state = AppState::new();
        state.add_chat("alice".to_string());
        state.add_chat("bob".to_string());

        // Mark all as having pending initially
        state.get_chat_mut("alice").unwrap().mark_has_pending();
        state.get_chat_mut("bob").unwrap().mark_has_pending();

        // Sync with empty set
        let pending_uids = HashSet::new();
        state.sync_pending_status(&pending_uids);

        // All should be cleared
        assert!(!state.get_chat("alice").unwrap().has_pending_messages);
        assert!(!state.get_chat("bob").unwrap().has_pending_messages);
    }

    #[test]
    fn test_appstate_get_or_create_chat() {
        let mut state = AppState::new();

        // Get or create should create new chat
        let chat = state.get_or_create_chat("new_contact");
        assert_eq!(chat.contact_uid, "new_contact");
        assert_eq!(state.chats.len(), 1);

        // Get or create should return existing chat
        let chat2 = state.get_or_create_chat("new_contact");
        assert_eq!(chat2.contact_uid, "new_contact");
        assert_eq!(state.chats.len(), 1); // Should not create duplicate
    }

    #[test]
    fn test_appstate_get_chat() {
        let mut state = AppState::new();
        state.add_chat("alice".to_string());

        // Get existing chat
        let chat = state.get_chat("alice");
        assert!(chat.is_some());
        assert_eq!(chat.unwrap().contact_uid, "alice");

        // Get non-existent chat
        let chat = state.get_chat("bob");
        assert!(chat.is_none());
    }

    #[test]
    fn test_appstate_get_chat_mut() {
        let mut state = AppState::new();
        state.add_chat("alice".to_string());

        // Get mutable reference and modify
        if let Some(chat) = state.get_chat_mut("alice") {
            chat.mark_has_pending();
            chat.mark_unread();
        }

        // Verify changes persisted
        let chat = state.get_chat("alice").unwrap();
        assert!(chat.has_pending_messages);
        assert!(chat.is_active);
    }

    #[test]
    fn test_chat_serialization_with_pending_flag() {
        let mut chat = Chat::new("contact_123".to_string());
        chat.mark_has_pending();
        chat.mark_unread();

        // Add a message
        let msg = Message {
            id: "msg_1".to_string(),
            sender: "sender".to_string(),
            recipient: "contact_123".to_string(),
            content: vec![1, 2, 3],
            timestamp: 1000,
            delivered: false,
        };
        chat.append_message(msg);

        // Serialize to JSON
        let json = serde_json::to_string(&chat).expect("Failed to serialize");

        // Deserialize
        let loaded: Chat = serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify all fields including pending flag
        assert_eq!(loaded.contact_uid, "contact_123");
        assert!(loaded.is_active);
        assert!(loaded.has_pending_messages);
        assert_eq!(loaded.messages.len(), 1);
    }

    #[test]
    fn test_settings_save_and_load() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        // Create and save settings
        let mut settings = Settings::default();
        settings.retry_interval_minutes = 15;
        settings.global_retry_interval_ms = 15 * 60 * 1000; // Keep in sync
        settings.storage_path = "/custom/path".to_string();

        settings.save(path).expect("Failed to save settings");

        // Load settings
        let loaded = Settings::load(path).expect("Failed to load settings");

        assert_eq!(loaded.retry_interval_minutes, 15);
        assert_eq!(loaded.global_retry_interval_ms, 15 * 60 * 1000);
        assert_eq!(loaded.storage_path, "/custom/path");
        assert_eq!(loaded.default_contact_expiry_days, 30);
    }

    #[test]
    fn test_settings_load_nonexistent() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let path = temp_dir.path().join("nonexistent.json");

        // Load from nonexistent file should return defaults
        let settings = Settings::load(&path).expect("Failed to load settings");

        assert_eq!(settings.retry_interval_minutes, 10);
        assert_eq!(settings.storage_path, "./data");
    }

    #[test]
    fn test_settings_load_empty_file() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        // File exists but is empty - should return defaults
        let settings = Settings::load(path).expect("Failed to load settings");

        assert_eq!(settings.retry_interval_minutes, 10);
        assert_eq!(settings.storage_path, "./data");
        assert_eq!(settings.max_message_retries, 5);
    }

    #[test]
    fn test_settings_update_retry_interval() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        // Create settings and update retry interval
        let mut settings = Settings::default();
        settings.update_retry_interval(20, path).expect("Failed to update");

        // Verify values are updated
        assert_eq!(settings.retry_interval_minutes, 20);
        assert_eq!(settings.global_retry_interval_ms, 20 * 60 * 1000);

        // Verify auto-save worked
        let loaded = Settings::load(path).expect("Failed to load");
        assert_eq!(loaded.retry_interval_minutes, 20);
        assert_eq!(loaded.global_retry_interval_ms, 20 * 60 * 1000);
    }

    #[test]
    fn test_settings_sync_retry_interval() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        // Create settings with mismatched values (shouldn't happen in practice)
        let mut settings = Settings::default();
        settings.global_retry_interval_ms = 900_000; // 15 minutes
        settings.retry_interval_minutes = 10; // Wrong value

        // Save and reload should sync the values
        settings.save(path).expect("Failed to save");
        let loaded = Settings::load(path).expect("Failed to load");

        // Minutes should be synced to match milliseconds
        assert_eq!(loaded.retry_interval_minutes, 15);
        assert_eq!(loaded.global_retry_interval_ms, 900_000);
    }

    #[test]
    fn test_settings_set_global_retry_interval_ms() {
        let mut settings = Settings::default();

        // Set milliseconds directly
        settings.set_global_retry_interval_ms(1_800_000); // 30 minutes

        // Both values should be updated
        assert_eq!(settings.global_retry_interval_ms, 1_800_000);
        assert_eq!(settings.retry_interval_minutes, 30);
    }

    #[test]
    fn test_settings_get_retry_intervals() {
        let settings = Settings::default();

        assert_eq!(settings.get_retry_interval_minutes(), 10);
        assert_eq!(settings.get_global_retry_interval_ms(), 600_000);
    }

    #[test]
    fn test_settings_json_format() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let settings = Settings::default();
        settings.save(path).expect("Failed to save");

        // Read the JSON file
        let json = std::fs::read_to_string(path).expect("Failed to read file");

        // Verify JSON contains expected fields
        assert!(json.contains("retry_interval_minutes"));
        assert!(json.contains("storage_path"));
        assert!(json.contains("global_retry_interval_ms"));
        assert!(json.contains("\"./data\"")); // storage_path default

        // Verify the JSON can be deserialized
        let parsed: Settings = serde_json::from_str(&json).expect("Failed to parse JSON");
        assert_eq!(parsed.retry_interval_minutes, 10);
    }

    #[test]
    fn test_settings_create_parent_directory() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let path = temp_dir.path().join("subdir").join("settings.json");

        // Parent directory doesn't exist yet
        assert!(!path.parent().unwrap().exists());

        let settings = Settings::default();
        settings.save(&path).expect("Failed to save");

        // Parent directory should be created
        assert!(path.parent().unwrap().exists());
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_settings_manager_new() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        // Create manager
        let manager = SettingsManager::new(path).await.expect("Failed to create manager");

        // Should have default values
        assert_eq!(manager.get_retry_interval_minutes().await, 10);
        assert_eq!(manager.get_storage_path().await, "./data");
    }

    #[tokio::test]
    async fn test_settings_manager_set_retry_interval() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let manager = SettingsManager::new(path).await.expect("Failed to create manager");

        // Update retry interval
        manager.set_retry_interval_minutes(20).await.expect("Failed to set");

        // Verify updated
        assert_eq!(manager.get_retry_interval_minutes().await, 20);

        // Verify persisted
        let loaded = Settings::load(path).expect("Failed to load");
        assert_eq!(loaded.retry_interval_minutes, 20);
        assert_eq!(loaded.global_retry_interval_ms, 20 * 60 * 1000);
    }

    #[tokio::test]
    async fn test_settings_manager_set_storage_path() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let manager = SettingsManager::new(path).await.expect("Failed to create manager");

        // Update storage path
        manager.set_storage_path("/custom/storage".to_string()).await.expect("Failed to set");

        // Verify updated
        assert_eq!(manager.get_storage_path().await, "/custom/storage");

        // Verify persisted
        let loaded = Settings::load(path).expect("Failed to load");
        assert_eq!(loaded.storage_path, "/custom/storage");
    }

    #[tokio::test]
    async fn test_settings_manager_notifications() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let manager = SettingsManager::new(path).await.expect("Failed to create manager");

        // Default should be enabled
        assert!(manager.get_notifications_enabled().await);

        // Disable
        manager.set_notifications_enabled(false).await.expect("Failed to set");
        assert!(!manager.get_notifications_enabled().await);

        // Enable
        manager.set_notifications_enabled(true).await.expect("Failed to set");
        assert!(manager.get_notifications_enabled().await);
    }

    #[tokio::test]
    async fn test_settings_manager_max_retries() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let manager = SettingsManager::new(path).await.expect("Failed to create manager");

        // Update max retries
        manager.set_max_message_retries(10).await.expect("Failed to set");

        // Verify
        assert_eq!(manager.get_max_message_retries().await, 10);
    }

    #[tokio::test]
    async fn test_settings_manager_contact_expiry() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let manager = SettingsManager::new(path).await.expect("Failed to create manager");

        // Update contact expiry
        manager.set_default_contact_expiry_days(60).await.expect("Failed to set");

        // Verify
        assert_eq!(manager.get_default_contact_expiry_days().await, 60);
    }

    #[tokio::test]
    async fn test_settings_manager_get_all() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let manager = SettingsManager::new(path).await.expect("Failed to create manager");

        // Get all settings
        let settings = manager.get_all().await;

        assert_eq!(settings.retry_interval_minutes, 10);
        assert_eq!(settings.storage_path, "./data");
        assert_eq!(settings.max_message_retries, 5);
    }

    #[tokio::test]
    async fn test_settings_manager_update_multiple() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let manager = SettingsManager::new(path).await.expect("Failed to create manager");

        // Update multiple settings at once
        manager.update(|s| {
            s.retry_interval_minutes = 25;
            s.global_retry_interval_ms = 25 * 60 * 1000;
            s.storage_path = "/new/path".to_string();
            s.max_message_retries = 8;
        }).await.expect("Failed to update");

        // Verify all updated
        assert_eq!(manager.get_retry_interval_minutes().await, 25);
        assert_eq!(manager.get_storage_path().await, "/new/path");
        assert_eq!(manager.get_max_message_retries().await, 8);

        // Verify persisted
        let loaded = Settings::load(path).expect("Failed to load");
        assert_eq!(loaded.retry_interval_minutes, 25);
        assert_eq!(loaded.storage_path, "/new/path");
        assert_eq!(loaded.max_message_retries, 8);
    }

    #[tokio::test]
    async fn test_settings_manager_reload() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let manager = SettingsManager::new(path).await.expect("Failed to create manager");

        // Update via manager
        manager.set_retry_interval_minutes(15).await.expect("Failed to set");

        // Modify file directly
        let mut settings = Settings::load(path).expect("Failed to load");
        settings.retry_interval_minutes = 30;
        settings.global_retry_interval_ms = 30 * 60 * 1000;
        settings.save(path).expect("Failed to save");

        // Manager still has old value
        assert_eq!(manager.get_retry_interval_minutes().await, 15);

        // Reload from disk
        manager.reload().await.expect("Failed to reload");

        // Now has new value
        assert_eq!(manager.get_retry_interval_minutes().await, 30);
    }

    #[tokio::test]
    async fn test_settings_manager_concurrent_access() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let manager = SettingsManager::new(path).await.expect("Failed to create manager");

        // Clone for concurrent access
        let manager1 = manager.clone();
        let manager2 = manager.clone();
        let manager3 = manager.clone();

        // Spawn concurrent tasks
        let task1 = tokio::spawn(async move {
            manager1.set_retry_interval_minutes(15).await
        });

        let task2 = tokio::spawn(async move {
            manager2.set_storage_path("/path1".to_string()).await
        });

        let task3 = tokio::spawn(async move {
            manager3.set_notifications_enabled(false).await
        });

        // Wait for all tasks
        task1.await.unwrap().expect("Task 1 failed");
        task2.await.unwrap().expect("Task 2 failed");
        task3.await.unwrap().expect("Task 3 failed");

        // Verify all changes applied
        assert_eq!(manager.get_retry_interval_minutes().await, 15);
        assert_eq!(manager.get_storage_path().await, "/path1");
        assert!(!manager.get_notifications_enabled().await);
    }

    #[tokio::test]
    async fn test_settings_manager_clone() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        let manager = SettingsManager::new(path).await.expect("Failed to create manager");
        let cloned = manager.clone();

        // Update via original
        manager.set_retry_interval_minutes(20).await.expect("Failed to set");

        // Clone sees the update (shared state)
        assert_eq!(cloned.get_retry_interval_minutes().await, 20);

        // Update via clone
        cloned.set_storage_path("/clone/path".to_string()).await.expect("Failed to set");

        // Original sees the update
        assert_eq!(manager.get_storage_path().await, "/clone/path");
    }
}
