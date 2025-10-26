// Chat Tests - Testing Chat and Message structs

use crate::storage::{Chat, DeliveryStatus, Message};

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

    let msg1 = Message::new(
        "msg_1".to_string(),
        "sender_1".to_string(),
        "uid_456".to_string(),
        vec![1, 2, 3],
        1000,
    );

    chat.append_message(msg1);
    assert_eq!(chat.messages.len(), 1);
    assert_eq!(chat.messages[0].id, "msg_1");

    let mut msg2 = Message::new(
        "msg_2".to_string(),
        "sender_2".to_string(),
        "uid_456".to_string(),
        vec![4, 5, 6],
        2000,
    );
    msg2.mark_delivered();

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
    let mut msg = Message::new(
        "test_msg_123".to_string(),
        "sender_uid".to_string(),
        "recipient_uid".to_string(),
        vec![10, 20, 30, 40, 50],
        1234567890,
    );
    msg.mark_delivered();

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
        let mut msg = Message::new(
            format!("msg_{}", i),
            "sender".to_string(),
            "contact_123".to_string(),
            vec![i as u8; 10],
            1000 * i as i64,
        );
        if i % 2 == 0 {
            msg.mark_delivered();
        }
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
    let msg = Message::new(
        "msg_1".to_string(),
        "sender".to_string(),
        "contact_123".to_string(),
        vec![1, 2, 3],
        1000,
    );
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

// Delivery Status Tests

#[test]
fn test_message_new() {
    let msg = Message::new(
        "msg_id".to_string(),
        "sender_uid".to_string(),
        "recipient_uid".to_string(),
        vec![1, 2, 3],
        1000,
    );

    assert_eq!(msg.id, "msg_id");
    assert_eq!(msg.sender, "sender_uid");
    assert_eq!(msg.recipient, "recipient_uid");
    assert_eq!(msg.content, vec![1, 2, 3]);
    assert_eq!(msg.timestamp, 1000);
    assert!(!msg.delivered);
    assert_eq!(msg.delivery_status, DeliveryStatus::Sent);
    assert_eq!(msg.next_retry_at, None);
    assert_eq!(msg.attempts, 0);
}

#[test]
fn test_message_mark_delivered() {
    let mut msg = Message::new(
        "msg_id".to_string(),
        "sender".to_string(),
        "recipient".to_string(),
        vec![],
        1000,
    );

    msg.mark_delivered();

    assert!(msg.delivered);
    assert_eq!(msg.delivery_status, DeliveryStatus::Delivered);
    assert_eq!(msg.next_retry_at, None);
}

#[test]
fn test_message_mark_pending() {
    let mut msg = Message::new(
        "msg_id".to_string(),
        "sender".to_string(),
        "recipient".to_string(),
        vec![],
        1000,
    );

    let retry_time = 5000; // 5 seconds from now
    msg.mark_pending(retry_time);

    assert_eq!(msg.delivery_status, DeliveryStatus::Pending);
    assert_eq!(msg.next_retry_at, Some(retry_time));
    assert_eq!(msg.attempts, 1);

    // Mark pending again
    msg.mark_pending(10000);
    assert_eq!(msg.attempts, 2);
}

#[test]
fn test_message_mark_failed() {
    let mut msg = Message::new(
        "msg_id".to_string(),
        "sender".to_string(),
        "recipient".to_string(),
        vec![],
        1000,
    );

    msg.mark_failed();

    assert_eq!(msg.delivery_status, DeliveryStatus::Failed);
    assert_eq!(msg.next_retry_at, None);
}

#[test]
fn test_message_status_indicator() {
    let mut msg = Message::new("id".to_string(), "s".to_string(), "r".to_string(), vec![], 1000);

    assert_eq!(msg.status_indicator(), "✓");

    msg.mark_delivered();
    assert_eq!(msg.status_indicator(), "✓✓");

    msg.delivery_status = DeliveryStatus::Pending;
    assert_eq!(msg.status_indicator(), "↻");

    msg.mark_failed();
    assert_eq!(msg.status_indicator(), "✗");
}

#[test]
fn test_message_status_text() {
    let mut msg = Message::new("id".to_string(), "s".to_string(), "r".to_string(), vec![], 1000);

    assert_eq!(msg.status_text(), "sent");

    msg.mark_delivered();
    assert_eq!(msg.status_text(), "delivered");

    msg.mark_failed();
    assert_eq!(msg.status_text(), "failed");
}

#[test]
fn test_message_status_text_with_retry_countdown() {
    let mut msg = Message::new("id".to_string(), "s".to_string(), "r".to_string(), vec![], 1000);

    // Set retry time to 5 minutes from now
    let now = chrono::Utc::now().timestamp_millis();
    let retry_at = now + (5 * 60 * 1000); // 5 minutes
    msg.mark_pending(retry_at);

    let status = msg.status_text();
    // Should contain "retry in" and time
    assert!(status.contains("retry in"));
    assert!(status.contains("m") || status.contains("s"));
}

#[test]
fn test_message_time_until_retry() {
    let msg = Message::new("id".to_string(), "s".to_string(), "r".to_string(), vec![], 1000);

    // No retry set
    assert_eq!(msg.time_until_retry(), None);

    let mut msg_pending = Message::new("id".to_string(), "s".to_string(), "r".to_string(), vec![], 1000);
    let now = chrono::Utc::now().timestamp_millis();
    let retry_at = now + 10000; // 10 seconds from now
    msg_pending.mark_pending(retry_at);

    let time_left = msg_pending.time_until_retry();
    assert!(time_left.is_some());
    let seconds = time_left.unwrap();
    assert!(seconds >= 9 && seconds <= 11); // Should be around 10 seconds
}

#[test]
fn test_message_delivery_status_serialization() {
    let mut msg = Message::new("id".to_string(), "s".to_string(), "r".to_string(), vec![1, 2, 3], 1000);
    msg.mark_pending(5000);
    msg.attempts = 3;

    // Serialize to JSON
    let json = serde_json::to_string(&msg).expect("Failed to serialize");

    // Deserialize
    let loaded: Message = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(loaded.delivery_status, DeliveryStatus::Pending);
    assert_eq!(loaded.next_retry_at, Some(5000));
    assert_eq!(loaded.attempts, 3);
}

#[test]
fn test_message_backward_compatibility() {
    // Old format message (without new fields)
    let old_json = r#"{
        "id": "msg_1",
        "sender": "sender_uid",
        "recipient": "recipient_uid",
        "content": [1, 2, 3],
        "timestamp": 1000,
        "delivered": true
    }"#;

    // Should deserialize with default values for new fields
    let msg: Message = serde_json::from_str(old_json).expect("Failed to deserialize old format");

    assert_eq!(msg.id, "msg_1");
    assert!(msg.delivered);
    assert_eq!(msg.delivery_status, DeliveryStatus::Sent); // Default
    assert_eq!(msg.next_retry_at, None);
    assert_eq!(msg.attempts, 0);
}
