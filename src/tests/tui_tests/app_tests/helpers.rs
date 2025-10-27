//! Shared test helpers for app tests

use crate::tui::App;
use crate::storage::{AppState, Settings};
use tempfile::TempDir;

/// Helper to create an App with temporary settings file
/// Returns (App, TempDir) - the TempDir must be kept alive for the test duration
pub fn create_test_app() -> (App, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");
    let app = App::new_with_settings(Some(&settings_path))
        .expect("Failed to create app");
    (app, temp_dir)
}

/// Helper to create an App with custom settings
pub fn create_test_app_with_settings(settings: Settings) -> (App, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let state_path = temp_dir.path().join("app_state.json");

    // Create app state with custom settings
    let mut app_state = AppState::new();
    app_state.settings = settings;

    // Since we're using in-memory storage for tests, we need to:
    // 1. Create the app first (which creates in-memory storage)
    // 2. Then update its settings and save
    let mut app = App::new_with_settings(Some(&state_path))
        .expect("Failed to create app");

    // Update the app's settings
    app.app_state.settings = app_state.settings;
    app.save_state().expect("Failed to save app state");

    (app, temp_dir)
}
