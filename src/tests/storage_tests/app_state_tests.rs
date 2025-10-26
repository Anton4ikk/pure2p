// AppState Tests - Testing AppState struct and its methods

use crate::storage::{AppState, Chat, Contact, Message};
use chrono::{Duration, Utc};
use tempfile::NamedTempFile;
use std::collections::HashSet;

#[test]
fn test_app_state_creation() {
    let state = AppState::new();

    assert!(state.contacts.is_empty());
    assert!(state.chats.is_empty());
    assert!(state.message_queue.is_empty());
    assert_eq!(state.settings.default_contact_expiry_days, 30);
}

#[test]
fn test_app_state_save_load_json() {
    // Create temp file
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    // Create state with some data
    let mut state = AppState::new();
    state.contacts.push(Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3, 4],
        vec![99u8; 32],
        Utc::now() + Duration::days(30),
    ));
    state.chats.push(Chat::new("test_uid".to_string()));
    state.message_queue.push("msg_1".to_string());
    state.settings.enable_notifications = false;

    // Save state
    state.save(path).expect("Failed to save state");

    // Load state
    let loaded = AppState::load(path).expect("Failed to load state");

    // Verify all fields
    assert_eq!(loaded.contacts.len(), 1);
    assert_eq!(loaded.contacts[0].uid, "test_uid");
    assert_eq!(loaded.chats.len(), 1);
    assert_eq!(loaded.chats[0].contact_uid, "test_uid");
    assert_eq!(loaded.message_queue.len(), 1);
    assert_eq!(loaded.message_queue[0], "msg_1");
    assert!(!loaded.settings.enable_notifications);
}

#[test]
fn test_app_state_save_load_cbor() {
    // Create temp file
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    // Create state with some data
    let mut state = AppState::new();
    state.contacts.push(Contact::new(
        "cbor_uid".to_string(),
        "192.168.1.1:9000".to_string(),
        vec![10, 20, 30],
        vec![99u8; 32],
        Utc::now() + Duration::days(60),
    ));
    state.settings.max_message_retries = 10;

    // Save state as CBOR
    state.save_cbor(path).expect("Failed to save state as CBOR");

    // Load state from CBOR
    let loaded = AppState::load_cbor(path).expect("Failed to load state from CBOR");

    // Verify fields
    assert_eq!(loaded.contacts.len(), 1);
    assert_eq!(loaded.contacts[0].uid, "cbor_uid");
    assert_eq!(loaded.settings.max_message_retries, 10);
}

#[test]
fn test_app_state_load_nonexistent_file() {
    // Try to load from a file that doesn't exist
    let loaded = AppState::load("/tmp/nonexistent_pure2p_state.json")
        .expect("Should return empty state for nonexistent file");

    // Should return a new empty state
    assert!(loaded.contacts.is_empty());
    assert!(loaded.chats.is_empty());
    assert_eq!(loaded.settings.default_contact_expiry_days, 30);
}

#[test]
fn test_app_state_load_cbor_nonexistent_file() {
    // Try to load from a CBOR file that doesn't exist
    let loaded = AppState::load_cbor("/tmp/nonexistent_pure2p_state.cbor")
        .expect("Should return empty state for nonexistent file");

    // Should return a new empty state
    assert!(loaded.contacts.is_empty());
    assert!(loaded.chats.is_empty());
}

#[test]
fn test_app_state_with_multiple_contacts_and_chats() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    // Create state with multiple contacts and chats
    let mut state = AppState::new();

    for i in 0..5 {
        let uid = format!("uid_{}", i);
        state.contacts.push(Contact::new(
            uid.clone(),
            format!("10.0.0.{}:8080", i),
            vec![i as u8; 10],
            vec![99u8; 32],
            Utc::now() + Duration::days(30),
        ));

        let mut chat = Chat::new(uid.clone());
        let mut msg = Message::new(
            format!("msg_{}", i),
            uid.clone(),
            "self".to_string(),
            vec![i as u8; 5],
            1000 * i as i64,
        );
        msg.mark_delivered();
        chat.append_message(msg);
        chat.mark_unread();
        state.chats.push(chat);
    }

    // Save and load
    state.save(path).expect("Failed to save state");
    let loaded = AppState::load(path).expect("Failed to load state");

    // Verify
    assert_eq!(loaded.contacts.len(), 5);
    assert_eq!(loaded.chats.len(), 5);
    assert!(loaded.chats[0].is_active);
    assert_eq!(loaded.chats[4].contact_uid, "uid_4");
    assert_eq!(loaded.chats[0].messages.len(), 1);
}

#[test]
fn test_app_state_json_format_human_readable() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let mut state = AppState::new();
    state.contacts.push(Contact::new(
        "readable_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        vec![99u8; 32],
        Utc::now() + Duration::days(7),
    ));

    // Save state
    state.save(path).expect("Failed to save state");

    // Read raw file content
    let content = std::fs::read_to_string(path).expect("Failed to read file");

    // Verify it's human-readable JSON
    assert!(content.contains("readable_uid"));
    assert!(content.contains("127.0.0.1:8080"));
    assert!(content.contains("contacts"));
    assert!(content.contains("settings"));
}

#[test]
fn test_appstate_sync_pending_status() {
    let mut state = AppState::new();

    // Add some chats
    state.add_chat("alice".to_string());
    state.add_chat("bob".to_string());
    state.add_chat("charlie".to_string());

    // Create pending UIDs set
    let mut pending_uids = HashSet::new();
    pending_uids.insert("alice".to_string());
    pending_uids.insert("charlie".to_string());

    // Sync pending status
    state.sync_pending_status(&pending_uids);

    // Verify flags
    assert!(state.get_chat("alice").unwrap().has_pending_messages);
    assert!(!state.get_chat("bob").unwrap().has_pending_messages);
    assert!(state.get_chat("charlie").unwrap().has_pending_messages);
}

#[test]
fn test_appstate_sync_pending_status_empty() {
    let mut state = AppState::new();
    state.add_chat("alice".to_string());
    state.add_chat("bob".to_string());

    // Mark all as having pending initially
    state.get_chat_mut("alice").unwrap().mark_has_pending();
    state.get_chat_mut("bob").unwrap().mark_has_pending();

    // Sync with empty set
    let pending_uids = HashSet::new();
    state.sync_pending_status(&pending_uids);

    // All should be cleared
    assert!(!state.get_chat("alice").unwrap().has_pending_messages);
    assert!(!state.get_chat("bob").unwrap().has_pending_messages);
}

#[test]
fn test_appstate_get_or_create_chat() {
    let mut state = AppState::new();

    // Get or create should create new chat
    let chat = state.get_or_create_chat("new_contact");
    assert_eq!(chat.contact_uid, "new_contact");
    assert_eq!(state.chats.len(), 1);

    // Get or create should return existing chat
    let chat2 = state.get_or_create_chat("new_contact");
    assert_eq!(chat2.contact_uid, "new_contact");
    assert_eq!(state.chats.len(), 1); // Should not create duplicate
}

#[test]
fn test_appstate_get_chat() {
    let mut state = AppState::new();
    state.add_chat("alice".to_string());

    // Get existing chat
    let chat = state.get_chat("alice");
    assert!(chat.is_some());
    assert_eq!(chat.unwrap().contact_uid, "alice");

    // Get non-existent chat
    let chat = state.get_chat("bob");
    assert!(chat.is_none());
}

#[test]
fn test_appstate_get_chat_mut() {
    let mut state = AppState::new();
    state.add_chat("alice".to_string());

    // Get mutable reference and modify
    if let Some(chat) = state.get_chat_mut("alice") {
        chat.mark_has_pending();
        chat.mark_unread();
    }

    // Verify changes persisted
    let chat = state.get_chat("alice").unwrap();
    assert!(chat.has_pending_messages);
    assert!(chat.is_active);
}
