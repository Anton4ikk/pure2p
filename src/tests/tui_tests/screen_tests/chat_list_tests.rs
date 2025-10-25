// ChatListScreen Tests - Testing chat list navigation and management

use crate::tui::screens::ChatListScreen;

#[test]
fn test_chat_list_screen_creation() {
    let screen = ChatListScreen::new();

    assert_eq!(screen.selected_index, 0, "Should start at index 0");
    assert!(
        screen.status_message.is_none(),
        "Should have no status message initially"
    );
}

#[test]
fn test_chat_list_screen_navigation() {
    let mut screen = ChatListScreen::new();

    // Test next with 3 chats
    screen.next(3);
    assert_eq!(screen.selected_index, 1);
    screen.next(3);
    assert_eq!(screen.selected_index, 2);

    // Test wrap around
    screen.next(3);
    assert_eq!(screen.selected_index, 0, "Should wrap to beginning");

    // Test previous
    screen.previous(3);
    assert_eq!(screen.selected_index, 2, "Should wrap to end");
    screen.previous(3);
    assert_eq!(screen.selected_index, 1);

    // Test with empty list
    screen.next(0);
    assert_eq!(screen.selected_index, 1, "Should not change with empty list");
}

#[test]
fn test_chat_list_screen_status() {
    let mut screen = ChatListScreen::new();

    assert!(screen.status_message.is_none());

    screen.set_status("Test status".to_string());
    assert_eq!(screen.status_message.as_ref().unwrap(), "Test status");

    screen.clear_status();
    assert!(screen.status_message.is_none());
}

#[test]
fn test_chat_list_screen_delete_popup() {
    let mut screen = ChatListScreen::new();

    // Initially no popup shown
    assert!(!screen.show_delete_confirmation);
    assert!(screen.pending_delete_index.is_none());

    // Show delete popup
    screen.show_delete_popup(2);
    assert!(screen.show_delete_confirmation);
    assert_eq!(screen.pending_delete_index, Some(2));

    // Hide delete popup
    screen.hide_delete_popup();
    assert!(!screen.show_delete_confirmation);
    assert!(screen.pending_delete_index.is_none());
}

#[test]
fn test_chat_list_screen_popup_state_independent() {
    let mut screen = ChatListScreen::new();

    // Can call hide without showing first
    screen.hide_delete_popup();
    assert!(!screen.show_delete_confirmation);

    // Can call show multiple times
    screen.show_delete_popup(0);
    screen.show_delete_popup(1);
    assert!(screen.show_delete_confirmation);
    assert_eq!(screen.pending_delete_index, Some(1));
}
