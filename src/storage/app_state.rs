//! Application state persistence and management

use crate::{
    crypto::KeyPair,
    storage::{chat::Chat, contact::Contact, settings::Settings, storage_db::Storage},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Persistent application state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    /// User's cryptographic identity (keypair + UID)
    pub user_keypair: Option<KeyPair>,
    /// User's detected external IP address
    pub user_ip: Option<String>,
    /// User's listening port
    pub user_port: u16,
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
    /// Generate a random port in the dynamic/private port range (49152-65535)
    ///
    /// This range is officially designated by IANA for dynamic/private use and
    /// is typically not blocked by ISPs. The large range (16,384 ports) ensures
    /// that multiple devices on the same network will likely get different ports.
    pub fn generate_random_port() -> u16 {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        // IANA dynamic/private port range: 49152-65535
        rng.gen_range(49152..=65535)
    }

    /// Create a new empty application state
    pub fn new() -> Self {
        Self {
            user_keypair: None, // Will be generated on first run
            user_ip: None,      // Will be detected by connectivity diagnostics
            user_port: Self::generate_random_port(), // Random port for P2P
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

    // ========== SQLite-based storage methods ==========

    /// Save the entire application state to SQLite database
    ///
    /// # Errors
    /// Returns an error if database operations fail
    pub fn save_to_db(&self, db: &Storage) -> Result<()> {
        // Save user identity
        if let Some(ref keypair) = self.user_keypair {
            db.save_user_identity(
                keypair,
                self.user_ip.as_deref(),
                self.user_port,
            )?;
        }

        // Save all contacts
        for contact in &self.contacts {
            db.save_contact(contact)?;
        }

        // Save all chats (which includes messages)
        for chat in &self.chats {
            db.save_chat(chat)?;
        }

        // Save settings
        db.save_settings(&self.settings)?;

        Ok(())
    }

    /// Load the entire application state from SQLite database
    ///
    /// # Returns
    /// A loaded `AppState` or a new empty state if database is empty
    ///
    /// # Errors
    /// Returns an error if database operations fail
    pub fn load_from_db(db: &Storage) -> Result<Self> {
        // Load user identity
        let (user_keypair, user_ip, user_port) = if let Some((keypair, ip, port)) = db.load_user_identity()? {
            (Some(keypair), ip, port)
        } else {
            // No user identity yet, return new state
            return Ok(Self::new());
        };

        // Load contacts
        let contacts = db.load_contacts()?;

        // Load chats
        let chats = db.load_chats()?;

        // Load settings (or use defaults)
        let settings = db.load_settings()?.unwrap_or_default();

        Ok(Self {
            user_keypair,
            user_ip,
            user_port,
            contacts,
            chats,
            message_queue: Vec::new(), // Queue is managed separately in message_queue.db
            settings,
        })
    }

    /// Migrate from JSON file to SQLite database
    ///
    /// # Arguments
    /// * `json_path` - Path to the old app_state.json file
    /// * `db` - SQLite storage instance
    ///
    /// # Returns
    /// True if migration was performed, false if no JSON file exists or is corrupt
    ///
    /// # Errors
    /// Returns an error if migration fails (but not if JSON is corrupt)
    pub fn migrate_from_json<P: AsRef<Path>>(json_path: P, db: &Storage) -> Result<bool> {
        let path_ref = json_path.as_ref();

        // Check if JSON file exists
        if !path_ref.exists() {
            return Ok(false);
        }

        // Try to load state from JSON (may fail if corrupt)
        let state = match Self::load(path_ref) {
            Ok(s) => s,
            Err(_) => {
                // JSON is corrupt, skip migration and remove bad file
                tracing::warn!("Corrupt app_state.json found, skipping migration");
                let backup_path = path_ref.with_extension("json.corrupt");
                let _ = std::fs::rename(path_ref, &backup_path);
                return Ok(false);
            }
        };

        // Save to database
        state.save_to_db(db)?;

        // Rename the old JSON file to .bak
        let backup_path = path_ref.with_extension("json.bak");
        std::fs::rename(path_ref, &backup_path)
            .map_err(|e| Error::Storage(format!("Failed to backup old state file: {}", e)))?;

        Ok(true)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
