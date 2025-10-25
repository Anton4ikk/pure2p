// SettingsScreen Tests - Testing settings configuration screen

use crate::storage::Settings;
use crate::tui::screens::SettingsScreen;
use tempfile::NamedTempFile;

#[test]
fn test_settings_screen_creation() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_string_lossy().to_string();

    let screen = SettingsScreen::new(path.clone());

    // Should load default settings
    assert_eq!(screen.retry_interval_input, "10"); // Default is 10 minutes
    assert_eq!(screen.selected_field, 0);
    assert!(screen.status_message.is_some());
    assert!(!screen.is_error);
    assert_eq!(screen.settings_path, path);
}

#[test]
fn test_settings_screen_add_char() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_string_lossy().to_string();

    let mut screen = SettingsScreen::new(path);
    screen.clear_input();

    // Should accept digits
    screen.add_char('1');
    assert_eq!(screen.retry_interval_input, "1");

    screen.add_char('5');
    assert_eq!(screen.retry_interval_input, "15");

    // Should reject non-digits
    screen.add_char('a');
    assert_eq!(screen.retry_interval_input, "15");

    screen.add_char('!');
    assert_eq!(screen.retry_interval_input, "15");
}

#[test]
fn test_settings_screen_backspace() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_string_lossy().to_string();

    let mut screen = SettingsScreen::new(path);
    screen.retry_interval_input = "123".to_string();

    screen.backspace();
    assert_eq!(screen.retry_interval_input, "12");

    screen.backspace();
    assert_eq!(screen.retry_interval_input, "1");

    screen.backspace();
    assert_eq!(screen.retry_interval_input, "");

    // Backspace on empty should not panic
    screen.backspace();
    assert_eq!(screen.retry_interval_input, "");
}

#[test]
fn test_settings_screen_clear_input() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_string_lossy().to_string();

    let mut screen = SettingsScreen::new(path);
    screen.retry_interval_input = "123".to_string();

    screen.clear_input();
    assert_eq!(screen.retry_interval_input, "");
}

#[test]
fn test_settings_screen_validate_empty() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_string_lossy().to_string();

    let mut screen = SettingsScreen::new(path);
    screen.clear_input();

    let result = screen.validate_and_save();

    assert!(!result);
    assert!(screen.is_error);
    assert!(screen.status_message.as_ref().unwrap().contains("empty"));
}

#[test]
fn test_settings_screen_validate_zero() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_string_lossy().to_string();

    let mut screen = SettingsScreen::new(path);
    screen.retry_interval_input = "0".to_string();

    let result = screen.validate_and_save();

    assert!(!result);
    assert!(screen.is_error);
    assert!(screen.status_message.as_ref().unwrap().contains("at least 1"));
}

#[test]
fn test_settings_screen_validate_too_large() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_string_lossy().to_string();

    let mut screen = SettingsScreen::new(path);
    screen.retry_interval_input = "2000".to_string();

    let result = screen.validate_and_save();

    assert!(!result);
    assert!(screen.is_error);
    assert!(screen.status_message.as_ref().unwrap().contains("1440"));
}

#[test]
fn test_settings_screen_validate_valid() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_string_lossy().to_string();

    let mut screen = SettingsScreen::new(path);
    screen.retry_interval_input = "30".to_string();

    let result = screen.validate_and_save();

    assert!(result);
    assert!(!screen.is_error);
    assert!(screen.status_message.as_ref().unwrap().contains("✓"));
    assert!(screen.status_message.as_ref().unwrap().contains("30"));
}

#[test]
fn test_settings_screen_save_persists() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_string_lossy().to_string();

    let mut screen = SettingsScreen::new(path.clone());
    screen.retry_interval_input = "45".to_string();

    let result = screen.validate_and_save();
    assert!(result);

    // Load settings from file to verify
    let settings = Settings::load(&path).expect("Failed to load settings");
    assert_eq!(settings.retry_interval_minutes, 45);
    assert_eq!(settings.global_retry_interval_ms, 45 * 60 * 1000);
}
