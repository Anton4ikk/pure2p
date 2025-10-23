//! High-level messaging module
//!
//! This module provides user-facing messaging functions that combine
//! transport and queue operations for reliable message delivery.

use crate::{
    queue::{MessageQueue, Priority},
    storage::{AppState, Contact, Message},
    transport::Transport,
    Result,
};
use chrono::Utc;

/// Send a message to a contact with automatic queueing on failure
///
/// This function attempts to deliver a message immediately. If delivery fails,
/// the message is automatically queued for retry with the specified priority.
///
/// # Arguments
/// * `transport` - The transport layer for sending messages
/// * `queue` - The message queue for retry logic
/// * `contact` - The contact to send the message to
/// * `message` - The message to send
/// * `priority` - Priority for queueing if delivery fails (default: Normal)
///
/// # Returns
/// * `Ok(true)` - Message delivered successfully
/// * `Ok(false)` - Message queued for retry (delivery failed)
/// * `Err(Error)` - Failed to queue message
///
/// # Example
/// ```rust,no_run
/// use pure2p::messaging::send_message;
/// use pure2p::transport::Transport;
/// use pure2p::queue::{MessageQueue, Priority};
/// use pure2p::storage::{Contact, Message};
/// use chrono::{Utc, Duration};
///
/// # async fn example() -> pure2p::Result<()> {
/// let transport = Transport::new();
/// let mut queue = MessageQueue::new()?;
///
/// let contact = Contact::new(
///     "alice_uid".to_string(),
///     "192.168.1.100:8080".to_string(),
///     vec![1, 2, 3],
///     Utc::now() + Duration::days(30),
/// );
///
/// let message = Message {
///     id: "msg_123".to_string(),
///     sender: "my_uid".to_string(),
///     recipient: "alice_uid".to_string(),
///     content: b"Hello, Alice!".to_vec(),
///     timestamp: Utc::now().timestamp_millis(),
///     delivered: false,
/// };
///
/// let delivered = send_message(&transport, &mut queue, &contact, &message, Priority::Normal).await?;
///
/// if delivered {
///     println!("Message delivered immediately");
/// } else {
///     println!("Message queued for retry");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn send_message(
    transport: &Transport,
    queue: &mut MessageQueue,
    contact: &Contact,
    message: &Message,
    priority: Priority,
) -> Result<bool> {
    // Try to send via transport using /message endpoint
    let result = transport
        .send_message(
            contact,
            &message.sender,
            "text", // Default message type
            message.content.clone(),
        )
        .await;

    match result {
        Ok(()) => {
            // Message delivered successfully
            tracing::info!("Message {} delivered to {}", message.id, contact.uid);
            Ok(true)
        }
        Err(e) => {
            // Delivery failed, enqueue for retry
            tracing::warn!(
                "Failed to deliver message {} to {}: {}. Queueing for retry.",
                message.id,
                contact.uid,
                e
            );
            queue.enqueue(message.clone(), priority)?;
            Ok(false)
        }
    }
}

/// Send a message with a custom message type and automatic queueing on failure
///
/// Similar to `send_message` but allows specifying a custom message type
/// (e.g., "text", "delete", "typing", "file", etc.).
///
/// # Arguments
/// * `transport` - The transport layer for sending messages
/// * `queue` - The message queue for retry logic
/// * `contact` - The contact to send the message to
/// * `message` - The message to send
/// * `message_type` - Type of message (e.g., "text", "delete", "typing")
/// * `priority` - Priority for queueing if delivery fails
///
/// # Returns
/// * `Ok(true)` - Message delivered successfully
/// * `Ok(false)` - Message queued for retry (delivery failed)
/// * `Err(Error)` - Failed to queue message
pub async fn send_message_with_type(
    transport: &Transport,
    queue: &mut MessageQueue,
    contact: &Contact,
    message: &Message,
    message_type: &str,
    priority: Priority,
) -> Result<bool> {
    // Try to send via transport using /message endpoint
    let result = transport
        .send_message(contact, &message.sender, message_type, message.content.clone())
        .await;

    match result {
        Ok(()) => {
            // Message delivered successfully
            tracing::info!(
                "Message {} (type: {}) delivered to {}",
                message.id,
                message_type,
                contact.uid
            );
            Ok(true)
        }
        Err(e) => {
            // Delivery failed, enqueue for retry
            tracing::warn!(
                "Failed to deliver message {} (type: {}) to {}: {}. Queueing for retry.",
                message.id,
                message_type,
                contact.uid,
                e
            );
            queue.enqueue_with_type(message.clone(), priority, message_type)?;
            Ok(false)
        }
    }
}

/// Send a delete chat request to a contact
///
/// This function sends a special DELETE-type message to a contact to notify them
/// that the local user has deleted the chat. The contact should remove the chat
/// from their local AppState upon receiving this message.
///
/// # Arguments
/// * `transport` - The transport layer for sending messages
/// * `queue` - The message queue for retry logic
/// * `contact` - The contact to send the delete request to
/// * `local_uid` - The UID of the local user (sender)
///
/// # Returns
/// * `Ok(true)` - Delete request delivered successfully
/// * `Ok(false)` - Delete request queued for retry (delivery failed)
/// * `Err(Error)` - Failed to queue message
///
/// # Example
/// ```rust,no_run
/// use pure2p::messaging::send_delete_chat;
/// use pure2p::transport::Transport;
/// use pure2p::queue::MessageQueue;
/// use pure2p::storage::Contact;
/// use chrono::{Utc, Duration};
///
/// # async fn example() -> pure2p::Result<()> {
/// let transport = Transport::new();
/// let mut queue = MessageQueue::new()?;
///
/// let contact = Contact::new(
///     "alice_uid".to_string(),
///     "192.168.1.100:8080".to_string(),
///     vec![1, 2, 3],
///     Utc::now() + Duration::days(30),
/// );
///
/// let delivered = send_delete_chat(&transport, &mut queue, &contact, "my_uid").await?;
///
/// if delivered {
///     println!("Delete request sent immediately");
/// } else {
///     println!("Delete request queued for retry");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn send_delete_chat(
    transport: &Transport,
    queue: &mut MessageQueue,
    contact: &Contact,
    local_uid: &str,
) -> Result<bool> {
    // Create a delete message with empty payload
    // Use timestamp + recipient UID to ensure uniqueness
    let timestamp = Utc::now().timestamp_millis();
    let message = Message {
        id: format!("delete_chat_{}_{}", contact.uid, timestamp),
        sender: local_uid.to_string(),
        recipient: contact.uid.clone(),
        content: vec![],
        timestamp,
        delivered: false,
    };

    // Send with "delete_chat" message type and urgent priority
    send_message_with_type(
        transport,
        queue,
        contact,
        &message,
        "delete_chat",
        Priority::Urgent,
    )
    .await
}

/// Handle incoming delete chat request
///
/// This function should be called when a DELETE-type message is received.
/// It removes the chat associated with the sender from the local AppState.
///
/// # Arguments
/// * `app_state` - The application state containing chats
/// * `sender_uid` - The UID of the sender who initiated the delete request
///
/// # Returns
/// * `Ok(true)` - Chat was found and removed
/// * `Ok(false)` - Chat was not found (already deleted or never existed)
///
/// # Example
/// ```rust,no_run
/// use pure2p::messaging::handle_delete_chat;
/// use pure2p::storage::AppState;
///
/// let mut app_state = AppState::new();
/// app_state.add_chat("alice_uid".to_string());
///
/// let removed = handle_delete_chat(&mut app_state, "alice_uid");
/// assert!(removed); // Chat was removed
/// ```
pub fn handle_delete_chat(app_state: &mut AppState, sender_uid: &str) -> bool {
    let initial_len = app_state.chats.len();
    app_state.chats.retain(|chat| chat.contact_uid != sender_uid);
    let new_len = app_state.chats.len();

    // Return true if a chat was removed
    initial_len > new_len
}

/// Handle an incoming message
///
/// This function processes incoming messages by:
/// - Getting or creating a chat for the sender
/// - Appending the message to the chat history
/// - Marking the chat as unread for TUI display
///
/// # Arguments
/// * `app_state` - The application state to update
/// * `sender_uid` - The UID of the message sender
/// * `recipient_uid` - The UID of the message recipient (local user)
/// * `message_id` - The unique message ID
/// * `content` - The message content (encrypted or plaintext)
/// * `timestamp` - The message timestamp
///
/// # Example
/// ```rust,no_run
/// use pure2p::messaging::handle_incoming_message;
/// use pure2p::storage::AppState;
/// use chrono::Utc;
/// use uuid::Uuid;
///
/// let mut app_state = AppState::new();
/// let sender_uid = "alice_uid";
/// let recipient_uid = "bob_uid";
/// let message_id = Uuid::new_v4().to_string();
/// let content = b"Hello!".to_vec();
/// let timestamp = Utc::now().timestamp_millis();
///
/// handle_incoming_message(
///     &mut app_state,
///     sender_uid,
///     recipient_uid,
///     &message_id,
///     content,
///     timestamp,
/// );
/// ```
pub fn handle_incoming_message(
    app_state: &mut AppState,
    sender_uid: &str,
    recipient_uid: &str,
    message_id: &str,
    content: Vec<u8>,
    timestamp: i64,
) {
    // Get or create chat for this sender
    let chat = app_state.get_or_create_chat(sender_uid);

    // Create message object
    let message = Message {
        id: message_id.to_string(),
        sender: sender_uid.to_string(),
        recipient: recipient_uid.to_string(),
        content,
        timestamp,
        delivered: true, // Incoming messages are already delivered to us
    };

    // Append message to chat history
    chat.append_message(message);

    // Mark chat as unread for TUI display
    chat.mark_unread();

    tracing::info!(
        "Received message {} from {} in chat",
        message_id,
        sender_uid
    );
}

/// Create or update chat based on ping response
///
/// This function manages chat lifecycle based on ping success/failure:
/// - On successful ping: Creates active chat or activates existing chat
/// - On ping failure: Creates inactive chat (contact offline/unreachable)
///
/// # Arguments
/// * `transport` - The transport layer for sending pings
/// * `app_state` - The application state to store chats
/// * `contact` - The contact to ping
///
/// # Returns
/// * `Ok(true)` - Ping successful, active chat created/updated
/// * `Ok(false)` - Ping failed, inactive chat created
///
/// # Example
/// ```rust,no_run
/// use pure2p::messaging::create_chat_from_ping;
/// use pure2p::transport::Transport;
/// use pure2p::storage::{AppState, Contact};
/// use chrono::{Utc, Duration};
///
/// # async fn example() -> pure2p::Result<()> {
/// let transport = Transport::new();
/// let mut app_state = AppState::new();
///
/// let contact = Contact::new(
///     "alice_uid".to_string(),
///     "192.168.1.100:8080".to_string(),
///     vec![1, 2, 3],
///     Utc::now() + Duration::days(30),
/// );
///
/// let online = create_chat_from_ping(&transport, &mut app_state, &contact).await?;
///
/// if online {
///     println!("Contact is online, active chat created");
/// } else {
///     println!("Contact is offline, inactive chat created");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn create_chat_from_ping(
    transport: &Transport,
    app_state: &mut AppState,
    contact: &Contact,
) -> Result<bool> {
    // Try to ping the contact
    let ping_result = transport.send_ping(contact).await;

    match ping_result {
        Ok(response) => {
            // Ping successful - contact is online
            tracing::info!(
                "Ping successful for {}, creating/activating chat",
                response.uid
            );

            // Get or create chat
            let chat = app_state.get_or_create_chat(&contact.uid);

            // Mark as active (online and reachable)
            chat.mark_unread();

            Ok(true)
        }
        Err(e) => {
            // Ping failed - contact is offline or unreachable
            tracing::warn!("Ping failed for {}: {}, creating inactive chat", contact.uid, e);

            // Get or create chat
            let chat = app_state.get_or_create_chat(&contact.uid);

            // Mark as inactive (offline/unreachable)
            chat.mark_read();

            Ok(false)
        }
    }
}

/// Create active chat for a contact
///
/// Creates a new chat or activates an existing chat without pinging.
/// Use this when you already know the contact is online (e.g., received a message from them).
///
/// # Arguments
/// * `app_state` - The application state to store chats
/// * `contact_uid` - The UID of the contact
///
/// # Example
/// ```rust,no_run
/// use pure2p::messaging::create_active_chat;
/// use pure2p::storage::AppState;
///
/// let mut app_state = AppState::new();
/// create_active_chat(&mut app_state, "alice_uid");
///
/// let chat = app_state.get_chat("alice_uid").unwrap();
/// assert!(chat.is_active);
/// ```
pub fn create_active_chat(app_state: &mut AppState, contact_uid: &str) {
    let chat = app_state.get_or_create_chat(contact_uid);
    chat.mark_unread();
}

/// Create inactive chat for a contact
///
/// Creates a new chat or marks an existing chat as inactive without pinging.
/// Use this when you know the contact is offline or want to prepare a chat
/// before the contact becomes available.
///
/// # Arguments
/// * `app_state` - The application state to store chats
/// * `contact_uid` - The UID of the contact
///
/// # Example
/// ```rust,no_run
/// use pure2p::messaging::create_inactive_chat;
/// use pure2p::storage::AppState;
///
/// let mut app_state = AppState::new();
/// create_inactive_chat(&mut app_state, "bob_uid");
///
/// let chat = app_state.get_chat("bob_uid").unwrap();
/// assert!(!chat.is_active);
/// ```
pub fn create_inactive_chat(app_state: &mut AppState, contact_uid: &str) {
    let chat = app_state.get_or_create_chat(contact_uid);
    chat.mark_read();
}

/// Delete a chat with appropriate logic based on active status
///
/// This function implements different deletion strategies:
/// - **Inactive chat**: Deletes immediately (no network call needed)
/// - **Active chat**: Sends delete request to contact, then deletes locally
///
/// # Arguments
/// * `transport` - The transport layer for sending delete requests
/// * `queue` - The message queue for retry logic
/// * `app_state` - The application state containing chats
/// * `contact` - The contact whose chat should be deleted
/// * `local_uid` - The UID of the local user
///
/// # Returns
/// * `Ok(true)` - Chat was active, delete request sent (or queued), chat deleted locally
/// * `Ok(false)` - Chat was inactive, deleted immediately without notification
/// * `Err(Error)` - Failed to queue delete request
///
/// # Example
/// ```rust,no_run
/// use pure2p::messaging::delete_chat;
/// use pure2p::transport::Transport;
/// use pure2p::queue::MessageQueue;
/// use pure2p::storage::{AppState, Contact};
/// use chrono::{Utc, Duration};
///
/// # async fn example() -> pure2p::Result<()> {
/// let transport = Transport::new();
/// let mut queue = MessageQueue::new()?;
/// let mut app_state = AppState::new();
///
/// let contact = Contact::new(
///     "alice_uid".to_string(),
///     "192.168.1.100:8080".to_string(),
///     vec![1, 2, 3],
///     Utc::now() + Duration::days(30),
/// );
///
/// let was_active = delete_chat(&transport, &mut queue, &mut app_state, &contact, "my_uid").await?;
///
/// if was_active {
///     println!("Active chat deleted, notification sent to contact");
/// } else {
///     println!("Inactive chat deleted immediately");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn delete_chat(
    transport: &Transport,
    queue: &mut MessageQueue,
    app_state: &mut AppState,
    contact: &Contact,
    local_uid: &str,
) -> Result<bool> {
    // Check if chat exists and is active
    let is_active = if let Some(chat) = app_state.get_chat(&contact.uid) {
        chat.is_active
    } else {
        // Chat doesn't exist, nothing to delete
        return Ok(false);
    };

    if is_active {
        // Active chat - send delete request to contact
        tracing::info!(
            "Deleting active chat with {}, sending delete request",
            contact.uid
        );

        // Send delete request (will queue if delivery fails)
        let _delivered = send_delete_chat(transport, queue, contact, local_uid).await?;

        // Delete local chat regardless of delivery status
        // (delete request is queued if delivery failed)
        handle_delete_chat(app_state, &contact.uid);

        Ok(true)
    } else {
        // Inactive chat - delete immediately without notification
        tracing::info!(
            "Deleting inactive chat with {} immediately",
            contact.uid
        );

        handle_delete_chat(app_state, &contact.uid);

        Ok(false)
    }
}

/// Delete inactive chat immediately
///
/// Deletes an inactive chat without sending any notification.
/// Returns an error if the chat is active (use `delete_chat()` instead).
///
/// # Arguments
/// * `app_state` - The application state containing chats
/// * `contact_uid` - The UID of the contact whose chat should be deleted
///
/// # Returns
/// * `Ok(true)` - Inactive chat was found and deleted
/// * `Ok(false)` - Chat was not found
/// * `Err(Error)` - Chat exists but is active (cannot delete immediately)
///
/// # Example
/// ```rust,no_run
/// use pure2p::messaging::delete_inactive_chat_immediate;
/// use pure2p::storage::AppState;
///
/// # fn example() -> pure2p::Result<()> {
/// let mut app_state = AppState::new();
/// app_state.add_chat("bob_uid".to_string());
/// app_state.get_chat_mut("bob_uid").unwrap().mark_read(); // Make inactive
///
/// let deleted = delete_inactive_chat_immediate(&mut app_state, "bob_uid")?;
/// assert!(deleted);
/// # Ok(())
/// # }
/// ```
pub fn delete_inactive_chat_immediate(
    app_state: &mut AppState,
    contact_uid: &str,
) -> Result<bool> {
    // Check if chat exists and its state
    if let Some(chat) = app_state.get_chat(contact_uid) {
        if chat.is_active {
            // Chat is active, cannot delete immediately
            return Err(crate::Error::Storage(format!(
                "Cannot delete active chat immediately: {}. Use delete_chat() instead.",
                contact_uid
            )));
        }
    } else {
        // Chat doesn't exist
        return Ok(false);
    }

    // Delete the inactive chat
    let deleted = handle_delete_chat(app_state, contact_uid);
    Ok(deleted)
}

/// Delete active chat with notification
///
/// Sends delete request to contact and deletes chat locally.
/// Use this when you want to notify the contact that you're deleting the chat.
///
/// # Arguments
/// * `transport` - The transport layer for sending delete requests
/// * `queue` - The message queue for retry logic
/// * `app_state` - The application state containing chats
/// * `contact` - The contact whose chat should be deleted
/// * `local_uid` - The UID of the local user
///
/// # Returns
/// * `Ok(true)` - Delete request sent (or queued), chat deleted locally
/// * `Ok(false)` - Chat was not found
/// * `Err(Error)` - Failed to queue delete request
///
/// # Example
/// ```rust,no_run
/// use pure2p::messaging::delete_active_chat_with_notification;
/// use pure2p::transport::Transport;
/// use pure2p::queue::MessageQueue;
/// use pure2p::storage::{AppState, Contact};
/// use chrono::{Utc, Duration};
///
/// # async fn example() -> pure2p::Result<()> {
/// let transport = Transport::new();
/// let mut queue = MessageQueue::new()?;
/// let mut app_state = AppState::new();
///
/// let contact = Contact::new(
///     "alice_uid".to_string(),
///     "192.168.1.100:8080".to_string(),
///     vec![1, 2, 3],
///     Utc::now() + Duration::days(30),
/// );
///
/// let deleted = delete_active_chat_with_notification(
///     &transport,
///     &mut queue,
///     &mut app_state,
///     &contact,
///     "my_uid"
/// ).await?;
///
/// if deleted {
///     println!("Delete request sent, chat removed");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn delete_active_chat_with_notification(
    transport: &Transport,
    queue: &mut MessageQueue,
    app_state: &mut AppState,
    contact: &Contact,
    local_uid: &str,
) -> Result<bool> {
    // Check if chat exists
    if app_state.get_chat(&contact.uid).is_none() {
        return Ok(false);
    }

    // Send delete request
    tracing::info!(
        "Deleting chat with {}, sending delete notification",
        contact.uid
    );

    let _delivered = send_delete_chat(transport, queue, contact, local_uid).await?;

    // Delete local chat
    let deleted = handle_delete_chat(app_state, &contact.uid);

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_contact() -> Contact {
        Contact::new(
            "test_uid".to_string(),
            "127.0.0.1:9999".to_string(), // Use unlikely port to simulate failure
            vec![1, 2, 3],
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
        use std::sync::Arc;
        use tokio::sync::Mutex;

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

    #[tokio::test]
    async fn test_delete_chat_roundtrip() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

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

    #[tokio::test]
    async fn test_create_chat_from_ping_updates_existing() {
        // Start a test server
        let mut receiver = Transport::new();
        receiver.set_local_uid("bob_uid".to_string()).await;

        receiver
            .start("127.0.0.1:0".parse().unwrap())
            .await
            .expect("Failed to start receiver");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let receiver_addr = receiver.local_addr().expect("No local address");

        let transport = Transport::new();
        let mut app_state = AppState::new();

        // Create contact
        let contact = Contact::new(
            "bob_uid".to_string(),
            receiver_addr.to_string(),
            vec![1, 2, 3],
            Utc::now() + Duration::days(30),
        );

        // Create inactive chat manually first
        create_inactive_chat(&mut app_state, "bob_uid");
        assert!(!app_state.get_chat("bob_uid").unwrap().is_active);

        // Ping should reactivate the chat
        let online = create_chat_from_ping(&transport, &mut app_state, &contact)
            .await
            .expect("Failed to create chat from ping");

        assert!(online);

        // Should still have only 1 chat, but now active
        assert_eq!(app_state.chats.len(), 1);
        let chat = app_state.get_chat("bob_uid").unwrap();
        assert!(chat.is_active, "Chat should now be active");
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
    fn test_create_active_chat_existing_inactive() {
        let mut app_state = AppState::new();

        // Create inactive chat first
        create_inactive_chat(&mut app_state, "bob_uid");
        assert!(!app_state.get_chat("bob_uid").unwrap().is_active);

        // Activate it
        create_active_chat(&mut app_state, "bob_uid");

        // Should still be 1 chat, now active
        assert_eq!(app_state.chats.len(), 1);
        let chat = app_state.get_chat("bob_uid").unwrap();
        assert!(chat.is_active);
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

    #[test]
    fn test_create_inactive_chat_existing_active() {
        let mut app_state = AppState::new();

        // Create active chat first
        create_active_chat(&mut app_state, "dave_uid");
        assert!(app_state.get_chat("dave_uid").unwrap().is_active);

        // Deactivate it
        create_inactive_chat(&mut app_state, "dave_uid");

        // Should still be 1 chat, now inactive
        assert_eq!(app_state.chats.len(), 1);
        let chat = app_state.get_chat("dave_uid").unwrap();
        assert!(!chat.is_active);
    }

    #[test]
    fn test_create_chat_preserves_messages() {
        let mut app_state = AppState::new();

        // Create chat with messages
        let chat = app_state.add_chat("alice_uid".to_string());
        chat.append_message(create_test_message("msg1", "alice_uid", "me"));
        chat.append_message(create_test_message("msg2", "me", "alice_uid"));
        chat.mark_read();

        assert_eq!(chat.messages.len(), 2);
        assert!(!chat.is_active);

        // Activate the chat
        create_active_chat(&mut app_state, "alice_uid");

        // Messages should be preserved, chat should be active
        let chat = app_state.get_chat("alice_uid").unwrap();
        assert_eq!(chat.messages.len(), 2);
        assert!(chat.is_active);
    }

    #[test]
    fn test_multiple_chat_lifecycle_operations() {
        let mut app_state = AppState::new();

        // Create multiple chats with different states
        create_active_chat(&mut app_state, "alice");
        create_inactive_chat(&mut app_state, "bob");
        create_active_chat(&mut app_state, "charlie");

        assert_eq!(app_state.chats.len(), 3);
        assert!(app_state.get_chat("alice").unwrap().is_active);
        assert!(!app_state.get_chat("bob").unwrap().is_active);
        assert!(app_state.get_chat("charlie").unwrap().is_active);

        // Toggle states
        create_inactive_chat(&mut app_state, "alice");
        create_active_chat(&mut app_state, "bob");

        // Verify changes
        assert!(!app_state.get_chat("alice").unwrap().is_active);
        assert!(app_state.get_chat("bob").unwrap().is_active);
        assert!(app_state.get_chat("charlie").unwrap().is_active);

        // Should still be 3 chats
        assert_eq!(app_state.chats.len(), 3);
    }

    #[tokio::test]
    async fn test_chat_lifecycle_with_messages() {
        // Start a test server
        let mut receiver = Transport::new();
        receiver.set_local_uid("alice_uid".to_string()).await;

        receiver
            .start("127.0.0.1:0".parse().unwrap())
            .await
            .expect("Failed to start receiver");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let receiver_addr = receiver.local_addr().expect("No local address");

        let transport = Transport::new();
        let mut app_state = AppState::new();

        // Create contact
        let contact = Contact::new(
            "alice_uid".to_string(),
            receiver_addr.to_string(),
            vec![1, 2, 3],
            Utc::now() + Duration::days(30),
        );

        // Add some messages first
        let chat = app_state.add_chat("alice_uid".to_string());
        chat.append_message(create_test_message("msg1", "alice_uid", "me"));
        chat.mark_read();

        assert_eq!(chat.messages.len(), 1);
        assert!(!chat.is_active);

        // Ping to activate
        let online = create_chat_from_ping(&transport, &mut app_state, &contact)
            .await
            .expect("Failed to create chat from ping");

        assert!(online);

        // Chat should be active, messages preserved
        let chat = app_state.get_chat("alice_uid").unwrap();
        assert!(chat.is_active);
        assert_eq!(chat.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_ping_multiple_contacts() {
        // Start two test servers
        let mut receiver1 = Transport::new();
        let mut receiver2 = Transport::new();

        receiver1.set_local_uid("alice_uid".to_string()).await;
        receiver2.set_local_uid("bob_uid".to_string()).await;

        receiver1
            .start("127.0.0.1:0".parse().unwrap())
            .await
            .expect("Failed to start receiver1");

        receiver2
            .start("127.0.0.1:0".parse().unwrap())
            .await
            .expect("Failed to start receiver2");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let addr1 = receiver1.local_addr().expect("No local address");
        let addr2 = receiver2.local_addr().expect("No local address");

        let transport = Transport::new();
        let mut app_state = AppState::new();

        // Create contacts
        let contact1 = Contact::new(
            "alice_uid".to_string(),
            addr1.to_string(),
            vec![1],
            Utc::now() + Duration::days(30),
        );

        let contact2 = Contact::new(
            "bob_uid".to_string(),
            addr2.to_string(),
            vec![2],
            Utc::now() + Duration::days(30),
        );

        // Create offline contact
        let contact3 = Contact::new(
            "charlie_uid".to_string(),
            "127.0.0.1:59998".to_string(),
            vec![3],
            Utc::now() + Duration::days(30),
        );

        // Ping all three
        let online1 = create_chat_from_ping(&transport, &mut app_state, &contact1)
            .await
            .expect("Failed");
        let online2 = create_chat_from_ping(&transport, &mut app_state, &contact2)
            .await
            .expect("Failed");
        let online3 = create_chat_from_ping(&transport, &mut app_state, &contact3)
            .await
            .expect("Failed");

        // First two should be online, third offline
        assert!(online1);
        assert!(online2);
        assert!(!online3);

        // Should have 3 chats
        assert_eq!(app_state.chats.len(), 3);
        assert!(app_state.get_chat("alice_uid").unwrap().is_active);
        assert!(app_state.get_chat("bob_uid").unwrap().is_active);
        assert!(!app_state.get_chat("charlie_uid").unwrap().is_active);
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

    #[tokio::test]
    async fn test_delete_nonexistent_chat() {
        let transport = Transport::new();
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let mut app_state = AppState::new();

        let contact = create_test_contact();

        // Try to delete nonexistent chat
        let result = delete_chat(&transport, &mut queue, &mut app_state, &contact, "my_uid")
            .await
            .expect("Failed to delete chat");

        assert!(!result, "Should return false for nonexistent chat");
        assert_eq!(app_state.chats.len(), 0);
        assert_eq!(queue.size().expect("Failed to get queue size"), 0);
    }

    #[test]
    fn test_delete_inactive_chat_immediate_function() {
        let mut app_state = AppState::new();

        // Create inactive chat
        create_inactive_chat(&mut app_state, "alice_uid");
        assert_eq!(app_state.chats.len(), 1);

        // Delete it immediately
        let deleted = delete_inactive_chat_immediate(&mut app_state, "alice_uid")
            .expect("Failed to delete inactive chat");

        assert!(deleted);
        assert_eq!(app_state.chats.len(), 0);
    }

    #[test]
    fn test_delete_inactive_chat_immediate_rejects_active() {
        let mut app_state = AppState::new();

        // Create active chat
        create_active_chat(&mut app_state, "bob_uid");

        // Try to delete immediately - should fail
        let result = delete_inactive_chat_immediate(&mut app_state, "bob_uid");

        assert!(result.is_err());
        match result {
            Err(crate::Error::Storage(msg)) => {
                assert!(msg.contains("Cannot delete active chat immediately"));
            }
            _ => panic!("Expected Storage error"),
        }

        // Chat should still exist
        assert_eq!(app_state.chats.len(), 1);
    }

    #[test]
    fn test_delete_inactive_chat_immediate_nonexistent() {
        let mut app_state = AppState::new();

        // Try to delete nonexistent chat
        let deleted = delete_inactive_chat_immediate(&mut app_state, "nonexistent")
            .expect("Failed to delete");

        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_delete_active_chat_with_notification_function() {
        let transport = Transport::new();
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let mut app_state = AppState::new();

        // Create active chat
        create_active_chat(&mut app_state, "test_uid");

        let contact = create_test_contact();

        // Delete with notification
        let deleted = delete_active_chat_with_notification(
            &transport,
            &mut queue,
            &mut app_state,
            &contact,
            "my_uid",
        )
        .await
        .expect("Failed to delete");

        assert!(deleted);
        assert_eq!(app_state.chats.len(), 0);
        assert_eq!(queue.size().expect("Failed to get queue size"), 1);
    }

    #[tokio::test]
    async fn test_delete_chat_preserves_messages_in_other_chats() {
        let transport = Transport::new();
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let mut app_state = AppState::new();

        // Create multiple chats with messages
        for uid in &["alice", "bob", "charlie"] {
            let chat = app_state.add_chat(uid.to_string());
            chat.append_message(create_test_message("msg1", uid, "me"));
            chat.mark_unread();
        }

        assert_eq!(app_state.chats.len(), 3);

        // Create contact for bob
        let contact = Contact::new(
            "bob".to_string(),
            "127.0.0.1:9999".to_string(),
            vec![2],
            Utc::now() + Duration::days(30),
        );

        // Delete bob's chat
        delete_chat(&transport, &mut queue, &mut app_state, &contact, "my_uid")
            .await
            .expect("Failed to delete");

        // Should have 2 chats left with their messages
        assert_eq!(app_state.chats.len(), 2);
        assert!(app_state.get_chat("alice").is_some());
        assert!(app_state.get_chat("bob").is_none());
        assert!(app_state.get_chat("charlie").is_some());

        assert_eq!(app_state.get_chat("alice").unwrap().messages.len(), 1);
        assert_eq!(app_state.get_chat("charlie").unwrap().messages.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_chat_active_then_inactive() {
        let transport = Transport::new();
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let mut app_state = AppState::new();

        let contact = create_test_contact();

        // Create and delete active chat
        create_active_chat(&mut app_state, "test_uid");
        let result1 = delete_chat(&transport, &mut queue, &mut app_state, &contact, "my_uid")
            .await
            .expect("Failed");

        assert!(result1); // Was active
        assert_eq!(queue.size().expect("Failed"), 1); // Queued delete message

        // Create and delete inactive chat
        create_inactive_chat(&mut app_state, "test_uid");
        let result2 = delete_chat(&transport, &mut queue, &mut app_state, &contact, "my_uid")
            .await
            .expect("Failed");

        assert!(!result2); // Was inactive
        assert_eq!(queue.size().expect("Failed"), 1); // Still only 1 message (no new message)
    }

    #[tokio::test]
    async fn test_delete_chat_with_pending_messages() {
        let transport = Transport::new();
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let mut app_state = AppState::new();

        // Create active chat with messages and pending flag
        let chat = app_state.add_chat("alice_uid".to_string());
        chat.append_message(create_test_message("msg1", "alice_uid", "me"));
        chat.append_message(create_test_message("msg2", "me", "alice_uid"));
        chat.mark_unread();
        chat.mark_has_pending();

        assert_eq!(chat.messages.len(), 2);
        assert!(chat.has_pending_messages);

        let contact = Contact::new(
            "alice_uid".to_string(),
            "127.0.0.1:9999".to_string(),
            vec![1],
            Utc::now() + Duration::days(30),
        );

        // Delete chat
        delete_chat(&transport, &mut queue, &mut app_state, &contact, "my_uid")
            .await
            .expect("Failed to delete");

        // Chat should be completely gone
        assert!(app_state.get_chat("alice_uid").is_none());
    }

    #[tokio::test]
    async fn test_delete_multiple_chats_mixed_states() {
        let transport = Transport::new();
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let mut app_state = AppState::new();

        // Create mixed active/inactive chats
        create_active_chat(&mut app_state, "alice");
        create_inactive_chat(&mut app_state, "bob");
        create_active_chat(&mut app_state, "charlie");
        create_inactive_chat(&mut app_state, "dave");

        assert_eq!(app_state.chats.len(), 4);

        // Delete active chats
        let contact1 = Contact::new(
            "alice".to_string(),
            "127.0.0.1:9999".to_string(),
            vec![1],
            Utc::now() + Duration::days(30),
        );

        let contact3 = Contact::new(
            "charlie".to_string(),
            "127.0.0.1:9998".to_string(),
            vec![3],
            Utc::now() + Duration::days(30),
        );

        delete_chat(&transport, &mut queue, &mut app_state, &contact1, "me")
            .await
            .expect("Failed");

        // Add a small delay to ensure unique message IDs (timestamp-based)
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        delete_chat(&transport, &mut queue, &mut app_state, &contact3, "me")
            .await
            .expect("Failed");

        // Should have 2 inactive chats left, 2 delete messages queued
        assert_eq!(app_state.chats.len(), 2);
        assert_eq!(queue.size().expect("Failed"), 2);

        // Delete inactive chats
        delete_inactive_chat_immediate(&mut app_state, "bob").expect("Failed");
        delete_inactive_chat_immediate(&mut app_state, "dave").expect("Failed");

        // Should have no chats, still 2 delete messages
        assert_eq!(app_state.chats.len(), 0);
        assert_eq!(queue.size().expect("Failed"), 2);
    }

    #[tokio::test]
    async fn test_delete_chat_integration_with_message_handler() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        // Setup sender and receiver
        let sender_transport = Transport::new();
        let mut receiver_transport = Transport::new();

        let sender_state = Arc::new(Mutex::new(AppState::new()));
        let receiver_state = Arc::new(Mutex::new(AppState::new()));

        // Add chats on both sides
        {
            let mut s_state = sender_state.lock().await;
            create_active_chat(&mut s_state, "receiver_uid");
        }
        {
            let mut r_state = receiver_state.lock().await;
            create_active_chat(&mut r_state, "sender_uid");
        }

        // Setup receiver to handle delete messages
        let r_state_clone = receiver_state.clone();
        receiver_transport
            .set_new_message_handler(move |msg| {
                let state = r_state_clone.clone();
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

        // Create contact
        let contact = Contact::new(
            "receiver_uid".to_string(),
            receiver_addr.to_string(),
            vec![1],
            Utc::now() + Duration::days(30),
        );

        // Delete chat from sender
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        {
            let mut s_state = sender_state.lock().await;
            delete_chat(&sender_transport, &mut queue, &mut s_state, &contact, "sender_uid")
                .await
                .expect("Failed to delete");

            // Sender's chat should be deleted
            assert_eq!(s_state.chats.len(), 0);
        }

        // Wait for receiver to process
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Receiver's chat should also be deleted
        {
            let r_state = receiver_state.lock().await;
            assert_eq!(r_state.chats.len(), 0);
        }
    }
}
