// Screen Tests - Testing screen structs and their methods

use crate::crypto::KeyPair;
use crate::storage::{generate_contact_token, parse_contact_token, Contact, Settings};
use crate::tui::screens::*;
use chrono::{Duration, Utc};

// ShareContactScreen Tests

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
fn test_token_consistency() {
    // Same keypair and IP should generate same token
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let local_ip = "192.168.1.100:8080";
    let expiry = Utc::now() + Duration::days(30);

    let token1 = generate_contact_token(local_ip, &keypair.public_key, &keypair.x25519_public, expiry);
    let token2 = generate_contact_token(local_ip, &keypair.public_key, &keypair.x25519_public, expiry);

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

    let token1 = generate_contact_token(local_ip, &keypair1.public_key, &keypair1.x25519_public, expiry);
    let token2 = generate_contact_token(local_ip, &keypair2.public_key, &keypair2.x25519_public, expiry);

    assert_ne!(
        token1, token2,
        "Different keypairs should generate different tokens"
    );
}

// ImportContactScreen Tests

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
    let token = generate_contact_token(local_ip, &keypair.public_key, &keypair.x25519_public, expiry);

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
    let token = generate_contact_token(local_ip, &keypair.public_key, &keypair.x25519_public, expiry);

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

// ChatListScreen Tests

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

// ChatViewScreen Tests

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

// SettingsScreen Tests

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

// StartupSyncScreen Tests

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

// DiagnosticsScreen Tests

#[test]
fn test_diagnostics_screen_new() {
    let screen = DiagnosticsScreen::new(8080);

    assert_eq!(screen.local_port, 8080);
    assert!(!screen.cgnat_detected);
    assert!(!screen.is_refreshing);
    assert!(screen.pcp_status.is_none());
    assert!(screen.natpmp_status.is_none());
    assert!(screen.upnp_status.is_none());
}

#[test]
fn test_diagnostics_screen_set_cgnat_detected() {
    let mut screen = DiagnosticsScreen::new(8080);
    assert!(!screen.cgnat_detected);

    screen.set_cgnat_detected(true);
    assert!(screen.cgnat_detected);

    screen.set_cgnat_detected(false);
    assert!(!screen.cgnat_detected);
}

#[test]
fn test_diagnostics_screen_update_from_connectivity_result() {
    use crate::connectivity::{ConnectivityResult, StrategyAttempt, PortMappingResult, MappingProtocol};
    use std::net::{IpAddr, Ipv4Addr};
    use chrono::Utc;

    let mut screen = DiagnosticsScreen::new(8080);

    let mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)), // CGNAT IP
        external_port: 60000,
        lifetime_secs: 3600,
        protocol: MappingProtocol::PCP,
        created_at_ms: Utc::now().timestamp_millis(),
    };

    let mut result = ConnectivityResult::new();
    result.cgnat_detected = true;
    result.pcp = StrategyAttempt::Success(mapping.clone());
    result.natpmp = StrategyAttempt::Failed("No gateway".to_string());
    result.mapping = Some(mapping);

    screen.update_from_connectivity_result(&result);

    assert!(screen.cgnat_detected, "CGNAT should be detected");
    assert!(!screen.is_refreshing, "Should not be refreshing after update");
    assert!(screen.pcp_status.is_some(), "PCP status should be set");
    assert!(screen.natpmp_status.is_some(), "NAT-PMP status should be set");

    // Verify PCP succeeded
    if let Some(Ok(pcp_mapping)) = &screen.pcp_status {
        assert_eq!(pcp_mapping.external_port, 60000);
    } else {
        panic!("Expected PCP success status");
    }

    // Verify NAT-PMP failed
    if let Some(Err(error)) = &screen.natpmp_status {
        assert!(error.contains("No gateway"));
    } else {
        panic!("Expected NAT-PMP error status");
    }
}

#[test]
fn test_diagnostics_screen_start_refresh() {
    let mut screen = DiagnosticsScreen::new(8080);
    assert!(!screen.is_refreshing);

    screen.start_refresh();
    assert!(screen.is_refreshing);
    assert!(screen.status_message.is_some());
}

// Status indicators and contact expiry tests

#[test]
fn test_status_indicators_priority_expired_contact() {
    use crate::tui::App;

    let mut app = App::new().expect("Failed to create app");

    // Add a chat
    app.app_state.add_chat("alice_uid".to_string());

    // Add an expired contact
    let contact = Contact::new(
        "alice_uid".to_string(),
        "192.168.1.100:8080".to_string(),
        vec![1, 2, 3, 4],
        vec![99u8; 32], // x25519_pubkey placeholder
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
    use crate::tui::App;

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
        vec![99u8; 32], // x25519_pubkey placeholder
        chrono::Utc::now() + chrono::Duration::days(30),
    );
    app.app_state.contacts.push(contact);

    // Verify pending messages flag is set
    assert!(app.app_state.chats[0].has_pending_messages);
    assert!(!app.app_state.contacts[0].is_expired());
}

#[test]
fn test_status_indicators_priority_active_chat() {
    use crate::tui::App;

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
        vec![99u8; 32], // x25519_pubkey placeholder
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
    use crate::tui::App;

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
    // Test expired contact
    let expired_contact = Contact::new(
        "expired_uid".to_string(),
        "192.168.1.100:8080".to_string(),
        vec![1, 2, 3, 4],
        vec![99u8; 32], // x25519_pubkey placeholder
        Utc::now() - Duration::hours(1),
    );
    assert!(expired_contact.is_expired(), "Contact should be expired");

    // Test valid contact
    let valid_contact = Contact::new(
        "valid_uid".to_string(),
        "192.168.1.101:8080".to_string(),
        vec![5, 6, 7, 8],
        vec![99u8; 32], // x25519_pubkey placeholder
        Utc::now() + Duration::days(30),
    );
    assert!(!valid_contact.is_expired(), "Contact should not be expired");
}

#[test]
fn test_chat_with_no_matching_contact() {
    use crate::tui::App;

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
    use crate::tui::App;

    let mut app = App::new().expect("Failed to create app");

    // Chat 1: Expired contact
    app.app_state.add_chat("expired_uid".to_string());
    let expired_contact = Contact::new(
        "expired_uid".to_string(),
        "192.168.1.100:8080".to_string(),
        vec![1, 2, 3, 4],
        vec![99u8; 32], // x25519_pubkey placeholder
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
        vec![99u8; 32], // x25519_pubkey placeholder
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
        vec![99u8; 32], // x25519_pubkey placeholder
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
    use crate::tui::App;

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
