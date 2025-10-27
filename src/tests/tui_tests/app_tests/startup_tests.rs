//! App startup and connectivity tests

use crate::tui::Screen;
use super::helpers::create_test_app;

#[test]
fn test_app_startup_with_no_pending_messages() {
    let (app, _temp_dir) = create_test_app();

    // App should always start on MainMenu (retry worker handles queue silently)
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
