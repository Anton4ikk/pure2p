// Chat Tests - Testing Chat and Message structs

use crate::storage::{Chat, Message};

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
