// ImportContactScreen Tests - Testing contact token import and parsing

use crate::crypto::KeyPair;
use crate::storage::generate_contact_token;
use crate::tui::screens::ImportContactScreen;
use chrono::{Duration, Utc};

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
    let token = generate_contact_token(local_ip, &keypair.public_key, &keypair.private_key, &keypair.x25519_public, expiry).expect("Failed to generate token");

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
    let token = generate_contact_token(local_ip, &keypair.public_key, &keypair.private_key, &keypair.x25519_public, expiry).expect("Failed to generate token");

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
fn test_paste_from_clipboard_graceful_degradation() {
    use crate::tui::clipboard::mock::MockClipboard;

    // Test that clipboard errors are handled gracefully (especially for SSH)
    let mut screen = ImportContactScreen::new();

    // Test with failing clipboard (simulates SSH environment)
    let mock_clipboard = MockClipboard::new_failing();
    screen.paste_from_clipboard_with_provider(&mut Ok(mock_clipboard));

    // Should have error message
    assert!(
        screen.status_message.is_some(),
        "Status message should be set after paste attempt"
    );

    let status = screen.status_message.as_ref().unwrap();

    // Should mention typing manually
    assert!(
        status.to_lowercase().contains("manual") || status.to_lowercase().contains("type"),
        "Error message should suggest alternative (type manually): {}",
        status
    );
    assert!(
        screen.is_error,
        "Should be in error state when clipboard unavailable"
    );
}

#[test]
fn test_paste_clipboard_error_messages_helpful() {
    use crate::tui::clipboard::mock::MockClipboard;

    // Verify error messages are user-friendly and actionable
    let mut screen = ImportContactScreen::new();

    // Test with failing clipboard
    let mock_clipboard = MockClipboard::new_failing();
    screen.paste_from_clipboard_with_provider(&mut Ok(mock_clipboard));

    if let Some(status) = &screen.status_message {
        // If there was an error, message should be user-friendly
        if screen.is_error {
            assert!(
                !status.contains("X11") && !status.contains("server connection"),
                "Error messages should be user-friendly, not technical: {}",
                status
            );
            // Should suggest alternative action
            assert!(
                status.to_lowercase().contains("manual") || status.to_lowercase().contains("type"),
                "Error message should suggest typing manually: {}",
                status
            );
        }
    }
}

#[test]
fn test_paste_from_clipboard_success() {
    use crate::tui::clipboard::mock::MockClipboard;
    use crate::tui::clipboard::ClipboardProvider;
    use crate::storage::generate_contact_token;

    // Test successful clipboard paste
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";
    let expiry = Utc::now() + Duration::days(30);
    let token = generate_contact_token(local_ip, &keypair.public_key, &keypair.private_key, &keypair.x25519_public, expiry)
        .expect("Failed to generate token");

    let mut screen = ImportContactScreen::new();

    // Set up mock clipboard with token
    let mut mock_clipboard = MockClipboard::new();
    mock_clipboard.set_text(&token).expect("Failed to set mock clipboard");
    screen.paste_from_clipboard_with_provider(&mut Ok(mock_clipboard));

    // Should have success message
    assert!(!screen.is_error, "Should not be in error state");
    assert_eq!(screen.input, token, "Input should contain pasted token");
    assert!(
        screen.status_message.as_ref().unwrap().contains("Pasted"),
        "Should have paste success message"
    );
}
