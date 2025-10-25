// StartupSyncScreen Tests - Testing startup message queue sync screen

use crate::tui::screens::StartupSyncScreen;

#[test]
fn test_startup_sync_screen_creation() {
    let screen = StartupSyncScreen::new(10);

    assert_eq!(screen.total_messages, 10);
    assert_eq!(screen.succeeded, 0);
    assert_eq!(screen.failed, 0);
    assert_eq!(screen.current, 0);
    assert!(!screen.is_complete);
}

#[test]
fn test_startup_sync_screen_process_message_success() {
    let mut screen = StartupSyncScreen::new(3);

    screen.process_message(true);
    assert_eq!(screen.succeeded, 1);
    assert_eq!(screen.failed, 0);
    assert_eq!(screen.current, 1);
    assert!(!screen.is_complete);

    screen.process_message(true);
    assert_eq!(screen.succeeded, 2);
    assert_eq!(screen.current, 2);
    assert!(!screen.is_complete);

    screen.process_message(true);
    assert_eq!(screen.succeeded, 3);
    assert_eq!(screen.current, 3);
    assert!(screen.is_complete);
}

#[test]
fn test_startup_sync_screen_process_message_failure() {
    let mut screen = StartupSyncScreen::new(2);

    screen.process_message(false);
    assert_eq!(screen.succeeded, 0);
    assert_eq!(screen.failed, 1);
    assert_eq!(screen.current, 1);

    screen.process_message(false);
    assert_eq!(screen.succeeded, 0);
    assert_eq!(screen.failed, 2);
    assert_eq!(screen.current, 2);
    assert!(screen.is_complete);
}

#[test]
fn test_startup_sync_screen_mixed_results() {
    let mut screen = StartupSyncScreen::new(5);

    screen.process_message(true);   // Success
    screen.process_message(false);  // Fail
    screen.process_message(true);   // Success
    screen.process_message(true);   // Success
    screen.process_message(false);  // Fail

    assert_eq!(screen.succeeded, 3);
    assert_eq!(screen.failed, 2);
    assert_eq!(screen.current, 5);
    assert!(screen.is_complete);
}

#[test]
fn test_startup_sync_screen_progress_percentage() {
    let mut screen = StartupSyncScreen::new(10);

    assert_eq!(screen.get_progress_percentage(), 0);

    screen.current = 2;
    assert_eq!(screen.get_progress_percentage(), 20);

    screen.current = 5;
    assert_eq!(screen.get_progress_percentage(), 50);

    screen.current = 10;
    assert_eq!(screen.get_progress_percentage(), 100);
}

#[test]
fn test_startup_sync_screen_progress_percentage_empty() {
    let screen = StartupSyncScreen::new(0);
    assert_eq!(screen.get_progress_percentage(), 100);
}

#[test]
fn test_startup_sync_screen_elapsed_time() {
    let screen = StartupSyncScreen::new(5);

    // Should return a formatted time string
    let elapsed = screen.get_elapsed_time();
    assert!(elapsed.ends_with('s'));
    assert!(elapsed.contains('.'));
}

#[test]
fn test_startup_sync_completes_after_all_messages() {
    let mut screen = StartupSyncScreen::new(3);

    assert!(!screen.is_complete);

    screen.process_message(true);
    assert!(!screen.is_complete);

    screen.process_message(true);
    assert!(!screen.is_complete);

    screen.process_message(true);
    assert!(screen.is_complete);

    // Processing after complete should not panic
    screen.process_message(true);
    assert_eq!(screen.current, 4); // Still increments
}

#[test]
fn test_startup_sync_screen_zero_messages() {
    let screen = StartupSyncScreen::new(0);

    assert_eq!(screen.total_messages, 0);
    assert!(screen.is_complete); // Should be complete immediately
    assert_eq!(screen.get_progress_percentage(), 100);
}
