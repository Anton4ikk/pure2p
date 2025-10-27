//! Chat management tests (creation, deletion, selection)

use crate::tui::Screen;
use super::helpers::create_test_app;

#[test]
fn test_app_delete_chat() {
    let (mut app, _temp_dir) = create_test_app();

    // Add some test chats
    app.app_state.add_chat("alice_uid".to_string());
    app.app_state.add_chat("bob_uid".to_string());
    app.app_state.add_chat("charlie_uid".to_string());
    assert_eq!(app.app_state.chats.len(), 3);

    // Show chat list screen
    app.show_chat_list_screen();

    // Delete second chat (bob) - now requires confirmation
    if let Some(screen) = &mut app.chat_list_screen {
        screen.selected_index = 1;
    }
    app.show_delete_confirmation();
    app.confirm_delete_chat();

    // Should have 2 chats left
    assert_eq!(app.app_state.chats.len(), 2);
    assert_eq!(app.app_state.chats[0].contact_uid, "alice_uid");
    assert_eq!(app.app_state.chats[1].contact_uid, "charlie_uid");

    // Status message should be set
    assert!(app.chat_list_screen.as_ref().unwrap().status_message.is_some());
}

#[test]
fn test_app_delete_last_chat() {
    let (mut app, _temp_dir) = create_test_app();

    // Add one chat
    app.app_state.add_chat("alice_uid".to_string());
    assert_eq!(app.app_state.chats.len(), 1);

    // Show chat list screen
    app.show_chat_list_screen();

    // Delete the chat - now requires confirmation
    app.show_delete_confirmation();
    app.confirm_delete_chat();

    // Should have no chats
    assert_eq!(app.app_state.chats.len(), 0);
}

#[test]
fn test_app_delete_adjusts_selection() {
    let (mut app, _temp_dir) = create_test_app();

    // Add chats
    app.app_state.add_chat("alice_uid".to_string());
    app.app_state.add_chat("bob_uid".to_string());

    // Show chat list and select last chat
    app.show_chat_list_screen();
    if let Some(screen) = &mut app.chat_list_screen {
        screen.selected_index = 1;
    }

    // Delete it - now requires confirmation
    app.show_delete_confirmation();
    app.confirm_delete_chat();

    // Selection should be adjusted to 0
    assert_eq!(app.chat_list_screen.as_ref().unwrap().selected_index, 0);
}

#[test]
fn test_app_open_selected_chat() {
    let (mut app, _temp_dir) = create_test_app();

    // Add a chat
    app.app_state.add_chat("alice_uid".to_string());

    // Show chat list
    app.show_chat_list_screen();

    // Open the chat
    app.open_selected_chat();

    // Should be on chat view screen
    assert_eq!(app.current_screen, Screen::ChatView);
    assert!(app.chat_view_screen.is_some());
    assert_eq!(
        app.chat_view_screen.as_ref().unwrap().contact_uid,
        "alice_uid"
    );
}

#[test]
fn test_app_back_to_chat_list() {
    let (mut app, _temp_dir) = create_test_app();

    // Add chat and open it
    app.app_state.add_chat("alice_uid".to_string());
    app.show_chat_list_screen();
    app.open_selected_chat();

    assert_eq!(app.current_screen, Screen::ChatView);

    // Go back
    app.back_to_chat_list();

    assert_eq!(app.current_screen, Screen::ChatList);
    assert!(app.chat_view_screen.is_none());
}

#[test]
fn test_app_show_delete_confirmation() {
    let (mut app, _temp_dir) = create_test_app();

    // Add some chats
    app.app_state.add_chat("alice_uid".to_string());
    app.app_state.add_chat("bob_uid".to_string());

    // Show chat list
    app.show_chat_list_screen();

    // Initially no popup
    assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);

    // Show delete confirmation
    app.show_delete_confirmation();

    // Popup should be shown
    assert!(app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);
    assert_eq!(app.chat_list_screen.as_ref().unwrap().pending_delete_index, Some(0));
}

#[test]
fn test_app_cancel_delete_chat() {
    let (mut app, _temp_dir) = create_test_app();

    // Add chat
    app.app_state.add_chat("alice_uid".to_string());

    // Show chat list and delete confirmation
    app.show_chat_list_screen();
    app.show_delete_confirmation();

    assert!(app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);

    // Cancel deletion
    app.cancel_delete_chat();

    // Popup should be hidden
    assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);
    assert!(app.chat_list_screen.as_ref().unwrap().pending_delete_index.is_none());

    // Chat should still exist
    assert_eq!(app.app_state.chats.len(), 1);
}

#[test]
fn test_app_confirm_delete_inactive_chat() {
    let (mut app, _temp_dir) = create_test_app();

    // Add inactive chat
    app.app_state.add_chat("alice_uid".to_string());
    app.app_state.chats[0].is_active = false;

    // Show chat list and delete confirmation
    app.show_chat_list_screen();
    app.show_delete_confirmation();

    // Confirm deletion
    app.confirm_delete_chat();

    // Chat should be deleted
    assert_eq!(app.app_state.chats.len(), 0);

    // Popup should be hidden
    assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);

    // Status should indicate inactive chat deletion
    let status = app.chat_list_screen.as_ref().unwrap().status_message.as_ref();
    assert!(status.is_some());
    assert!(status.unwrap().contains("inactive"));
}

#[test]
fn test_app_confirm_delete_active_chat() {
    let (mut app, _temp_dir) = create_test_app();

    // Add active chat
    app.app_state.add_chat("alice_uid".to_string());
    app.app_state.chats[0].is_active = true;

    // Show chat list and delete confirmation
    app.show_chat_list_screen();
    app.show_delete_confirmation();

    // Confirm deletion
    app.confirm_delete_chat();

    // Chat should be deleted
    assert_eq!(app.app_state.chats.len(), 0);

    // Popup should be hidden
    assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);

    // Status should indicate delete request was sent
    let status = app.chat_list_screen.as_ref().unwrap().status_message.as_ref();
    assert!(status.is_some());
    assert!(status.unwrap().contains("delete request"));
}

#[test]
fn test_app_delete_chat_adjusts_selection() {
    let (mut app, _temp_dir) = create_test_app();

    // Add chats
    app.app_state.add_chat("alice_uid".to_string());
    app.app_state.add_chat("bob_uid".to_string());
    app.app_state.add_chat("charlie_uid".to_string());

    // Show chat list and select last chat
    app.show_chat_list_screen();
    if let Some(screen) = &mut app.chat_list_screen {
        screen.selected_index = 2;
    }

    // Show confirmation for last chat and confirm
    app.show_delete_confirmation();
    app.confirm_delete_chat();

    // Selection should be adjusted to index 1 (the new last item)
    assert_eq!(app.chat_list_screen.as_ref().unwrap().selected_index, 1);
    assert_eq!(app.app_state.chats.len(), 2);
}

#[test]
fn test_app_delete_middle_chat_keeps_selection() {
    let (mut app, _temp_dir) = create_test_app();

    // Add chats
    app.app_state.add_chat("alice_uid".to_string());
    app.app_state.add_chat("bob_uid".to_string());
    app.app_state.add_chat("charlie_uid".to_string());

    // Show chat list and select middle chat
    app.show_chat_list_screen();
    if let Some(screen) = &mut app.chat_list_screen {
        screen.selected_index = 1;
    }

    // Delete middle chat
    app.show_delete_confirmation();
    app.confirm_delete_chat();

    // Selection should stay at index 1 (now pointing to charlie)
    assert_eq!(app.chat_list_screen.as_ref().unwrap().selected_index, 1);
    assert_eq!(app.app_state.chats.len(), 2);
    assert_eq!(app.app_state.chats[0].contact_uid, "alice_uid");
    assert_eq!(app.app_state.chats[1].contact_uid, "charlie_uid");
}

#[test]
fn test_app_delete_only_chat() {
    let (mut app, _temp_dir) = create_test_app();

    // Add one chat
    app.app_state.add_chat("alice_uid".to_string());

    // Show chat list
    app.show_chat_list_screen();

    // Delete the only chat
    app.show_delete_confirmation();
    app.confirm_delete_chat();

    // Should have no chats
    assert_eq!(app.app_state.chats.len(), 0);

    // Popup should be hidden
    assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);
}

#[test]
fn test_app_delete_empty_list_does_nothing() {
    let (mut app, _temp_dir) = create_test_app();

    // Show chat list (with no chats)
    app.show_chat_list_screen();

    // Try to show delete confirmation
    app.show_delete_confirmation();

    // Popup should not be shown
    assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);
}

#[test]
fn test_confirm_delete_with_invalid_index() {
    let (mut app, _temp_dir) = create_test_app();

    // Add one chat
    app.app_state.add_chat("alice_uid".to_string());

    // Show chat list
    app.show_chat_list_screen();

    // Manually set an invalid pending delete index
    if let Some(screen) = &mut app.chat_list_screen {
        screen.show_delete_confirmation = true;
        screen.pending_delete_index = Some(999);
    }

    let initial_count = app.app_state.chats.len();

    // Try to confirm delete
    app.confirm_delete_chat();

    // Chat should not be deleted
    assert_eq!(app.app_state.chats.len(), initial_count);
}
