// ShareContactScreen Tests - Testing contact token generation and sharing

use crate::crypto::KeyPair;
use crate::storage::{generate_contact_token, parse_contact_token};
use crate::tui::screens::ShareContactScreen;
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

    // Verify default expiry is approximately 1 day
    let expiry_duration = screen.expiry.signed_duration_since(Utc::now());
    assert!(
        expiry_duration.num_hours() >= 23 && expiry_duration.num_hours() <= 24,
        "Default expiry should be approximately 1 day (24 hours), got {} hours",
        expiry_duration.num_hours()
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
fn test_token_consistency() {
    // Same keypair and IP should generate same token
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";
    let expiry = Utc::now() + Duration::days(30);

    let token1 = generate_contact_token(local_ip, &keypair.public_key, &keypair.private_key, &keypair.x25519_public, expiry).expect("Failed to generate token 1");
    let token2 = generate_contact_token(local_ip, &keypair.public_key, &keypair.private_key, &keypair.x25519_public, expiry).expect("Failed to generate token 2");

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

    let token1 = generate_contact_token(local_ip, &keypair1.public_key, &keypair1.private_key, &keypair1.x25519_public, expiry).expect("Failed to generate token 1");
    let token2 = generate_contact_token(local_ip, &keypair2.public_key, &keypair2.private_key, &keypair2.x25519_public, expiry).expect("Failed to generate token 2");

    assert_ne!(
        token1, token2,
        "Different keypairs should generate different tokens"
    );
}

#[test]
fn test_copy_to_clipboard_graceful_degradation() {
    use crate::tui::clipboard::mock::MockClipboard;

    // Test that clipboard errors are handled gracefully (especially for SSH)
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";

    let mut screen = ShareContactScreen::new(&keypair, local_ip);

    // Test with failing clipboard (simulates SSH environment)
    let mock_clipboard = MockClipboard::new_failing();
    screen.copy_to_clipboard_with_provider(&mut Ok(mock_clipboard));

    // Should have error message
    assert!(
        screen.status_message.is_some(),
        "Status message should be set after copy attempt"
    );

    let status = screen.status_message.as_ref().unwrap();

    // Should mention using 's' to save to file
    assert!(
        status.to_lowercase().contains("save") || status.contains("'s'"),
        "Error message should suggest alternative (save to file): {}",
        status
    );
}

#[test]
fn test_clipboard_error_messages_helpful() {
    use crate::tui::clipboard::mock::MockClipboard;

    // Verify error messages are user-friendly and actionable
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";

    let mut screen = ShareContactScreen::new(&keypair, local_ip);

    // Test with failing clipboard
    let mock_clipboard = MockClipboard::new_failing();
    screen.copy_to_clipboard_with_provider(&mut Ok(mock_clipboard));

    if let Some(status) = &screen.status_message {
        // Messages should not contain technical jargon like "X11 server connection"
        // Instead, should be user-friendly like "Clipboard not available over SSH"
        if status.contains("error") || status.contains("failed") || status.contains("not available") {
            assert!(
                !status.contains("X11") && !status.contains("server connection"),
                "Error messages should be user-friendly, not technical: {}",
                status
            );
        }
    }
}

#[test]
fn test_copy_to_clipboard_success() {
    use crate::tui::clipboard::mock::MockClipboard;

    // Test successful clipboard copy
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";

    let mut screen = ShareContactScreen::new(&keypair, local_ip);
    let token = screen.token.clone();

    // Test with working mock clipboard
    let mock_clipboard = MockClipboard::new();
    screen.copy_to_clipboard_with_provider(&mut Ok(mock_clipboard.clone()));

    // Should have success message
    assert_eq!(
        screen.status_message,
        Some("Copied to clipboard!".to_string())
    );

    // Verify token was actually copied
    assert_eq!(mock_clipboard.get_content(), Some(token));
}
