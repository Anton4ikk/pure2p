//! App initialization and state loading tests

use crate::tui::{App, Screen};
use crate::storage::Settings;
use super::helpers::{create_test_app, create_test_app_with_settings};
use tempfile::TempDir;

#[test]
fn test_app_initialization() {
    let (app, _temp_dir) = create_test_app();

    // Verify initial state
    assert_eq!(
        app.current_screen,
        Screen::MainMenu,
        "Should start on main menu (with consent already set)"
    );
    assert_eq!(app.selected_index, 0, "Should start with first item selected");
    assert!(!app.should_quit, "Should not be quitting initially");
    assert!(
        app.share_contact_screen.is_none(),
        "Share contact screen should not be active initially"
    );
    assert_eq!(
        app.menu_items.len(),
        6,
        "Should have 6 menu items"
    );
}

#[test]
fn test_app_state_initialized() {
    let (app, _temp_dir) = create_test_app();

    assert!(app.app_state.chats.is_empty(), "Should have no chats initially");
    assert!(app.app_state.contacts.is_empty(), "Should have no contacts initially");
}

#[test]
fn test_app_creates_state_on_first_run() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let state_path = temp_dir.path().join("app_state.json");

    // Verify file doesn't exist before test
    assert!(!state_path.exists(),
        "app_state.json should not exist before first run");

    // Create app (this should use default state)
    let app = App::new_with_settings(Some(&state_path))
        .expect("Failed to create app");

    // App state should be initialized with defaults
    assert_eq!(app.app_state.settings.retry_interval_minutes, 10,
        "Should have default retry interval");
    assert_eq!(app.app_state.settings.default_contact_expiry_days, 30,
        "Should have default contact expiry");
    assert_eq!(app.app_state.settings.max_message_retries, 5,
        "Should have default max retries");
    assert!(app.app_state.contacts.is_empty(), "Should have no contacts");
    assert!(app.app_state.chats.is_empty(), "Should have no chats");
}

#[test]
fn test_app_loads_existing_settings() {
    // Create custom settings
    let mut custom_settings = Settings::default();
    custom_settings.retry_interval_minutes = 25;
    custom_settings.global_retry_interval_ms = 25 * 60 * 1000;
    custom_settings.default_contact_expiry_days = 60;

    // Create app with custom settings
    let (app, _temp_dir) = create_test_app_with_settings(custom_settings);

    // Verify custom settings were loaded
    assert_eq!(app.app_state.settings.retry_interval_minutes, 25,
        "Should load custom retry interval");
    assert_eq!(app.app_state.settings.default_contact_expiry_days, 60,
        "Should load custom contact expiry");
}

#[test]
fn test_app_falls_back_to_defaults_on_corrupt_state() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let state_path = temp_dir.path().join("app_state.json");

    // Note: With SQLite storage, corrupt JSON files are ignored (migration is optional)
    // This test now verifies that creating a fresh app gives defaults
    // Create a corrupted JSON file (it will be ignored during migration)
    std::fs::write(&state_path, "{ invalid json }")
        .expect("Failed to write corrupt state");

    // Create app (uses in-memory SQLite for tests, ignores corrupt JSON)
    let app = App::new_with_settings(Some(&state_path))
        .expect("Failed to create app");

    // Verify default state is used (fresh database)
    assert_eq!(app.app_state.settings.retry_interval_minutes, 10,
        "Should use default retry interval");
    assert_eq!(app.app_state.settings.default_contact_expiry_days, 30,
        "Should use default contact expiry");
    assert!(app.app_state.contacts.is_empty(), "Should have no contacts");
    assert!(app.app_state.chats.is_empty(), "Should have no chats");
}

#[test]
fn test_app_connectivity_result_starts_as_none() {
    let (app, _temp_dir) = create_test_app();

    // Verify connectivity_result starts as None (diagnostics not run yet)
    assert!(app.connectivity_result.is_none(),
        "connectivity_result should be None on initialization");
    assert!(app.diagnostics_refresh_handle.is_none(),
        "diagnostics_refresh_handle should be None on initialization");
}
