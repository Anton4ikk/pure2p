// App Tests - Testing App struct and its methods

use crate::tui::{App, Screen, MenuItem};
use crate::storage::Settings;
use std::sync::Mutex;

// Global mutex to serialize access to settings.json across all tests
static SETTINGS_LOCK: Mutex<()> = Mutex::new(());

/// Helper function to ensure settings file exists with consent set (for testing)
/// This function acquires a lock to prevent race conditions between parallel tests
fn ensure_consent_set() -> std::sync::MutexGuard<'static, ()> {
    let guard = SETTINGS_LOCK.lock().unwrap();
    let settings = Settings::load("settings.json").unwrap_or_default();
    let _ = settings.save("settings.json");
    guard
}

/// Helper to clean up test settings (call AFTER test completes, releases lock)
fn cleanup_test_settings(_guard: std::sync::MutexGuard<()>) {
    let _ = std::fs::remove_file("settings.json");
    // Guard is dropped here, releasing the lock
}

#[test]
fn test_app_initialization() {
    let _lock = ensure_consent_set();
    let app = App::new().expect("Failed to create app");

    // Verify initial state
    assert_eq!(
        app.current_screen,
        Screen::MainMenu,
        "Should start on main menu (with consent already set)"
    );

    cleanup_test_settings(_lock);
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
fn test_app_navigation() {
    let mut app = App::new().expect("Failed to create app");

    // Test next navigation
    assert_eq!(app.selected_index, 0);
    app.next();
    assert_eq!(app.selected_index, 1);
    app.next();
    assert_eq!(app.selected_index, 2);
    app.next();
    assert_eq!(app.selected_index, 3);
    app.next();
    assert_eq!(app.selected_index, 4);
    app.next();
    assert_eq!(app.selected_index, 5);

    // Test wrap around
    app.next();
    assert_eq!(app.selected_index, 0, "Should wrap to beginning");

    // Test previous navigation
    app.previous();
    assert_eq!(app.selected_index, 5, "Should wrap to end");
    app.previous();
    assert_eq!(app.selected_index, 4);
}

#[test]
fn test_app_show_share_contact_screen() {
    let _lock = ensure_consent_set();
    let mut app = App::new().expect("Failed to create app");

    // Initially on main menu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.share_contact_screen.is_none());

    // Show share contact screen
    app.show_share_contact_screen();

    // Verify screen changed
    assert_eq!(app.current_screen, Screen::ShareContact);
    assert!(
        app.share_contact_screen.is_some(),
        "Share contact screen should be initialized"
    );

    // Verify token was generated
    let screen = app.share_contact_screen.as_ref().unwrap();
    assert!(!screen.token.is_empty());
    assert!(screen.expiry > chrono::Utc::now());

    cleanup_test_settings(_lock);
}

#[test]
fn test_app_back_to_main_menu() {
    let mut app = App::new().expect("Failed to create app");

    // Show share contact screen
    app.show_share_contact_screen();
    assert_eq!(app.current_screen, Screen::ShareContact);
    assert!(app.share_contact_screen.is_some());

    // Go back to main menu
    app.back_to_main_menu();

    // Verify state
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(
        app.share_contact_screen.is_none(),
        "Share contact screen should be cleared"
    );
}

#[test]
fn test_app_select_share_contact() {
    let mut app = App::new().expect("Failed to create app");

    // Select ShareContact item (index 1)
    app.selected_index = 1;
    assert_eq!(app.selected_item(), MenuItem::ShareContact);

    // Trigger selection
    app.select();

    // Should navigate to share contact screen
    assert_eq!(app.current_screen, Screen::ShareContact);
    assert!(app.share_contact_screen.is_some());
}

#[test]
fn test_app_select_exit() {
    let mut app = App::new().expect("Failed to create app");

    // Navigate to Exit item (index 5)
    app.selected_index = 5;
    assert_eq!(app.selected_item(), MenuItem::Exit);
    assert!(!app.should_quit);

    // Trigger selection
    app.select();

    // Should set quit flag
    assert!(app.should_quit);
}

#[test]
fn test_app_show_import_contact_screen() {
    let _lock = ensure_consent_set();
    let mut app = App::new().expect("Failed to create app");

    // Initially on main menu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.import_contact_screen.is_none());

    // Show import contact screen
    app.show_import_contact_screen();

    // Verify screen changed
    assert_eq!(app.current_screen, Screen::ImportContact);
    assert!(
        app.import_contact_screen.is_some(),
        "Import contact screen should be initialized"
    );

    cleanup_test_settings(_lock);
}

#[test]
fn test_app_back_from_import_contact() {
    let mut app = App::new().expect("Failed to create app");

    // Show import contact screen
    app.show_import_contact_screen();
    assert_eq!(app.current_screen, Screen::ImportContact);
    assert!(app.import_contact_screen.is_some());

    // Go back to main menu
    app.back_to_main_menu();

    // Verify state
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(
        app.import_contact_screen.is_none(),
        "Import contact screen should be cleared"
    );
}

#[test]
fn test_app_select_import_contact() {
    let mut app = App::new().expect("Failed to create app");

    // Navigate to ImportContact item (index 2)
    app.selected_index = 2;
    assert_eq!(app.selected_item(), MenuItem::ImportContact);

    // Trigger selection
    app.select();

    // Should navigate to import contact screen
    assert_eq!(app.current_screen, Screen::ImportContact);
    assert!(app.import_contact_screen.is_some());
}

#[test]
fn test_app_show_chat_list_screen() {
    let _lock = ensure_consent_set();
    let mut app = App::new().expect("Failed to create app");

    // Initially on main menu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.chat_list_screen.is_none());

    // Show chat list screen
    app.show_chat_list_screen();

    // Verify screen changed
    assert_eq!(app.current_screen, Screen::ChatList);
    assert!(
        app.chat_list_screen.is_some(),
        "Chat list screen should be initialized"
    );

    cleanup_test_settings(_lock);
}

#[test]
fn test_app_back_from_chat_list() {
    let mut app = App::new().expect("Failed to create app");

    // Show chat list screen
    app.show_chat_list_screen();
    assert_eq!(app.current_screen, Screen::ChatList);
    assert!(app.chat_list_screen.is_some());

    // Go back to main menu
    app.back_to_main_menu();

    // Verify state
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(
        app.chat_list_screen.is_none(),
        "Chat list screen should be cleared"
    );
}

#[test]
fn test_app_select_chat_list() {
    let mut app = App::new().expect("Failed to create app");

    // Navigate to ChatList item (index 0)
    app.selected_index = 0;
    assert_eq!(app.selected_item(), MenuItem::ChatList);

    // Trigger selection
    app.select();

    // Should navigate to chat list screen
    assert_eq!(app.current_screen, Screen::ChatList);
    assert!(app.chat_list_screen.is_some());
}

#[test]
fn test_app_delete_chat() {
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

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
fn test_app_state_initialized() {
    let app = App::new().expect("Failed to create app");

    assert!(app.app_state.chats.is_empty(), "Should have no chats initially");
    assert!(app.app_state.contacts.is_empty(), "Should have no contacts initially");
}

#[test]
fn test_app_open_selected_chat() {
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

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
fn test_app_send_message() {
    let mut app = App::new().expect("Failed to create app");

    // Add chat
    app.app_state.add_chat("alice_uid".to_string());

    // Open chat
    app.show_chat_list_screen();
    app.open_selected_chat();

    // Type a message
    if let Some(screen) = &mut app.chat_view_screen {
        screen.input = "Hello Alice!".to_string();
    }

    // Send it
    app.send_message_in_chat();

    // Verify message was added
    let chat = app.app_state.chats.iter()
        .find(|c| c.contact_uid == "alice_uid")
        .unwrap();
    assert_eq!(chat.messages.len(), 1);

    let msg = &chat.messages[0];
    assert_eq!(msg.sender, app.keypair.uid.to_string());
    assert_eq!(msg.recipient, "alice_uid");
    assert_eq!(
        String::from_utf8(msg.content.clone()).unwrap(),
        "Hello Alice!"
    );

    // Input should be cleared
    assert!(app.chat_view_screen.as_ref().unwrap().input.is_empty());
}

#[test]
fn test_app_send_empty_message() {
    let mut app = App::new().expect("Failed to create app");

    // Add chat
    app.app_state.add_chat("alice_uid".to_string());

    // Open chat
    app.show_chat_list_screen();
    app.open_selected_chat();

    // Try to send empty message
    app.send_message_in_chat();

    // Should not have added any message
    let chat = app.app_state.chats.iter()
        .find(|c| c.contact_uid == "alice_uid")
        .unwrap();
    assert_eq!(chat.messages.len(), 0);
}

#[test]
fn test_app_multiple_messages() {
    let mut app = App::new().expect("Failed to create app");

    // Add chat
    app.app_state.add_chat("alice_uid".to_string());

    // Open chat
    app.show_chat_list_screen();
    app.open_selected_chat();

    // Send multiple messages
    for i in 1..=3 {
        if let Some(screen) = &mut app.chat_view_screen {
            screen.input = format!("Message {}", i);
        }
        app.send_message_in_chat();
    }

    // Verify all messages were added
    let chat = app.app_state.chats.iter()
        .find(|c| c.contact_uid == "alice_uid")
        .unwrap();
    assert_eq!(chat.messages.len(), 3);

    for (i, msg) in chat.messages.iter().enumerate() {
        let expected = format!("Message {}", i + 1);
        assert_eq!(
            String::from_utf8(msg.content.clone()).unwrap(),
            expected
        );
    }
}

#[test]
fn test_app_show_delete_confirmation() {
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

    // Show chat list (with no chats)
    app.show_chat_list_screen();

    // Try to show delete confirmation
    app.show_delete_confirmation();

    // Popup should not be shown
    assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);
}

#[test]
fn test_confirm_delete_with_invalid_index() {
    let mut app = App::new().expect("Failed to create app");

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

#[test]
fn test_app_show_settings_screen() {
    let _lock = ensure_consent_set();
    let mut app = App::new().expect("Failed to create app");

    // Initially on main menu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.settings_screen.is_none());

    // Show settings screen
    app.show_settings_screen();

    // Verify screen changed
    assert_eq!(app.current_screen, Screen::Settings);
    assert!(app.settings_screen.is_some());

    cleanup_test_settings(_lock);
}

#[test]
fn test_app_back_from_settings() {
    let mut app = App::new().expect("Failed to create app");

    // Show settings screen
    app.show_settings_screen();
    assert_eq!(app.current_screen, Screen::Settings);
    assert!(app.settings_screen.is_some());

    // Go back to main menu
    app.back_to_main_menu();

    // Verify state
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.settings_screen.is_none());
}

#[test]
fn test_app_select_settings() {
    let mut app = App::new().expect("Failed to create app");

    // Navigate to Settings item (index 4)
    app.selected_index = 4;
    assert_eq!(app.selected_item(), MenuItem::Settings);

    // Trigger selection
    app.select();

    // Should navigate to settings screen
    assert_eq!(app.current_screen, Screen::Settings);
    assert!(app.settings_screen.is_some());
}

#[test]
fn test_app_startup_with_no_pending_messages() {
    let _lock = ensure_consent_set();
    let app = App::new().expect("Failed to create app");

    // With no pending messages and consent set, should start on MainMenu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.startup_sync_screen.is_none());

    cleanup_test_settings(_lock);
}

#[test]
fn test_app_update_startup_sync() {
    let mut app = App::new().expect("Failed to create app");

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
    let mut app = App::new().expect("Failed to create app");

    // Set up startup sync screen
    app.startup_sync_screen = Some(crate::tui::screens::StartupSyncScreen::new(1));
    app.current_screen = Screen::StartupSync;

    // Complete the sync
    app.complete_startup_sync();

    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.startup_sync_screen.is_none());
}

