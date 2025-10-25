// Status Indicators Tests - Testing chat list status badges and contact expiry

use crate::storage::Contact;
use crate::tui::App;
use chrono::{Duration, Utc};

#[test]
fn test_status_indicators_priority_expired_contact() {
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
