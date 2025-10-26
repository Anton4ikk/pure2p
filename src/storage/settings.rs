//! Application settings and configuration

use crate::{Error, Result};
use serde::{Deserialize, Serialize};

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
