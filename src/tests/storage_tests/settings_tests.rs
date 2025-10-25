// Settings Tests - Testing Settings and SettingsManager

use crate::storage::{Settings, SettingsManager};
use tempfile::NamedTempFile;

// Settings Tests

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
fn test_settings_save_and_load() {
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

// SettingsManager Tests

#[tokio::test]
async fn test_settings_manager_new() {
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
