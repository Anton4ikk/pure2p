//! Thread-safe settings manager for concurrent access

use crate::{storage::settings::{MappingConsent, Settings}, Result};

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

    /// Get mapping consent status
    pub async fn get_mapping_consent(&self) -> MappingConsent {
        let settings = self.settings.read().await;
        settings.mapping_consent
    }

    /// Set mapping consent and auto-save
    pub async fn set_mapping_consent(&self, consent: MappingConsent) -> Result<()> {
        let mut settings = self.settings.write().await;
        settings.update_mapping_consent(consent, self.settings_path.as_str())
    }

    /// Check if mapping is allowed
    pub async fn is_mapping_allowed(&self) -> bool {
        let settings = self.settings.read().await;
        settings.is_mapping_allowed()
    }

    /// Check if consent dialog should be shown
    pub async fn should_ask_consent(&self) -> bool {
        let settings = self.settings.read().await;
        settings.should_ask_consent()
    }
}
