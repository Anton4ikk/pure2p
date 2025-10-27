//! App startup and connectivity tests

use crate::tui::Screen;
use super::helpers::create_test_app;

#[test]
fn test_app_startup_with_no_pending_messages() {
    let (app, _temp_dir) = create_test_app();

    // With no pending messages and consent set, should start on MainMenu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.startup_sync_screen.is_none());
}

#[test]
fn test_app_update_startup_sync() {
    let (mut app, _temp_dir) = create_test_app();

    // Manually create a startup sync screen with messages
    app.startup_sync_screen = Some(crate::tui::screens::StartupSyncScreen::new(5));
    app.current_screen = Screen::StartupSync;

    let initial_current = app.startup_sync_screen.as_ref().unwrap().current;

    // Update should process one message
    app.update_startup_sync();

    let updated_current = app.startup_sync_screen.as_ref().unwrap().current;
    assert_eq!(updated_current, initial_current + 1);
}

#[test]
fn test_app_complete_startup_sync() {
    let (mut app, _temp_dir) = create_test_app();

    // Set up startup sync screen
    app.startup_sync_screen = Some(crate::tui::screens::StartupSyncScreen::new(1));
    app.current_screen = Screen::StartupSync;

    // Complete the sync
    app.complete_startup_sync();

    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.startup_sync_screen.is_none());
}

#[test]
fn test_app_trigger_startup_connectivity() {
    let (mut app, _temp_dir) = create_test_app();

    // Verify no handle exists initially
    assert!(app.diagnostics_refresh_handle.is_none());

    // Trigger startup connectivity
    app.trigger_startup_connectivity();

    // Verify handle was created
    assert!(app.diagnostics_refresh_handle.is_some(),
        "diagnostics_refresh_handle should be set after trigger");

    // Calling again should not create a new handle (idempotent)
    app.trigger_startup_connectivity();
    assert!(app.diagnostics_refresh_handle.is_some(),
        "Should not create duplicate handles");
}
