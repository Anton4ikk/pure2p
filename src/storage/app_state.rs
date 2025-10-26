//! Application state persistence and management

use crate::{
    crypto::KeyPair,
    storage::{chat::Chat, contact::Contact, settings::Settings},
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
    /// Create a new empty application state
    pub fn new() -> Self {
        Self {
            user_keypair: None, // Will be generated on first run
            user_ip: None,      // Will be detected by connectivity diagnostics
            user_port: 8080,    // Default port
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
