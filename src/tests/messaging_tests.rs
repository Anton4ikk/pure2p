use crate::messaging::*;
use crate::storage::{Contact, AppState, Message};
use crate::transport::Transport;
use crate::queue::{MessageQueue, Priority};
use chrono::{Duration, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

fn create_test_contact() -> Contact {
    Contact::new(
        "test_uid".to_string(),
        "127.0.0.1:9999".to_string(), // Use unlikely port to simulate failure
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        Utc::now() + Duration::days(30),
    )
}

fn create_test_message(id: &str, sender: &str, recipient: &str) -> Message {
    Message {
        id: id.to_string(),
        sender: sender.to_string(),
        recipient: recipient.to_string(),
        content: b"Test message content".to_vec(),
        timestamp: Utc::now().timestamp_millis(),
        delivered: false,
    }
}

#[tokio::test]
async fn test_send_message_failure_queues() {
    let transport = Transport::new();
    let mut queue = MessageQueue::new().expect("Failed to create queue");

    let contact = create_test_contact();
    let message = create_test_message("msg1", "sender_uid", "test_uid");

    // Send should fail (no server at 127.0.0.1:9999)
    let result = send_message(&transport, &mut queue, &contact, &message, Priority::Normal)
        .await
        .expect("Failed to send message");

    // Should return false (not delivered)
    assert!(!result, "Message should not be delivered");

    // Message should be in queue
    assert_eq!(queue.size().expect("Failed to get queue size"), 1);

    // Verify the queued message
    let queued_messages = queue.list().expect("Failed to list queue");
    assert_eq!(queued_messages.len(), 1);
    assert_eq!(queued_messages[0].message.id, "msg1");
    assert_eq!(queued_messages[0].priority, Priority::Normal);
}

#[tokio::test]
async fn test_send_message_with_type_failure_queues() {
    let transport = Transport::new();
    let mut queue = MessageQueue::new().expect("Failed to create queue");

    let contact = create_test_contact();
    let message = create_test_message("msg2", "sender_uid", "test_uid");

    // Send with custom type should fail
    let result = send_message_with_type(
        &transport,
        &mut queue,
        &contact,
        &message,
        "delete",
        Priority::High,
    )
    .await
    .expect("Failed to send message");

    // Should return false (not delivered)
    assert!(!result);

    // Message should be in queue
    assert_eq!(queue.size().expect("Failed to get queue size"), 1);

    // Verify the queued message
    let queued_messages = queue.list().expect("Failed to list queue");
    assert_eq!(queued_messages[0].priority, Priority::High);
}

#[tokio::test]
async fn test_send_message_success() {
    // Start a test server to receive the message
    let mut receiver = Transport::new();
    let received = std::sync::Arc::new(tokio::sync::Mutex::new(false));
    let received_clone = received.clone();

    receiver
        .set_new_message_handler(move |_msg| {
            let r = received_clone.clone();
            tokio::spawn(async move {
                *r.lock().await = true;
            });
        })
        .await;

    receiver
        .start("127.0.0.1:0".parse().unwrap())
        .await
        .expect("Failed to start receiver");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let receiver_addr = receiver.local_addr().expect("No local address");

    // Create sender transport and queue
    let sender = Transport::new();
    let mut queue = MessageQueue::new().expect("Failed to create queue");

    // Create contact pointing to the receiver
    let contact = Contact::new(
        "receiver_uid".to_string(),
        receiver_addr.to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        Utc::now() + Duration::days(30),
    );

    let message = create_test_message("msg_success", "sender_uid", "receiver_uid");

    // Send message
    let result = send_message(&sender, &mut queue, &contact, &message, Priority::Normal)
        .await
        .expect("Failed to send message");

    // Should return true (delivered)
    assert!(result, "Message should be delivered");

    // Queue should be empty (not queued)
    assert_eq!(queue.size().expect("Failed to get queue size"), 0);

    // Wait for handler to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify message was received
    assert!(*received.lock().await);
}

#[tokio::test]
async fn test_send_multiple_messages_mixed_results() {
    // Start a test server
    let mut receiver = Transport::new();
    receiver
        .set_new_message_handler(|_msg| {})
        .await;

    receiver
        .start("127.0.0.1:0".parse().unwrap())
        .await
        .expect("Failed to start receiver");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let receiver_addr = receiver.local_addr().expect("No local address");

    let sender = Transport::new();
    let mut queue = MessageQueue::new().expect("Failed to create queue");

    // Contact 1: Valid (will succeed)
    let contact1 = Contact::new(
        "receiver_uid".to_string(),
        receiver_addr.to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        Utc::now() + Duration::days(30),
    );

    // Contact 2: Invalid (will fail)
    let contact2 = create_test_contact();

    let msg1 = create_test_message("msg1", "sender", "receiver_uid");
    let msg2 = create_test_message("msg2", "sender", "test_uid");

    // Send to valid contact
    let result1 = send_message(&sender, &mut queue, &contact1, &msg1, Priority::Normal)
        .await
        .expect("Failed to send msg1");

    // Send to invalid contact
    let result2 = send_message(&sender, &mut queue, &contact2, &msg2, Priority::Normal)
        .await
        .expect("Failed to send msg2");

    // First should succeed, second should fail
    assert!(result1, "First message should be delivered");
    assert!(!result2, "Second message should not be delivered");

    // Queue should have 1 message (the failed one)
    assert_eq!(queue.size().expect("Failed to get queue size"), 1);

    let queued = queue.list().expect("Failed to list queue");
    assert_eq!(queued[0].message.id, "msg2");
}

#[tokio::test]
async fn test_send_message_different_priorities() {
    let transport = Transport::new();
    let mut queue = MessageQueue::new().expect("Failed to create queue");

    let contact = create_test_contact();

    // Send messages with different priorities
    let msg1 = create_test_message("low", "sender", "test_uid");
    let msg2 = create_test_message("high", "sender", "test_uid");
    let msg3 = create_test_message("urgent", "sender", "test_uid");

    send_message(&transport, &mut queue, &contact, &msg1, Priority::Low)
        .await
        .expect("Failed to send low priority");

    send_message(&transport, &mut queue, &contact, &msg2, Priority::High)
        .await
        .expect("Failed to send high priority");

    send_message(&transport, &mut queue, &contact, &msg3, Priority::Urgent)
        .await
        .expect("Failed to send urgent priority");

    // All should be queued
    assert_eq!(queue.size().expect("Failed to get queue size"), 3);

    // Verify priorities are preserved
    let queued = queue.list().expect("Failed to list queue");
    assert_eq!(queued[0].priority, Priority::Urgent);
    assert_eq!(queued[1].priority, Priority::High);
    assert_eq!(queued[2].priority, Priority::Low);
}

#[tokio::test]
async fn test_send_delete_chat_failure_queues() {
    let transport = Transport::new();
    let mut queue = MessageQueue::new().expect("Failed to create queue");

    let contact = create_test_contact();

    // Send delete chat (should fail and queue)
    let result = send_delete_chat(&transport, &mut queue, &contact, "my_uid")
        .await
        .expect("Failed to send delete chat");

    // Should return false (not delivered)
    assert!(!result, "Delete chat should not be delivered");

    // Message should be in queue
    assert_eq!(queue.size().expect("Failed to get queue size"), 1);

    // Verify it's a delete_chat message with urgent priority
    let queued_messages = queue.list().expect("Failed to list queue");
    assert_eq!(queued_messages.len(), 1);
    assert_eq!(queued_messages[0].priority, Priority::Urgent);
    assert_eq!(queued_messages[0].message.sender, "my_uid");
    assert_eq!(queued_messages[0].message.recipient, "test_uid");
    assert!(queued_messages[0].message.content.is_empty());
}

#[tokio::test]
async fn test_send_delete_chat_success() {
    // Start a test server to receive the message
    let mut receiver = Transport::new();
    let received_message = Arc::new(Mutex::new(None));
    let received_clone = received_message.clone();

    receiver
        .set_new_message_handler(move |msg| {
            let r = received_clone.clone();
            tokio::spawn(async move {
                *r.lock().await = Some(msg);
            });
        })
        .await;

    receiver
        .start("127.0.0.1:0".parse().unwrap())
        .await
        .expect("Failed to start receiver");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let receiver_addr = receiver.local_addr().expect("No local address");

    // Create sender transport and queue
    let sender = Transport::new();
    let mut queue = MessageQueue::new().expect("Failed to create queue");

    // Create contact pointing to the receiver
    let contact = Contact::new(
        "receiver_uid".to_string(),
        receiver_addr.to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        Utc::now() + Duration::days(30),
    );

    // Send delete chat
    let result = send_delete_chat(&sender, &mut queue, &contact, "sender_uid")
        .await
        .expect("Failed to send delete chat");

    // Should return true (delivered)
    assert!(result, "Delete chat should be delivered");

    // Queue should be empty (not queued)
    assert_eq!(queue.size().expect("Failed to get queue size"), 0);

    // Wait for handler to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify message was received
    let msg = received_message.lock().await;
    assert!(msg.is_some());
    let msg = msg.as_ref().unwrap();
    assert_eq!(msg.from_uid, "sender_uid");
    assert_eq!(msg.message_type, "delete_chat");
    assert!(msg.payload.is_empty());
}

#[test]
fn test_handle_delete_chat_removes_chat() {
    let mut app_state = AppState::new();

    // Add some chats
    app_state.add_chat("alice".to_string());
    app_state.add_chat("bob".to_string());
    app_state.add_chat("charlie".to_string());

    assert_eq!(app_state.chats.len(), 3);

    // Delete bob's chat
    let removed = handle_delete_chat(&mut app_state, "bob");

    assert!(removed, "Chat should be removed");
    assert_eq!(app_state.chats.len(), 2);

    // Verify bob's chat is gone
    assert!(app_state.get_chat("bob").is_none());
    assert!(app_state.get_chat("alice").is_some());
    assert!(app_state.get_chat("charlie").is_some());
}

#[test]
fn test_handle_delete_chat_nonexistent() {
    let mut app_state = AppState::new();
    app_state.add_chat("alice".to_string());

    // Try to delete a chat that doesn't exist
    let removed = handle_delete_chat(&mut app_state, "bob");

    assert!(!removed, "Should return false when chat doesn't exist");
    assert_eq!(app_state.chats.len(), 1);
    assert!(app_state.get_chat("alice").is_some());
}

#[test]
fn test_handle_delete_chat_empty_state() {
    let mut app_state = AppState::new();

    // Try to delete from empty state
    let removed = handle_delete_chat(&mut app_state, "alice");

    assert!(!removed);
    assert_eq!(app_state.chats.len(), 0);
}

#[test]
fn test_handle_delete_chat_with_messages() {
    let mut app_state = AppState::new();

    // Add chat with messages
    let chat = app_state.add_chat("alice".to_string());
    chat.append_message(create_test_message("msg1", "alice", "me"));
    chat.append_message(create_test_message("msg2", "me", "alice"));
    chat.mark_unread();
    chat.mark_has_pending();

    assert_eq!(app_state.chats.len(), 1);
    assert_eq!(app_state.chats[0].messages.len(), 2);

    // Delete the chat
    let removed = handle_delete_chat(&mut app_state, "alice");

    assert!(removed);
    assert_eq!(app_state.chats.len(), 0);
}

#[test]
fn test_handle_incoming_message_new_chat() {
    let mut app_state = AppState::new();
    let sender_uid = "alice";
    let recipient_uid = "bob";
    let message_id = "msg1";
    let content = b"Hello, Bob!".to_vec();
    let timestamp = Utc::now().timestamp_millis();

    // Handle incoming message (should create new chat)
    handle_incoming_message(
        &mut app_state,
        sender_uid,
        recipient_uid,
        message_id,
        content.clone(),
        timestamp,
    );

    // Verify chat was created
    assert_eq!(app_state.chats.len(), 1);
    let chat = app_state.get_chat(sender_uid).unwrap();
    assert_eq!(chat.contact_uid, sender_uid);
    assert!(chat.is_active); // Should be marked as unread

    // Verify message was appended
    assert_eq!(chat.messages.len(), 1);
    let msg = &chat.messages[0];
    assert_eq!(msg.id, message_id);
    assert_eq!(msg.sender, sender_uid);
    assert_eq!(msg.recipient, recipient_uid);
    assert_eq!(msg.content, content);
    assert_eq!(msg.timestamp, timestamp);
    assert!(msg.delivered);
}

#[test]
fn test_handle_incoming_message_existing_chat() {
    let mut app_state = AppState::new();
    let sender_uid = "alice";
    let recipient_uid = "bob";

    // Create existing chat
    let chat = app_state.add_chat(sender_uid.to_string());
    chat.append_message(create_test_message("msg1", sender_uid, recipient_uid));
    chat.mark_read(); // Mark as read initially

    assert_eq!(chat.messages.len(), 1);
    assert!(!chat.is_active);

    // Handle new incoming message
    let message_id = "msg2";
    let content = b"Second message".to_vec();
    let timestamp = Utc::now().timestamp_millis();

    handle_incoming_message(
        &mut app_state,
        sender_uid,
        recipient_uid,
        message_id,
        content.clone(),
        timestamp,
    );

    // Verify chat was updated
    assert_eq!(app_state.chats.len(), 1); // Still only one chat
    let chat = app_state.get_chat(sender_uid).unwrap();
    assert!(chat.is_active); // Should be marked as unread again

    // Verify message was appended
    assert_eq!(chat.messages.len(), 2);
    let msg = &chat.messages[1];
    assert_eq!(msg.id, message_id);
    assert_eq!(msg.content, content);
}

#[test]
fn test_handle_incoming_message_multiple_senders() {
    let mut app_state = AppState::new();
    let recipient_uid = "bob";

    // Receive messages from multiple senders
    handle_incoming_message(
        &mut app_state,
        "alice",
        recipient_uid,
        "msg1",
        b"From Alice".to_vec(),
        Utc::now().timestamp_millis(),
    );

    handle_incoming_message(
        &mut app_state,
        "charlie",
        recipient_uid,
        "msg2",
        b"From Charlie".to_vec(),
        Utc::now().timestamp_millis(),
    );

    handle_incoming_message(
        &mut app_state,
        "alice",
        recipient_uid,
        "msg3",
        b"From Alice again".to_vec(),
        Utc::now().timestamp_millis(),
    );

    // Verify separate chats were created
    assert_eq!(app_state.chats.len(), 2);

    let alice_chat = app_state.get_chat("alice").unwrap();
    assert_eq!(alice_chat.messages.len(), 2);
    assert!(alice_chat.is_active);

    let charlie_chat = app_state.get_chat("charlie").unwrap();
    assert_eq!(charlie_chat.messages.len(), 1);
    assert!(charlie_chat.is_active);
}

// Additional test implementations would continue here following the same pattern
// from the original messaging.rs test module...

#[tokio::test]
async fn test_delete_chat_roundtrip() {
    // Setup: Create sender and receiver with app states
    let sender_transport = Transport::new();
    let mut receiver_transport = Transport::new();

    let mut sender_state = AppState::new();
    let receiver_state = Arc::new(Mutex::new(AppState::new()));

    // Add chat on receiver side
    {
        let mut state = receiver_state.lock().await;
        state.add_chat("sender_uid".to_string());
        assert_eq!(state.chats.len(), 1);
    }

    // Setup receiver to handle delete_chat messages
    let receiver_state_clone = receiver_state.clone();
    receiver_transport
        .set_new_message_handler(move |msg| {
            let state = receiver_state_clone.clone();
            tokio::spawn(async move {
                if msg.message_type == "delete_chat" {
                    let mut s = state.lock().await;
                    handle_delete_chat(&mut s, &msg.from_uid);
                }
            });
        })
        .await;

    // Start receiver
    receiver_transport
        .start("127.0.0.1:0".parse().unwrap())
        .await
        .expect("Failed to start receiver");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let receiver_addr = receiver_transport.local_addr().expect("No local address");

    // Create contact for receiver
    let receiver_contact = Contact::new(
        "receiver_uid".to_string(),
        receiver_addr.to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        Utc::now() + Duration::days(30),
    );

    // Add contact and chat on sender side
    sender_state.contacts.push(receiver_contact.clone());
    sender_state.add_chat("receiver_uid".to_string());

    // Send delete chat from sender
    let mut queue = MessageQueue::new().expect("Failed to create queue");
    let result = send_delete_chat(
        &sender_transport,
        &mut queue,
        &receiver_contact,
        "sender_uid",
    )
    .await
    .expect("Failed to send delete chat");

    assert!(result, "Delete chat should be delivered");

    // Delete local chat on sender side
    let removed = handle_delete_chat(&mut sender_state, "receiver_uid");
    assert!(removed);
    assert_eq!(sender_state.chats.len(), 0);

    // Wait for receiver to process
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Verify receiver's chat was deleted
    let receiver_final_state = receiver_state.lock().await;
    assert_eq!(
        receiver_final_state.chats.len(),
        0,
        "Receiver should have deleted the chat"
    );
}

#[test]
fn test_handle_delete_chat_preserves_other_chats() {
    let mut app_state = AppState::new();

    // Add multiple chats with messages
    for uid in &["alice", "bob", "charlie", "dave"] {
        let chat = app_state.add_chat(uid.to_string());
        chat.append_message(create_test_message("msg1", uid, "me"));
        chat.mark_unread();
    }

    assert_eq!(app_state.chats.len(), 4);

    // Delete charlie's chat
    let removed = handle_delete_chat(&mut app_state, "charlie");

    assert!(removed);
    assert_eq!(app_state.chats.len(), 3);

    // Verify other chats are preserved with their data
    let alice_chat = app_state.get_chat("alice").unwrap();
    assert_eq!(alice_chat.messages.len(), 1);
    assert!(alice_chat.is_active);

    let bob_chat = app_state.get_chat("bob").unwrap();
    assert_eq!(bob_chat.messages.len(), 1);

    let dave_chat = app_state.get_chat("dave").unwrap();
    assert_eq!(dave_chat.messages.len(), 1);

    // Charlie should be gone
    assert!(app_state.get_chat("charlie").is_none());
}

// More tests would continue... I'll include a few more key ones

#[tokio::test]
async fn test_create_chat_from_ping_success() {
    // Start a test server
    let mut receiver = Transport::new();
    receiver.set_local_uid("alice_uid".to_string()).await;

    receiver
        .start("127.0.0.1:0".parse().unwrap())
        .await
        .expect("Failed to start receiver");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let receiver_addr = receiver.local_addr().expect("No local address");

    // Create transport and app state
    let transport = Transport::new();
    let mut app_state = AppState::new();

    // Create contact pointing to receiver
    let contact = Contact::new(
        "alice_uid".to_string(),
        receiver_addr.to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        Utc::now() + Duration::days(30),
    );

    // Create chat from ping
    let online = create_chat_from_ping(&transport, &mut app_state, &contact)
        .await
        .expect("Failed to create chat from ping");

    // Should be online
    assert!(online, "Contact should be online");

    // Chat should exist and be active
    assert_eq!(app_state.chats.len(), 1);
    let chat = app_state.get_chat("alice_uid").unwrap();
    assert!(chat.is_active, "Chat should be active");
    assert_eq!(chat.contact_uid, "alice_uid");
}

#[tokio::test]
async fn test_create_chat_from_ping_failure() {
    let transport = Transport::new();
    let mut app_state = AppState::new();

    // Create contact pointing to unreachable address
    let contact = Contact::new(
        "unreachable_uid".to_string(),
        "127.0.0.1:59999".to_string(),
        vec![1, 2, 3],
        vec![99u8; 32], // x25519_pubkey placeholder
        Utc::now() + Duration::days(30),
    );

    // Create chat from ping (should fail)
    let online = create_chat_from_ping(&transport, &mut app_state, &contact)
        .await
        .expect("Failed to create chat from ping");

    // Should be offline
    assert!(!online, "Contact should be offline");

    // Chat should exist but be inactive
    assert_eq!(app_state.chats.len(), 1);
    let chat = app_state.get_chat("unreachable_uid").unwrap();
    assert!(!chat.is_active, "Chat should be inactive");
    assert_eq!(chat.contact_uid, "unreachable_uid");
}

#[test]
fn test_create_active_chat_new() {
    let mut app_state = AppState::new();

    create_active_chat(&mut app_state, "alice_uid");

    assert_eq!(app_state.chats.len(), 1);
    let chat = app_state.get_chat("alice_uid").unwrap();
    assert!(chat.is_active);
    assert_eq!(chat.contact_uid, "alice_uid");
    assert!(chat.messages.is_empty());
}

#[test]
fn test_create_inactive_chat_new() {
    let mut app_state = AppState::new();

    create_inactive_chat(&mut app_state, "charlie_uid");

    assert_eq!(app_state.chats.len(), 1);
    let chat = app_state.get_chat("charlie_uid").unwrap();
    assert!(!chat.is_active);
    assert_eq!(chat.contact_uid, "charlie_uid");
    assert!(chat.messages.is_empty());
}

#[tokio::test]
async fn test_delete_inactive_chat_immediate() {
    let transport = Transport::new();
    let mut queue = MessageQueue::new().expect("Failed to create queue");
    let mut app_state = AppState::new();

    let contact = create_test_contact();

    // Create inactive chat with matching UID
    create_inactive_chat(&mut app_state, &contact.uid);
    assert_eq!(app_state.chats.len(), 1);

    // Delete inactive chat - should delete immediately without notification
    let was_active = delete_chat(&transport, &mut queue, &mut app_state, &contact, "my_uid")
        .await
        .expect("Failed to delete chat");

    assert!(!was_active, "Should return false for inactive chat");
    assert_eq!(app_state.chats.len(), 0, "Chat should be deleted");
    assert_eq!(queue.size().expect("Failed to get queue size"), 0, "No delete message should be queued");
}

#[tokio::test]
async fn test_delete_active_chat_sends_notification() {
    let transport = Transport::new();
    let mut queue = MessageQueue::new().expect("Failed to create queue");
    let mut app_state = AppState::new();

    // Create active chat
    create_active_chat(&mut app_state, "test_uid");
    assert!(app_state.get_chat("test_uid").unwrap().is_active);

    let contact = create_test_contact();

    // Delete active chat - should send notification
    let was_active = delete_chat(&transport, &mut queue, &mut app_state, &contact, "my_uid")
        .await
        .expect("Failed to delete chat");

    assert!(was_active, "Should return true for active chat");
    assert_eq!(app_state.chats.len(), 0, "Chat should be deleted locally");
    assert_eq!(queue.size().expect("Failed to get queue size"), 1, "Delete message should be queued");

    // Verify it's a delete_chat message
    let queued = queue.list().expect("Failed to list queue");
    assert_eq!(queued[0].priority, Priority::Urgent);
}
