//! Local storage module
//!
//! This module handles persistent storage including:
//! - Message history
//! - Peer information
//! - User data
//! - Configuration
//!
//! The module is organized into submodules for better maintainability:
//! - `contact` - Contact/peer management and token generation/verification
//! - `message` - Message structures and delivery status
//! - `chat` - Chat conversation management
//! - `settings` - Application settings and configuration
//! - `settings_manager` - Thread-safe settings management
//! - `app_state` - Persistent application state
//! - `storage_db` - Low-level SQLite database (unimplemented)

// Submodules
pub mod app_state;
pub mod chat;
pub mod contact;
pub mod message;
pub mod settings;
pub mod settings_manager;
pub mod storage_db;

// Re-export commonly used types
pub use app_state::AppState;
pub use chat::Chat;
pub use contact::Contact;
pub use message::{DeliveryStatus, Message};
pub use settings::Settings;
pub use settings_manager::SettingsManager;
pub use storage_db::Storage;

// Re-export main functions
pub use contact::{generate_contact_token, parse_contact_token};
