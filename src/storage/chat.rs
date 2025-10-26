//! Chat conversation management

use crate::storage::message::Message;
use serde::{Deserialize, Serialize};

/// Represents a chat conversation with a contact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    /// Contact UID this chat is with
    pub contact_uid: String,
    /// Messages in this conversation
    pub messages: Vec<Message>,
    /// Whether this chat is active (unread messages present)
    pub is_active: bool,
    /// Whether there are pending (queued) messages for this contact
    pub has_pending_messages: bool,
}

impl Chat {
    /// Create a new chat with a contact
    pub fn new(contact_uid: String) -> Self {
        Self {
            contact_uid,
            messages: Vec::new(),
            is_active: false,
            has_pending_messages: false,
        }
    }

    /// Append a message to this chat
    pub fn append_message(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    /// Mark chat as having unread messages (active)
    pub fn mark_unread(&mut self) {
        self.is_active = true;
    }

    /// Mark chat as read (inactive)
    pub fn mark_read(&mut self) {
        self.is_active = false;
    }

    /// Mark chat as having pending messages in the queue
    pub fn mark_has_pending(&mut self) {
        self.has_pending_messages = true;
    }

    /// Mark chat as having no pending messages
    pub fn mark_no_pending(&mut self) {
        self.has_pending_messages = false;
    }

    /// Check if this chat has pending messages
    pub fn has_pending(&self) -> bool {
        self.has_pending_messages
    }
}
