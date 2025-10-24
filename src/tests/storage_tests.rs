use crate::storage::*;
use crate::crypto::{KeyPair, UID};
use crate::Error;
use chrono::{Duration, Utc};
use serde_json;
use serde_cbor;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use tempfile::NamedTempFile;

#[test]
fn test_storage_creation() {
    let storage = Storage::new();
    assert!(storage._conn.is_none());
}

#[test]
fn test_contact_creation() {
    let uid = "a1b2c3d4e5f6".to_string();
    let ip = "192.168.1.100:8080".to_string();
    let pubkey = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let expiry = Utc::now() + Duration::days(30);

    let contact = Contact::new(uid.clone(), ip.clone(), pubkey.clone(), expiry);

    assert_eq!(contact.uid, uid);
    assert_eq!(contact.ip, ip);
    assert_eq!(contact.pubkey, pubkey);
    assert_eq!(contact.expiry, expiry);
    assert!(contact.is_active); // Should be active by default
}

#[test]
fn test_contact_is_expired_future() {
    let expiry = Utc::now() + Duration::days(30);
    let contact = Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        expiry,
    );

    assert!(!contact.is_expired(), "Contact with future expiry should not be expired");
}

#[test]
fn test_contact_is_expired_past() {
    let expiry = Utc::now() - Duration::days(1);
    let contact = Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        expiry,
    );

    assert!(contact.is_expired(), "Contact with past expiry should be expired");
}

#[test]
fn test_contact_activate() {
    let expiry = Utc::now() + Duration::days(30);
    let mut contact = Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        expiry,
    );

    // Deactivate first
    contact.deactivate();
    assert!(!contact.is_active);

    // Then activate
    contact.activate();
    assert!(contact.is_active);
}

#[test]
fn test_contact_deactivate() {
    let expiry = Utc::now() + Duration::days(30);
    let mut contact = Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        expiry,
    );

    assert!(contact.is_active); // Starts active

    contact.deactivate();
    assert!(!contact.is_active);
}

#[test]
fn test_contact_serialize_deserialize_json() {
    let expiry = Utc::now() + Duration::days(30);
    let original = Contact::new(
        "a1b2c3d4e5f6".to_string(),
        "192.168.1.100:8080".to_string(),
        vec![10, 20, 30, 40, 50],
        expiry,
    );

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("Failed to serialize to JSON");

    // Deserialize from JSON
    let deserialized: Contact = serde_json::from_str(&json).expect("Failed to deserialize from JSON");

    // Verify all fields match
    assert_eq!(deserialized.uid, original.uid);
    assert_eq!(deserialized.ip, original.ip);
    assert_eq!(deserialized.pubkey, original.pubkey);
    assert_eq!(deserialized.expiry, original.expiry);
    assert_eq!(deserialized.is_active, original.is_active);
}

#[test]
fn test_contact_serialize_deserialize_cbor() {
    let expiry = Utc::now() + Duration::days(30);
    let original = Contact::new(
        "x9y8z7w6v5u4".to_string(),
        "10.0.0.1:9000".to_string(),
        vec![100, 101, 102, 103],
        expiry,
    );

    // Serialize to CBOR
    let cbor = serde_cbor::to_vec(&original).expect("Failed to serialize to CBOR");

    // Deserialize from CBOR
    let deserialized: Contact = serde_cbor::from_slice(&cbor).expect("Failed to deserialize from CBOR");

    // Verify all fields match
    assert_eq!(deserialized.uid, original.uid);
    assert_eq!(deserialized.ip, original.ip);
    assert_eq!(deserialized.pubkey, original.pubkey);
    assert_eq!(deserialized.expiry, original.expiry);
    assert_eq!(deserialized.is_active, original.is_active);
}

#[test]
fn test_contact_clone() {
    let expiry = Utc::now() + Duration::days(30);
    let original = Contact::new(
        "clone_test".to_string(),
        "localhost:8080".to_string(),
        vec![1, 2, 3, 4],
        expiry,
    );

    let cloned = original.clone();

    assert_eq!(cloned.uid, original.uid);
    assert_eq!(cloned.ip, original.ip);
    assert_eq!(cloned.pubkey, original.pubkey);
    assert_eq!(cloned.expiry, original.expiry);
    assert_eq!(cloned.is_active, original.is_active);
}

#[test]
fn test_contact_multiple_activate_deactivate() {
    let expiry = Utc::now() + Duration::days(30);
    let mut contact = Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:8080".to_string(),
        vec![1, 2, 3],
        expiry,
    );

    // Multiple activate/deactivate cycles
    contact.deactivate();
    assert!(!contact.is_active);

    contact.activate();
    assert!(contact.is_active);

    contact.activate(); // Double activate should be idempotent
    assert!(contact.is_active);

    contact.deactivate();
    assert!(!contact.is_active);

    contact.deactivate(); // Double deactivate should be idempotent
    assert!(!contact.is_active);
}

#[test]
fn test_generate_contact_token() {
    let ip = "192.168.1.100:8080";
    let pubkey = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let expiry = Utc::now() + Duration::days(30);

    let token = generate_contact_token(ip, &pubkey, expiry);

    // Token should not be empty
    assert!(!token.is_empty());

    // Token should be valid base64 URL-safe
    assert!(token.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
}

#[test]
fn test_parse_contact_token_roundtrip() {
    let ip = "10.0.0.1:9000";
    let pubkey = vec![100, 101, 102, 103, 104, 105, 106, 107, 108, 109];
    let expiry = Utc::now() + Duration::days(7);

    // Generate token
    let token = generate_contact_token(ip, &pubkey, expiry);

    // Parse token
    let contact = parse_contact_token(&token).expect("Failed to parse valid token");

    // Verify fields
    assert_eq!(contact.ip, ip);
    assert_eq!(contact.pubkey, pubkey);
    assert_eq!(contact.expiry, expiry);
    assert!(contact.is_active); // Should be active by default

    // UID should match the one derived from pubkey
    let expected_uid = UID::from_public_key(&pubkey);
    assert_eq!(contact.uid, expected_uid.to_string());
}

#[test]
fn test_parse_contact_token_expired() {
    let ip = "127.0.0.1:8080";
    let pubkey = vec![1, 2, 3, 4, 5];
    let expiry = Utc::now() - Duration::days(1); // Expired yesterday

    // Generate token with expired timestamp
    let token = generate_contact_token(ip, &pubkey, expiry);

    // Parsing should fail due to expiry
    let result = parse_contact_token(&token);
    assert!(result.is_err());

    if let Err(Error::Storage(msg)) = result {
        assert!(msg.contains("expired"));
    } else {
        panic!("Expected Storage error with 'expired' message");
    }
}

#[test]
fn test_parse_contact_token_invalid_base64() {
    let invalid_token = "not-valid-base64!!!";

    let result = parse_contact_token(invalid_token);
    assert!(result.is_err());

    if let Err(Error::Storage(msg)) = result {
        assert!(msg.contains("Invalid base64"));
    } else {
        panic!("Expected Storage error for invalid base64");
    }
}

#[test]
fn test_parse_contact_token_invalid_cbor() {
    // Create a valid base64 string but with invalid CBOR data
    let invalid_cbor = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid CBOR
    let token = URL_SAFE_NO_PAD.encode(invalid_cbor);

    let result = parse_contact_token(&token);
    assert!(result.is_err());

    if let Err(Error::CborSerialization(msg)) = result {
        assert!(msg.contains("Invalid token data"));
    } else {
        panic!("Expected CborSerialization error for invalid CBOR");
    }
}

#[test]
fn test_contact_token_with_real_crypto_keys() {
    // Generate a real keypair
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let ip = "203.0.113.10:8080";
    let expiry = Utc::now() + Duration::days(90);

    // Generate token with real public key
    let token = generate_contact_token(ip, &keypair.public_key, expiry);

    // Parse token
    let contact = parse_contact_token(&token).expect("Failed to parse token with real keys");

    // Verify fields
    assert_eq!(contact.ip, ip);
    assert_eq!(contact.pubkey, keypair.public_key);
    assert_eq!(contact.uid, keypair.uid.to_string());
    assert!(contact.is_active);
}

#[test]
fn test_contact_token_different_inputs_different_tokens() {
    let expiry = Utc::now() + Duration::days(30);

    let token1 = generate_contact_token("192.168.1.1:8080", &[1, 2, 3], expiry);
    let token2 = generate_contact_token("192.168.1.2:8080", &[1, 2, 3], expiry);
    let token3 = generate_contact_token("192.168.1.1:8080", &[4, 5, 6], expiry);

    // Different IPs should produce different tokens
    assert_ne!(token1, token2);

    // Different pubkeys should produce different tokens
    assert_ne!(token1, token3);
}

#[test]
fn test_contact_token_deterministic() {
    let ip = "10.20.30.40:5000";
    let pubkey = vec![10, 20, 30, 40, 50];
    let expiry = Utc::now() + Duration::days(15);

    // Generate token twice with same inputs
    let token1 = generate_contact_token(ip, &pubkey, expiry);
    let token2 = generate_contact_token(ip, &pubkey, expiry);

    // Should produce identical tokens
    assert_eq!(token1, token2);
}

#[test]
fn test_chat_creation() {
    let chat = Chat::new("test_uid_123".to_string());

    assert_eq!(chat.contact_uid, "test_uid_123");
    assert!(chat.messages.is_empty());
    assert!(!chat.is_active);
}

#[test]
fn test_chat_append_message() {
    let mut chat = Chat::new("uid_456".to_string());

    let msg1 = Message {
        id: "msg_1".to_string(),
        sender: "sender_1".to_string(),
        recipient: "uid_456".to_string(),
        content: vec![1, 2, 3],
        timestamp: 1000,
        delivered: false,
    };

    chat.append_message(msg1);
    assert_eq!(chat.messages.len(), 1);
    assert_eq!(chat.messages[0].id, "msg_1");

    let msg2 = Message {
        id: "msg_2".to_string(),
        sender: "sender_2".to_string(),
        recipient: "uid_456".to_string(),
        content: vec![4, 5, 6],
        timestamp: 2000,
        delivered: true,
    };

    chat.append_message(msg2);
    assert_eq!(chat.messages.len(), 2);
    assert_eq!(chat.messages[1].timestamp, 2000);
}

#[test]
fn test_chat_active_management() {
    let mut chat = Chat::new("uid_789".to_string());

    // Initially not active
    assert!(!chat.is_active);

    // Mark as unread (active)
    chat.mark_unread();
    assert!(chat.is_active);

    // Mark as read (inactive)
    chat.mark_read();
    assert!(!chat.is_active);

    // Can mark unread multiple times
    chat.mark_unread();
    chat.mark_unread();
    assert!(chat.is_active);
}

#[test]
fn test_settings_default() {
    let settings = Settings::default();

    assert_eq!(settings.default_contact_expiry_days, 30);
    assert!(!settings.auto_accept_contacts);
    assert_eq!(settings.max_message_retries, 5);
    assert_eq!(settings.retry_base_delay_ms, 1000);
    assert!(settings.enable_notifications);
    assert_eq!(settings.global_retry_interval_ms, 600_000); // 10 minutes
    assert_eq!(settings.retry_interval_minutes, 10);
    assert_eq!(settings.storage_path, "./data");
}

#[test]
fn test_settings_global_retry_interval() {
    let mut settings = Settings::default();

    // Default should be 10 minutes (600,000 ms)
    assert_eq!(settings.get_global_retry_interval_ms(), 600_000);

    // Update to 5 minutes
    settings.set_global_retry_interval_ms(300_000);
    assert_eq!(settings.get_global_retry_interval_ms(), 300_000);

    // Update to 30 minutes
    settings.set_global_retry_interval_ms(1_800_000);
    assert_eq!(settings.get_global_retry_interval_ms(), 1_800_000);
}

#[test]
fn test_settings_runtime_update() {
    let mut settings = Settings::default();

    // Change multiple settings at runtime
    settings.set_global_retry_interval_ms(120_000); // 2 minutes
    settings.max_message_retries = 10;
    settings.enable_notifications = false;

    assert_eq!(settings.global_retry_interval_ms, 120_000);
    assert_eq!(settings.max_message_retries, 10);
    assert!(!settings.enable_notifications);
}

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
            Utc::now() + Duration::days(30),
        ));

        let mut chat = Chat::new(uid.clone());
        let msg = Message {
            id: format!("msg_{}", i),
            sender: uid.clone(),
            recipient: "self".to_string(),
            content: vec![i as u8; 5],
            timestamp: 1000 * i as i64,
            delivered: true,
        };
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
fn test_settings_serialization() {
    let mut settings = Settings::default();
    settings.default_contact_expiry_days = 90;
    settings.auto_accept_contacts = true;

    // Serialize to JSON
    let json = serde_json::to_string(&settings).expect("Failed to serialize");

    // Deserialize
    let loaded: Settings = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(loaded.default_contact_expiry_days, 90);
    assert!(loaded.auto_accept_contacts);
}

#[test]
fn test_message_serialization() {
    let msg = Message {
        id: "test_msg_123".to_string(),
        sender: "sender_uid".to_string(),
        recipient: "recipient_uid".to_string(),
        content: vec![10, 20, 30, 40, 50],
        timestamp: 1234567890,
        delivered: true,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&msg).expect("Failed to serialize message");

    // Deserialize
    let loaded: Message = serde_json::from_str(&json).expect("Failed to deserialize message");

    assert_eq!(loaded.id, "test_msg_123");
    assert_eq!(loaded.sender, "sender_uid");
    assert_eq!(loaded.recipient, "recipient_uid");
    assert_eq!(loaded.content, vec![10, 20, 30, 40, 50]);
    assert_eq!(loaded.timestamp, 1234567890);
    assert!(loaded.delivered);
}

#[test]
fn test_chat_with_messages_serialization() {
    let mut chat = Chat::new("contact_123".to_string());

    // Add multiple messages
    for i in 0..3 {
        let msg = Message {
            id: format!("msg_{}", i),
            sender: "sender".to_string(),
            recipient: "contact_123".to_string(),
            content: vec![i as u8; 10],
            timestamp: 1000 * i as i64,
            delivered: i % 2 == 0,
        };
        chat.append_message(msg);
    }
    chat.mark_unread();

    // Serialize to JSON
    let json = serde_json::to_string(&chat).expect("Failed to serialize chat");

    // Deserialize
    let loaded: Chat = serde_json::from_str(&json).expect("Failed to deserialize chat");

    assert_eq!(loaded.contact_uid, "contact_123");
    assert_eq!(loaded.messages.len(), 3);
    assert!(loaded.is_active);
    assert_eq!(loaded.messages[0].id, "msg_0");
    assert_eq!(loaded.messages[2].timestamp, 2000);
}

#[test]
fn test_chat_pending_messages_flag() {
    let mut chat = Chat::new("contact_uid".to_string());

    // Initially, no pending messages
    assert!(!chat.has_pending());
    assert!(!chat.has_pending_messages);

    // Mark as having pending messages
    chat.mark_has_pending();
    assert!(chat.has_pending());
    assert!(chat.has_pending_messages);

    // Mark as no pending messages
    chat.mark_no_pending();
    assert!(!chat.has_pending());
    assert!(!chat.has_pending_messages);
}

#[test]
fn test_chat_pending_independent_of_active() {
    let mut chat = Chat::new("contact_uid".to_string());

    // Set both flags independently
    chat.mark_unread();
    chat.mark_has_pending();

    assert!(chat.is_active);
    assert!(chat.has_pending_messages);

    // Clear one flag
    chat.mark_read();

    assert!(!chat.is_active);
    assert!(chat.has_pending_messages); // Should remain true

    // Clear the other flag
    chat.mark_no_pending();

    assert!(!chat.is_active);
    assert!(!chat.has_pending_messages);
}

#[test]
fn test_appstate_sync_pending_status() {
    use std::collections::HashSet;

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
    use std::collections::HashSet;

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

#[test]
fn test_chat_serialization_with_pending_flag() {
    let mut chat = Chat::new("contact_123".to_string());
    chat.mark_has_pending();
    chat.mark_unread();

    // Add a message
    let msg = Message {
        id: "msg_1".to_string(),
        sender: "sender".to_string(),
        recipient: "contact_123".to_string(),
        content: vec![1, 2, 3],
        timestamp: 1000,
        delivered: false,
    };
    chat.append_message(msg);

    // Serialize to JSON
    let json = serde_json::to_string(&chat).expect("Failed to serialize");

    // Deserialize
    let loaded: Chat = serde_json::from_str(&json).expect("Failed to deserialize");

    // Verify all fields including pending flag
    assert_eq!(loaded.contact_uid, "contact_123");
    assert!(loaded.is_active);
    assert!(loaded.has_pending_messages);
    assert_eq!(loaded.messages.len(), 1);
}

#[test]
fn test_settings_save_and_load() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    // Create and save settings
    let mut settings = Settings::default();
    settings.retry_interval_minutes = 15;
    settings.global_retry_interval_ms = 15 * 60 * 1000; // Keep in sync
    settings.storage_path = "/custom/path".to_string();

    settings.save(path).expect("Failed to save settings");

    // Load settings
    let loaded = Settings::load(path).expect("Failed to load settings");

    assert_eq!(loaded.retry_interval_minutes, 15);
    assert_eq!(loaded.global_retry_interval_ms, 15 * 60 * 1000);
    assert_eq!(loaded.storage_path, "/custom/path");
    assert_eq!(loaded.default_contact_expiry_days, 30);
}

#[test]
fn test_settings_load_nonexistent() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path().join("nonexistent.json");

    // Load from nonexistent file should return defaults
    let settings = Settings::load(&path).expect("Failed to load settings");

    assert_eq!(settings.retry_interval_minutes, 10);
    assert_eq!(settings.storage_path, "./data");
}

#[test]
fn test_settings_load_empty_file() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    // File exists but is empty - should return defaults
    let settings = Settings::load(path).expect("Failed to load settings");

    assert_eq!(settings.retry_interval_minutes, 10);
    assert_eq!(settings.storage_path, "./data");
    assert_eq!(settings.max_message_retries, 5);
}

#[test]
fn test_settings_update_retry_interval() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    // Create settings and update retry interval
    let mut settings = Settings::default();
    settings.update_retry_interval(20, path).expect("Failed to update");

    // Verify values are updated
    assert_eq!(settings.retry_interval_minutes, 20);
    assert_eq!(settings.global_retry_interval_ms, 20 * 60 * 1000);

    // Verify auto-save worked
    let loaded = Settings::load(path).expect("Failed to load");
    assert_eq!(loaded.retry_interval_minutes, 20);
    assert_eq!(loaded.global_retry_interval_ms, 20 * 60 * 1000);
}

#[test]
fn test_settings_sync_retry_interval() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    // Create settings with mismatched values (shouldn't happen in practice)
    let mut settings = Settings::default();
    settings.global_retry_interval_ms = 900_000; // 15 minutes
    settings.retry_interval_minutes = 10; // Wrong value

    // Save and reload should sync the values
    settings.save(path).expect("Failed to save");
    let loaded = Settings::load(path).expect("Failed to load");

    // Minutes should be synced to match milliseconds
    assert_eq!(loaded.retry_interval_minutes, 15);
    assert_eq!(loaded.global_retry_interval_ms, 900_000);
}

#[test]
fn test_settings_set_global_retry_interval_ms() {
    let mut settings = Settings::default();

    // Set milliseconds directly
    settings.set_global_retry_interval_ms(1_800_000); // 30 minutes

    // Both values should be updated
    assert_eq!(settings.global_retry_interval_ms, 1_800_000);
    assert_eq!(settings.retry_interval_minutes, 30);
}

#[test]
fn test_settings_get_retry_intervals() {
    let settings = Settings::default();

    assert_eq!(settings.get_retry_interval_minutes(), 10);
    assert_eq!(settings.get_global_retry_interval_ms(), 600_000);
}

#[test]
fn test_settings_json_format() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let settings = Settings::default();
    settings.save(path).expect("Failed to save");

    // Read the JSON file
    let json = std::fs::read_to_string(path).expect("Failed to read file");

    // Verify JSON contains expected fields
    assert!(json.contains("retry_interval_minutes"));
    assert!(json.contains("storage_path"));
    assert!(json.contains("global_retry_interval_ms"));
    assert!(json.contains("\"./data\"")); // storage_path default

    // Verify the JSON can be deserialized
    let parsed: Settings = serde_json::from_str(&json).expect("Failed to parse JSON");
    assert_eq!(parsed.retry_interval_minutes, 10);
}

#[test]
fn test_settings_create_parent_directory() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path().join("subdir").join("settings.json");

    // Parent directory doesn't exist yet
    assert!(!path.parent().unwrap().exists());

    let settings = Settings::default();
    settings.save(&path).expect("Failed to save");

    // Parent directory should be created
    assert!(path.parent().unwrap().exists());
    assert!(path.exists());
}

#[tokio::test]
async fn test_settings_manager_new() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    // Create manager
    let manager = SettingsManager::new(path).await.expect("Failed to create manager");

    // Should have default values
    assert_eq!(manager.get_retry_interval_minutes().await, 10);
    assert_eq!(manager.get_storage_path().await, "./data");
}

#[tokio::test]
async fn test_settings_manager_set_retry_interval() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let manager = SettingsManager::new(path).await.expect("Failed to create manager");

    // Update retry interval
    manager.set_retry_interval_minutes(20).await.expect("Failed to set");

    // Verify updated
    assert_eq!(manager.get_retry_interval_minutes().await, 20);

    // Verify persisted
    let loaded = Settings::load(path).expect("Failed to load");
    assert_eq!(loaded.retry_interval_minutes, 20);
    assert_eq!(loaded.global_retry_interval_ms, 20 * 60 * 1000);
}

#[tokio::test]
async fn test_settings_manager_set_storage_path() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let manager = SettingsManager::new(path).await.expect("Failed to create manager");

    // Update storage path
    manager.set_storage_path("/custom/storage".to_string()).await.expect("Failed to set");

    // Verify updated
    assert_eq!(manager.get_storage_path().await, "/custom/storage");

    // Verify persisted
    let loaded = Settings::load(path).expect("Failed to load");
    assert_eq!(loaded.storage_path, "/custom/storage");
}

#[tokio::test]
async fn test_settings_manager_notifications() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let manager = SettingsManager::new(path).await.expect("Failed to create manager");

    // Default should be enabled
    assert!(manager.get_notifications_enabled().await);

    // Disable
    manager.set_notifications_enabled(false).await.expect("Failed to set");
    assert!(!manager.get_notifications_enabled().await);

    // Enable
    manager.set_notifications_enabled(true).await.expect("Failed to set");
    assert!(manager.get_notifications_enabled().await);
}

#[tokio::test]
async fn test_settings_manager_max_retries() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let manager = SettingsManager::new(path).await.expect("Failed to create manager");

    // Update max retries
    manager.set_max_message_retries(10).await.expect("Failed to set");

    // Verify
    assert_eq!(manager.get_max_message_retries().await, 10);
}

#[tokio::test]
async fn test_settings_manager_contact_expiry() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let manager = SettingsManager::new(path).await.expect("Failed to create manager");

    // Update contact expiry
    manager.set_default_contact_expiry_days(60).await.expect("Failed to set");

    // Verify
    assert_eq!(manager.get_default_contact_expiry_days().await, 60);
}

#[tokio::test]
async fn test_settings_manager_get_all() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let manager = SettingsManager::new(path).await.expect("Failed to create manager");

    // Get all settings
    let settings = manager.get_all().await;

    assert_eq!(settings.retry_interval_minutes, 10);
    assert_eq!(settings.storage_path, "./data");
    assert_eq!(settings.max_message_retries, 5);
}

#[tokio::test]
async fn test_settings_manager_update_multiple() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let manager = SettingsManager::new(path).await.expect("Failed to create manager");

    // Update multiple settings at once
    manager.update(|s| {
        s.retry_interval_minutes = 25;
        s.global_retry_interval_ms = 25 * 60 * 1000;
        s.storage_path = "/new/path".to_string();
        s.max_message_retries = 8;
    }).await.expect("Failed to update");

    // Verify all updated
    assert_eq!(manager.get_retry_interval_minutes().await, 25);
    assert_eq!(manager.get_storage_path().await, "/new/path");
    assert_eq!(manager.get_max_message_retries().await, 8);

    // Verify persisted
    let loaded = Settings::load(path).expect("Failed to load");
    assert_eq!(loaded.retry_interval_minutes, 25);
    assert_eq!(loaded.storage_path, "/new/path");
    assert_eq!(loaded.max_message_retries, 8);
}

#[tokio::test]
async fn test_settings_manager_reload() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let manager = SettingsManager::new(path).await.expect("Failed to create manager");

    // Update via manager
    manager.set_retry_interval_minutes(15).await.expect("Failed to set");

    // Modify file directly
    let mut settings = Settings::load(path).expect("Failed to load");
    settings.retry_interval_minutes = 30;
    settings.global_retry_interval_ms = 30 * 60 * 1000;
    settings.save(path).expect("Failed to save");

    // Manager still has old value
    assert_eq!(manager.get_retry_interval_minutes().await, 15);

    // Reload from disk
    manager.reload().await.expect("Failed to reload");

    // Now has new value
    assert_eq!(manager.get_retry_interval_minutes().await, 30);
}

#[tokio::test]
async fn test_settings_manager_concurrent_access() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let manager = SettingsManager::new(path).await.expect("Failed to create manager");

    // Clone for concurrent access
    let manager1 = manager.clone();
    let manager2 = manager.clone();
    let manager3 = manager.clone();

    // Spawn concurrent tasks
    let task1 = tokio::spawn(async move {
        manager1.set_retry_interval_minutes(15).await
    });

    let task2 = tokio::spawn(async move {
        manager2.set_storage_path("/path1".to_string()).await
    });

    let task3 = tokio::spawn(async move {
        manager3.set_notifications_enabled(false).await
    });

    // Wait for all tasks
    task1.await.unwrap().expect("Task 1 failed");
    task2.await.unwrap().expect("Task 2 failed");
    task3.await.unwrap().expect("Task 3 failed");

    // Verify all changes applied
    assert_eq!(manager.get_retry_interval_minutes().await, 15);
    assert_eq!(manager.get_storage_path().await, "/path1");
    assert!(!manager.get_notifications_enabled().await);
}

#[tokio::test]
async fn test_settings_manager_clone() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path();

    let manager = SettingsManager::new(path).await.expect("Failed to create manager");
    let cloned = manager.clone();

    // Update via original
    manager.set_retry_interval_minutes(20).await.expect("Failed to set");

    // Clone sees the update (shared state)
    assert_eq!(cloned.get_retry_interval_minutes().await, 20);

    // Update via clone
    cloned.set_storage_path("/clone/path".to_string()).await.expect("Failed to set");

    // Original sees the update
    assert_eq!(manager.get_storage_path().await, "/clone/path");
}
