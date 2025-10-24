// TUI Tests - now testing the public tui module

use crate::crypto::KeyPair;
use crate::storage::{generate_contact_token, parse_contact_token, Contact, Settings};
use crate::tui::{App, Screen, MenuItem};
use crate::tui::screens::*;
use crate::tui::ui::format_duration_until;
use chrono::{Duration, Utc};

#[test]
fn test_share_contact_screen_creation() {
    // Create a keypair for testing
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";

    // Create share contact screen
    let screen = ShareContactScreen::new(&keypair, local_ip);

    // Verify token is non-empty
    assert!(!screen.token.is_empty(), "Token should not be empty");

    // Verify expiry is in the future
    assert!(
        screen.expiry > Utc::now(),
        "Expiry should be in the future"
    );

    // Verify default expiry is approximately 30 days
    let expiry_duration = screen.expiry.signed_duration_since(Utc::now());
    assert!(
        expiry_duration.num_days() >= 29 && expiry_duration.num_days() <= 30,
        "Default expiry should be approximately 30 days, got {} days",
        expiry_duration.num_days()
    );

    // Verify no initial status message
    assert!(
        screen.status_message.is_none(),
        "Status message should be None initially"
    );
}

#[test]
fn test_share_contact_screen_token_valid() {
    // Create a keypair for testing
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";

    // Create share contact screen
    let screen = ShareContactScreen::new(&keypair, local_ip);

    // Parse the token to verify it's valid
    let parsed_contact =
        parse_contact_token(&screen.token).expect("Token should be valid and parseable");

    // Verify parsed contact matches expected values
    assert_eq!(
        parsed_contact.ip, local_ip,
        "Parsed IP should match input"
    );
    assert_eq!(
        parsed_contact.pubkey,
        keypair.public_key,
        "Parsed public key should match keypair"
    );
    assert_eq!(
        parsed_contact.uid,
        keypair.uid.to_string(),
        "Parsed UID should match keypair UID"
    );
}

#[test]
fn test_share_contact_screen_save_to_file() {
    use tempfile::TempDir;

    // Create a temporary directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    // Change to temp directory for the test
    std::env::set_current_dir(temp_dir.path()).expect("Failed to change dir");

    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";

    let mut screen = ShareContactScreen::new(&keypair, local_ip);
    let original_token = screen.token.clone();

    // Save to file
    screen.save_to_file();

    // Verify status message indicates success
    assert!(
        screen
            .status_message
            .as_ref()
            .expect("Status message should be set")
            .starts_with("Saved to"),
        "Status message should indicate successful save"
    );

    // Find the generated file
    let entries = std::fs::read_dir(temp_dir.path()).expect("Failed to read temp dir");
    let mut found_file = false;

    for entry in entries {
        let entry = entry.expect("Failed to read entry");
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if filename_str.starts_with("contact_token_") && filename_str.ends_with(".txt") {
            // Read the file contents
            let contents = std::fs::read_to_string(entry.path())
                .expect("Failed to read saved token file");

            // Verify contents match the token
            assert_eq!(
                contents.trim(),
                original_token,
                "Saved token should match original"
            );
            found_file = true;
            break;
        }
    }

    assert!(found_file, "Token file should have been created");

    // Restore original directory
    std::env::set_current_dir(original_dir).expect("Failed to restore dir");
}

#[test]
fn test_format_duration_until_days() {
    let expiry = Utc::now() + Duration::days(15);
    let formatted = format_duration_until(expiry);
    // Allow for slight timing variance (14-15 days)
    assert!(
        formatted == "14 days" || formatted == "15 days",
        "Expected 14 or 15 days, got: {}",
        formatted
    );
}

#[test]
fn test_format_duration_until_hours() {
    let expiry = Utc::now() + Duration::hours(12);
    let formatted = format_duration_until(expiry);
    // Allow for slight timing variance (11-12 hours)
    assert!(
        formatted == "11 hours" || formatted == "12 hours",
        "Expected 11 or 12 hours, got: {}",
        formatted
    );
}

#[test]
fn test_format_duration_until_minutes() {
    let expiry = Utc::now() + Duration::minutes(45);
    let formatted = format_duration_until(expiry);
    // Allow for slight timing variance (44-45 minutes)
    assert!(
        formatted == "44 minutes" || formatted == "45 minutes",
        "Expected 44 or 45 minutes, got: {}",
        formatted
    );
}

#[test]
fn test_format_duration_until_expired() {
    let expiry = Utc::now() - Duration::hours(1);
    let formatted = format_duration_until(expiry);
    assert_eq!(formatted, "expired");
}

#[test]
fn test_app_initialization() {
    let app = App::new().expect("Failed to create app");

    // Verify initial state
    assert_eq!(
        app.current_screen,
        Screen::MainMenu,
        "Should start on main menu"
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
    assert!(screen.expiry > Utc::now());
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
fn test_menu_item_labels() {
    assert_eq!(MenuItem::ShareContact.label(), "Share Contact");
    assert_eq!(MenuItem::ImportContact.label(), "Import Contact");
    assert_eq!(MenuItem::Settings.label(), "Settings");
    assert_eq!(MenuItem::Exit.label(), "Exit");
}

#[test]
fn test_menu_item_descriptions() {
    assert_eq!(
        MenuItem::ShareContact.description(),
        "Generate and share your contact token"
    );
    assert_eq!(
        MenuItem::ImportContact.description(),
        "Import a contact from their token"
    );
    assert_eq!(
        MenuItem::Settings.description(),
        "Configure application settings"
    );
    assert_eq!(MenuItem::Exit.description(), "Exit Pure2P");
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
fn test_token_consistency() {
    // Same keypair and IP should generate same token
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";
    let expiry = Utc::now() + Duration::days(30);

    let token1 = generate_contact_token(local_ip, &keypair.public_key, expiry);
    let token2 = generate_contact_token(local_ip, &keypair.public_key, expiry);

    assert_eq!(
        token1, token2,
        "Same inputs should generate identical tokens"
    );
}

#[test]
fn test_different_keypairs_different_tokens() {
    // Different keypairs should generate different tokens
    let keypair1 = KeyPair::generate().expect("Failed to generate keypair");
    let keypair2 = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";
    let expiry = Utc::now() + Duration::days(30);

    let token1 = generate_contact_token(local_ip, &keypair1.public_key, expiry);
    let token2 = generate_contact_token(local_ip, &keypair2.public_key, expiry);

    assert_ne!(
        token1, token2,
        "Different keypairs should generate different tokens"
    );
}

#[test]
fn test_import_contact_screen_creation() {
    let screen = ImportContactScreen::new();

    assert!(screen.input.is_empty(), "Input should be empty initially");
    assert!(
        screen.parsed_contact.is_none(),
        "No contact should be parsed initially"
    );
    assert!(
        screen.status_message.is_some(),
        "Should have initial status message"
    );
    assert!(!screen.is_error, "Should not be in error state initially");
}

#[test]
fn test_import_contact_screen_add_char() {
    let mut screen = ImportContactScreen::new();

    screen.add_char('a');
    assert_eq!(screen.input, "a");

    screen.add_char('b');
    assert_eq!(screen.input, "ab");

    screen.add_char('c');
    assert_eq!(screen.input, "abc");
}

#[test]
fn test_import_contact_screen_backspace() {
    let mut screen = ImportContactScreen::new();
    screen.input = "hello".to_string();

    screen.backspace();
    assert_eq!(screen.input, "hell");

    screen.backspace();
    assert_eq!(screen.input, "hel");

    // Backspace on empty should not panic
    screen.input.clear();
    screen.backspace();
    assert_eq!(screen.input, "");
}

#[test]
fn test_import_contact_screen_clear() {
    let mut screen = ImportContactScreen::new();
    screen.input = "some token".to_string();
    screen.is_error = true;

    screen.clear();

    assert!(screen.input.is_empty(), "Input should be cleared");
    assert!(
        screen.parsed_contact.is_none(),
        "Parsed contact should be cleared"
    );
    assert!(!screen.is_error, "Error flag should be cleared");
    assert!(screen.status_message.is_some(), "Should have status message");
}

#[test]
fn test_import_contact_screen_parse_empty() {
    let mut screen = ImportContactScreen::new();

    screen.parse_token();

    assert!(screen.is_error, "Should be in error state for empty input");
    assert!(
        screen.parsed_contact.is_none(),
        "Should not have parsed contact"
    );
    assert!(
        screen
            .status_message
            .as_ref()
            .unwrap()
            .contains("empty"),
        "Status should mention empty token"
    );
}

#[test]
fn test_import_contact_screen_parse_invalid() {
    let mut screen = ImportContactScreen::new();
    screen.input = "invalid_token_data".to_string();

    screen.parse_token();

    assert!(screen.is_error, "Should be in error state for invalid token");
    assert!(
        screen.parsed_contact.is_none(),
        "Should not have parsed contact"
    );
    assert!(
        screen
            .status_message
            .as_ref()
            .unwrap()
            .contains("Error"),
        "Status should indicate error"
    );
}

#[test]
fn test_import_contact_screen_parse_valid() {
    // Generate a valid token
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";
    let expiry = Utc::now() + Duration::days(30);
    let token = generate_contact_token(local_ip, &keypair.public_key, expiry);

    let mut screen = ImportContactScreen::new();
    screen.input = token.clone();

    screen.parse_token();

    assert!(!screen.is_error, "Should not be in error state");
    assert!(
        screen.parsed_contact.is_some(),
        "Should have parsed contact"
    );

    let contact = screen.parsed_contact.as_ref().unwrap();
    assert_eq!(contact.ip, local_ip);
    assert_eq!(contact.uid, keypair.uid.to_string());
}

#[test]
fn test_import_contact_screen_get_contact() {
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";
    let expiry = Utc::now() + Duration::days(30);
    let token = generate_contact_token(local_ip, &keypair.public_key, expiry);

    let mut screen = ImportContactScreen::new();

    // Initially no contact
    assert!(screen.get_contact().is_none());

    // Parse valid token
    screen.input = token;
    screen.parse_token();

    // Should have contact now
    assert!(screen.get_contact().is_some());
    assert_eq!(screen.get_contact().unwrap().ip, local_ip);
}

#[test]
fn test_app_show_import_contact_screen() {
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
fn test_app_show_chat_list_screen() {
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
fn test_menu_items_updated() {
    // Verify ChatList is first item
    assert_eq!(MenuItem::ChatList.label(), "Chat List");
    assert_eq!(
        MenuItem::ChatList.description(),
        "View and manage your conversations"
    );

    // Verify menu has 6 items now (added Diagnostics)
    let items = MenuItem::all();
    assert_eq!(items.len(), 6);
    assert_eq!(items[0], MenuItem::ChatList);
    assert_eq!(items[1], MenuItem::ShareContact);
    assert_eq!(items[2], MenuItem::ImportContact);
    assert_eq!(items[3], MenuItem::Diagnostics);
    assert_eq!(items[4], MenuItem::Settings);
    assert_eq!(items[5], MenuItem::Exit);
}

#[test]
fn test_chat_view_screen_creation() {
    let screen = ChatViewScreen::new("alice_uid".to_string());

    assert_eq!(screen.contact_uid, "alice_uid");
    assert!(screen.input.is_empty(), "Input should be empty initially");
    assert_eq!(screen.scroll_offset, 0, "Should start at top");
    assert!(
        screen.status_message.is_none(),
        "Should have no status message initially"
    );
}

#[test]
fn test_chat_view_screen_input() {
    let mut screen = ChatViewScreen::new("alice_uid".to_string());

    screen.add_char('H');
    screen.add_char('i');
    assert_eq!(screen.input, "Hi");

    screen.backspace();
    assert_eq!(screen.input, "H");

    screen.clear_input();
    assert!(screen.input.is_empty());
}

#[test]
fn test_chat_view_screen_scroll() {
    let mut screen = ChatViewScreen::new("alice_uid".to_string());

    // Scroll down
    screen.scroll_down(10);
    assert_eq!(screen.scroll_offset, 1);
    screen.scroll_down(10);
    assert_eq!(screen.scroll_offset, 2);

    // Scroll up
    screen.scroll_up();
    assert_eq!(screen.scroll_offset, 1);
    screen.scroll_up();
    assert_eq!(screen.scroll_offset, 0);

    // Can't scroll past 0
    screen.scroll_up();
    assert_eq!(screen.scroll_offset, 0);

    // Can't scroll past max
    screen.scroll_offset = 10;
    screen.scroll_down(10);
    assert_eq!(screen.scroll_offset, 10, "Should stay at max offset");
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
fn test_settings_screen_creation() {
    use tempfile::NamedTempFile;

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
    use tempfile::NamedTempFile;

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
    use tempfile::NamedTempFile;

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
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_string_lossy().to_string();

    let mut screen = SettingsScreen::new(path);
    screen.retry_interval_input = "123".to_string();

    screen.clear_input();
    assert_eq!(screen.retry_interval_input, "");
}

#[test]
fn test_settings_screen_validate_empty() {
    use tempfile::NamedTempFile;

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
    use tempfile::NamedTempFile;

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
    use tempfile::NamedTempFile;

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
    use tempfile::NamedTempFile;

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
    use tempfile::NamedTempFile;

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

#[test]
fn test_app_show_settings_screen() {
    let mut app = App::new().expect("Failed to create app");

    // Initially on main menu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.settings_screen.is_none());

    // Show settings screen
    app.show_settings_screen();

    // Verify screen changed
    assert_eq!(app.current_screen, Screen::Settings);
    assert!(app.settings_screen.is_some());
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
fn test_status_indicators_priority_expired_contact() {
    use chrono::{Duration, Utc};

    let mut app = App::new().expect("Failed to create app");

    // Add a chat
    app.app_state.add_chat("alice_uid".to_string());

    // Add an expired contact
    let contact = Contact::new(
        "alice_uid".to_string(),
        "192.168.1.100:8080".to_string(),
        vec![1, 2, 3, 4],
        Utc::now() - Duration::hours(1), // Expired 1 hour ago
    );
    app.app_state.contacts.push(contact);

    // Set chat to have pending messages AND be active
    app.app_state.chats[0].has_pending_messages = true;
    app.app_state.chats[0].is_active = true;

    // Expired contact should have highest priority
    // Even with pending and active flags set, should show warning indicator
    // We can't directly test the rendering, but we can verify the contact is expired
    let is_expired = app.app_state.contacts[0].is_expired();
    assert!(is_expired, "Contact should be expired");
}

#[test]
fn test_status_indicators_priority_pending_messages() {
    let mut app = App::new().expect("Failed to create app");

    // Add a chat with pending messages
    app.app_state.add_chat("bob_uid".to_string());
    app.app_state.chats[0].has_pending_messages = true;
    app.app_state.chats[0].is_active = true;

    // Add a non-expired contact
    let contact = Contact::new(
        "bob_uid".to_string(),
        "192.168.1.101:8080".to_string(),
        vec![5, 6, 7, 8],
        chrono::Utc::now() + chrono::Duration::days(30),
    );
    app.app_state.contacts.push(contact);

    // Verify pending messages flag is set
    assert!(app.app_state.chats[0].has_pending_messages);
    assert!(!app.app_state.contacts[0].is_expired());
}

#[test]
fn test_status_indicators_priority_active_chat() {
    let mut app = App::new().expect("Failed to create app");

    // Add an active chat (new messages)
    app.app_state.add_chat("charlie_uid".to_string());
    app.app_state.chats[0].is_active = true;
    app.app_state.chats[0].has_pending_messages = false;

    // Add a non-expired contact
    let contact = Contact::new(
        "charlie_uid".to_string(),
        "192.168.1.102:8080".to_string(),
        vec![9, 10, 11, 12],
        chrono::Utc::now() + chrono::Duration::days(30),
    );
    app.app_state.contacts.push(contact);

    // Verify chat is active and contact is not expired
    assert!(app.app_state.chats[0].is_active);
    assert!(!app.app_state.chats[0].has_pending_messages);
    assert!(!app.app_state.contacts[0].is_expired());
}

#[test]
fn test_status_indicators_priority_inactive_chat() {
    let mut app = App::new().expect("Failed to create app");

    // Add an inactive chat (read/no new messages)
    app.app_state.add_chat("dave_uid".to_string());
    app.app_state.chats[0].is_active = false;
    app.app_state.chats[0].has_pending_messages = false;

    // Verify chat is inactive
    assert!(!app.app_state.chats[0].is_active);
    assert!(!app.app_state.chats[0].has_pending_messages);
}

#[test]
fn test_contact_expiry_check() {
    use chrono::{Duration, Utc};

    // Test expired contact
    let expired_contact = Contact::new(
        "expired_uid".to_string(),
        "192.168.1.100:8080".to_string(),
        vec![1, 2, 3, 4],
        Utc::now() - Duration::hours(1),
    );
    assert!(expired_contact.is_expired(), "Contact should be expired");

    // Test valid contact
    let valid_contact = Contact::new(
        "valid_uid".to_string(),
        "192.168.1.101:8080".to_string(),
        vec![5, 6, 7, 8],
        Utc::now() + Duration::days(30),
    );
    assert!(!valid_contact.is_expired(), "Contact should not be expired");
}

#[test]
fn test_chat_with_no_matching_contact() {
    let mut app = App::new().expect("Failed to create app");

    // Add a chat without adding a corresponding contact
    app.app_state.add_chat("orphan_uid".to_string());
    app.app_state.chats[0].is_active = true;

    // Verify chat exists but no contact exists
    assert_eq!(app.app_state.chats.len(), 1);
    assert_eq!(app.app_state.contacts.len(), 0);

    // The rendering code should handle this gracefully (defaults to false for is_expired)
}

#[test]
fn test_multiple_chats_different_states() {
    use chrono::{Duration, Utc};

    let mut app = App::new().expect("Failed to create app");

    // Chat 1: Expired contact
    app.app_state.add_chat("expired_uid".to_string());
    let expired_contact = Contact::new(
        "expired_uid".to_string(),
        "192.168.1.100:8080".to_string(),
        vec![1, 2, 3, 4],
        Utc::now() - Duration::hours(1),
    );
    app.app_state.contacts.push(expired_contact);

    // Chat 2: Pending messages
    app.app_state.add_chat("pending_uid".to_string());
    app.app_state.chats[1].has_pending_messages = true;
    let pending_contact = Contact::new(
        "pending_uid".to_string(),
        "192.168.1.101:8080".to_string(),
        vec![5, 6, 7, 8],
        Utc::now() + Duration::days(30),
    );
    app.app_state.contacts.push(pending_contact);

    // Chat 3: Active (new messages)
    app.app_state.add_chat("active_uid".to_string());
    app.app_state.chats[2].is_active = true;
    let active_contact = Contact::new(
        "active_uid".to_string(),
        "192.168.1.102:8080".to_string(),
        vec![9, 10, 11, 12],
        Utc::now() + Duration::days(30),
    );
    app.app_state.contacts.push(active_contact);

    // Chat 4: Inactive (read)
    app.app_state.add_chat("inactive_uid".to_string());
    app.app_state.chats[3].is_active = false;

    // Verify all states
    assert_eq!(app.app_state.chats.len(), 4);
    assert_eq!(app.app_state.contacts.len(), 3);

    assert!(app.app_state.contacts[0].is_expired());
    assert!(!app.app_state.contacts[1].is_expired());
    assert!(app.app_state.chats[1].has_pending_messages);
    assert!(app.app_state.chats[2].is_active);
    assert!(!app.app_state.chats[3].is_active);
}

#[test]
fn test_chat_pending_flag_methods() {
    let mut app = App::new().expect("Failed to create app");
    app.app_state.add_chat("test_uid".to_string());

    // Initially no pending messages
    assert!(!app.app_state.chats[0].has_pending_messages);

    // Mark as having pending
    app.app_state.chats[0].mark_has_pending();
    assert!(app.app_state.chats[0].has_pending_messages);

    // Mark as no pending
    app.app_state.chats[0].mark_no_pending();
    assert!(!app.app_state.chats[0].has_pending_messages);
}

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
fn test_app_startup_with_no_pending_messages() {
    let app = App::new().expect("Failed to create app");

    // With no pending messages, should start on MainMenu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.startup_sync_screen.is_none());
}

#[test]
fn test_app_update_startup_sync() {
    let mut app = App::new().expect("Failed to create app");

    // Manually create a startup sync screen with messages
    app.startup_sync_screen = Some(StartupSyncScreen::new(5));
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
    app.startup_sync_screen = Some(StartupSyncScreen::new(1));
    app.current_screen = Screen::StartupSync;

    // Complete the sync
    app.complete_startup_sync();

    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.startup_sync_screen.is_none());
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
