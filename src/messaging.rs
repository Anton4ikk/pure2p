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

