//! SQLite-based storage backend
//!
//! This module provides a complete SQLite implementation for persistent storage
//! of all application data: keypairs, contacts, chats, messages, and settings.

use crate::{
    crypto::KeyPair,
    storage::{chat::Chat, contact::Contact, message::Message, settings::Settings},
    Error, Result,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

/// SQLite-based storage manager
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Create a new storage instance with a database file
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| Error::Storage(format!("Failed to open database: {}", e)))?;

        let mut storage = Self { conn };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Create an in-memory storage instance (for testing)
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| Error::Storage(format!("Failed to create in-memory database: {}", e)))?;

        let mut storage = Self { conn };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Initialize database schema
    fn init_schema(&mut self) -> Result<()> {
        // User identity table (single row)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS user_identity (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                public_key BLOB NOT NULL,
                private_key BLOB NOT NULL,
                x25519_public BLOB NOT NULL,
                x25519_secret BLOB NOT NULL,
                uid TEXT NOT NULL,
                ip TEXT,
                port INTEGER NOT NULL
            )",
            [],
        )?;

        // Contacts table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS contacts (
                uid TEXT PRIMARY KEY,
                ip TEXT NOT NULL,
                pubkey BLOB NOT NULL,
                x25519_pubkey BLOB NOT NULL,
                expiry INTEGER NOT NULL,
                is_active INTEGER NOT NULL
            )",
            [],
        )?;

        // Chats table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS chats (
                contact_uid TEXT PRIMARY KEY,
                is_active INTEGER NOT NULL,
                has_pending_messages INTEGER NOT NULL,
                FOREIGN KEY (contact_uid) REFERENCES contacts(uid) ON DELETE CASCADE
            )",
            [],
        )?;

        // Messages table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                sender TEXT NOT NULL,
                receiver TEXT NOT NULL,
                content BLOB NOT NULL,
                timestamp INTEGER NOT NULL,
                chat_uid TEXT NOT NULL,
                FOREIGN KEY (chat_uid) REFERENCES chats(contact_uid) ON DELETE CASCADE
            )",
            [],
        )?;

        // Settings table (single row)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                default_contact_expiry_days INTEGER NOT NULL,
                auto_accept_contacts INTEGER NOT NULL,
                max_message_retries INTEGER NOT NULL,
                retry_base_delay_ms INTEGER NOT NULL,
                enable_notifications INTEGER NOT NULL,
                global_retry_interval_ms INTEGER NOT NULL,
                retry_interval_minutes INTEGER NOT NULL,
                storage_path TEXT NOT NULL
            )",
            [],
        )?;

        // Create indexes for better query performance
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_chat ON messages(chat_uid)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp)",
            [],
        )?;

        Ok(())
    }

    // ========== User Identity ==========

    /// Save user identity (keypair, IP, port)
    pub fn save_user_identity(&self, keypair: &KeyPair, ip: Option<&str>, port: u16) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO user_identity (id, public_key, private_key, x25519_public, x25519_secret, uid, ip, port)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &keypair.public_key,
                &keypair.private_key,
                &keypair.x25519_public,
                &keypair.x25519_secret,
                &keypair.uid.to_string(),
                ip,
                port,
            ],
        )?;
        Ok(())
    }

    /// Load user identity
    pub fn load_user_identity(&self) -> Result<Option<(KeyPair, Option<String>, u16)>> {
        let result = self.conn.query_row(
            "SELECT public_key, private_key, x25519_public, x25519_secret, uid, ip, port FROM user_identity WHERE id = 1",
            [],
            |row| {
                let public_key: Vec<u8> = row.get(0)?;
                let private_key: Vec<u8> = row.get(1)?;
                let x25519_public: Vec<u8> = row.get(2)?;
                let x25519_secret: Vec<u8> = row.get(3)?;
                let _uid: String = row.get(4)?; // Stored for reference, but UID is derived from public_key
                let ip: Option<String> = row.get(5)?;
                let port: u16 = row.get(6)?;

                let keypair = KeyPair {
                    public_key: public_key.clone(),
                    private_key: private_key.clone(),
                    x25519_public: x25519_public.clone(),
                    x25519_secret: x25519_secret.clone(),
                    uid: crate::crypto::UID::from_public_key(&public_key),
                };

                Ok((keypair, ip, port))
            },
        ).optional()?;

        Ok(result)
    }

    // ========== Contacts ==========

    /// Save or update a contact
    pub fn save_contact(&self, contact: &Contact) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO contacts (uid, ip, pubkey, x25519_pubkey, expiry, is_active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &contact.uid,
                &contact.ip,
                &contact.pubkey,
                &contact.x25519_pubkey,
                contact.expiry.timestamp(),
                contact.is_active as i32,
            ],
        )?;
        Ok(())
    }

    /// Load all contacts
    pub fn load_contacts(&self) -> Result<Vec<Contact>> {
        let mut stmt = self.conn.prepare(
            "SELECT uid, ip, pubkey, x25519_pubkey, expiry, is_active FROM contacts"
        )?;

        let contacts = stmt.query_map([], |row| {
            let uid: String = row.get(0)?;
            let ip: String = row.get(1)?;
            let pubkey: Vec<u8> = row.get(2)?;
            let x25519_pubkey: Vec<u8> = row.get(3)?;
            let expiry_timestamp: i64 = row.get(4)?;
            let is_active: i32 = row.get(5)?;

            let expiry = chrono::DateTime::from_timestamp(expiry_timestamp, 0)
                .unwrap_or_else(chrono::Utc::now);

            Ok(Contact {
                uid,
                ip,
                pubkey,
                x25519_pubkey,
                expiry,
                is_active: is_active != 0,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(contacts)
    }

    /// Delete a contact
    pub fn delete_contact(&self, uid: &str) -> Result<()> {
        self.conn.execute("DELETE FROM contacts WHERE uid = ?1", params![uid])?;
        Ok(())
    }

    // ========== Chats ==========

    /// Save or update a chat
    pub fn save_chat(&self, chat: &Chat) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO chats (contact_uid, is_active, has_pending_messages)
             VALUES (?1, ?2, ?3)",
            params![
                &chat.contact_uid,
                chat.is_active as i32,
                chat.has_pending_messages as i32,
            ],
        )?;

        // Save all messages in the chat
        for message in &chat.messages {
            self.save_message(message, &chat.contact_uid)?;
        }

        Ok(())
    }

    /// Load all chats with their messages
    pub fn load_chats(&self) -> Result<Vec<Chat>> {
        let mut stmt = self.conn.prepare(
            "SELECT contact_uid, is_active, has_pending_messages FROM chats"
        )?;

        let mut chats = Vec::new();

        for row in stmt.query_map([], |row| {
            let contact_uid: String = row.get(0)?;
            let is_active: i32 = row.get(1)?;
            let has_pending_messages: i32 = row.get(2)?;

            Ok((contact_uid, is_active != 0, has_pending_messages != 0))
        })? {
            let (contact_uid, is_active, has_pending_messages) = row?;
            let messages = self.load_messages_for_chat(&contact_uid)?;

            chats.push(Chat {
                contact_uid,
                messages,
                is_active,
                has_pending_messages,
            });
        }

        Ok(chats)
    }

    /// Delete a chat and all its messages
    pub fn delete_chat(&self, contact_uid: &str) -> Result<()> {
        self.conn.execute("DELETE FROM chats WHERE contact_uid = ?1", params![contact_uid])?;
        // Messages will be deleted automatically due to CASCADE
        Ok(())
    }

    // ========== Messages ==========

    /// Save a message
    fn save_message(&self, message: &Message, chat_uid: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO messages (id, sender, receiver, content, timestamp, chat_uid)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &message.id,
                &message.sender,
                &message.recipient,
                &message.content,
                message.timestamp,
                chat_uid,
            ],
        )?;
        Ok(())
    }

    /// Load all messages for a specific chat
    fn load_messages_for_chat(&self, chat_uid: &str) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sender, receiver, content, timestamp FROM messages
             WHERE chat_uid = ?1 ORDER BY timestamp ASC"
        )?;

        let messages = stmt.query_map(params![chat_uid], |row| {
            Ok(Message {
                id: row.get(0)?,
                sender: row.get(1)?,
                recipient: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
                delivered: false,
                delivery_status: crate::storage::message::DeliveryStatus::Sent,
                next_retry_at: None,
                attempts: 0,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    // ========== Settings ==========

    /// Save settings
    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (
                id, default_contact_expiry_days, auto_accept_contacts,
                max_message_retries, retry_base_delay_ms, enable_notifications,
                global_retry_interval_ms, retry_interval_minutes, storage_path
            ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                settings.default_contact_expiry_days,
                settings.auto_accept_contacts as i32,
                settings.max_message_retries,
                settings.retry_base_delay_ms as i64,
                settings.enable_notifications as i32,
                settings.global_retry_interval_ms as i64,
                settings.retry_interval_minutes,
                &settings.storage_path,
            ],
        )?;
        Ok(())
    }

    /// Load settings
    pub fn load_settings(&self) -> Result<Option<Settings>> {
        let result = self.conn.query_row(
            "SELECT default_contact_expiry_days, auto_accept_contacts, max_message_retries,
                    retry_base_delay_ms, enable_notifications, global_retry_interval_ms,
                    retry_interval_minutes, storage_path
             FROM settings WHERE id = 1",
            [],
            |row| {
                Ok(Settings {
                    default_contact_expiry_days: row.get(0)?,
                    auto_accept_contacts: row.get::<_, i32>(1)? != 0,
                    max_message_retries: row.get(2)?,
                    retry_base_delay_ms: row.get::<_, i64>(3)? as u64,
                    enable_notifications: row.get::<_, i32>(4)? != 0,
                    global_retry_interval_ms: row.get::<_, i64>(5)? as u64,
                    retry_interval_minutes: row.get(6)?,
                    storage_path: row.get(7)?,
                })
            },
        ).optional()?;

        Ok(result)
    }

    // ========== Utility ==========

    /// Clear all data (for testing)
    pub fn clear_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM messages", [])?;
        self.conn.execute("DELETE FROM chats", [])?;
        self.conn.execute("DELETE FROM contacts", [])?;
        self.conn.execute("DELETE FROM user_identity", [])?;
        self.conn.execute("DELETE FROM settings", [])?;
        Ok(())
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new_with_default_path().expect("Failed to create default storage")
    }
}

impl Storage {
    /// Create storage with default path (./app_data/pure2p.db)
    pub fn new_with_default_path() -> Result<Self> {
        // Create app_data directory if it doesn't exist
        let app_data_dir = std::path::Path::new("./app_data");
        if !app_data_dir.exists() {
            std::fs::create_dir_all(app_data_dir)
                .map_err(|e| Error::Storage(format!("Failed to create app_data directory: {}", e)))?;
        }

        let db_path = app_data_dir.join("pure2p.db");
        Self::new(db_path)
    }
}
