//! Message queue module
//!
//! This module handles message queuing and delivery including:
//! - Outgoing message queue
//! - Retry logic for failed deliveries
//! - Priority handling
//! - Queue persistence
//! - Startup retry for pending messages
//!
//! ## Retry on Startup
//!
//! When the application starts, all pending messages in the queue should be
//! retried immediately using `retry_pending_on_startup()`. This ensures that
//! messages queued from previous sessions are delivered as soon as possible.
//!
//! ### Flow:
//! 1. App launches and initializes MessageQueue
//! 2. Call `retry_pending_on_startup()` with async delivery function
//! 3. All queued messages are fetched (ignoring retry time)
//! 4. Messages are processed in priority order
//! 5. For each message:
//!    - Async POST request to recipient's `/output` endpoint
//!    - On success: Remove from queue via `mark_success()`
//!    - On failure: Update retry count via `mark_failed()`
//! 6. Failed messages remain in queue for future retry attempts
//!
//! ### Example:
//! ```rust,no_run
//! use pure2p::queue::MessageQueue;
//! use pure2p::storage::Message;
//!
//! # async fn example() -> pure2p::Result<()> {
//! let mut queue = MessageQueue::new()?;
//!
//! // Define delivery function that performs HTTP POST
//! let deliver = |msg: Message, recipient: String| async move {
//!     // In a real implementation:
//!     // 1. Build MessageEnvelope from msg
//!     // 2. Serialize to CBOR
//!     // 3. POST to http://{recipient}/output
//!     // 4. Return Ok(()) on success, Err on failure
//!
//!     // Simplified example:
//!     println!("Delivering message {} to {}", msg.id, recipient);
//!     Ok(())
//! };
//!
//! // Retry all pending messages on startup
//! let (succeeded, failed) = queue.retry_pending_on_startup(deliver).await?;
//! println!("Delivered {} messages, {} failed", succeeded, failed);
//! # Ok(())
//! # }
//! ```

use crate::{storage::Message, Error, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Low priority message
    Low = 0,
    /// Normal priority message
    Normal = 1,
    /// High priority message
    High = 2,
    /// Urgent priority message
    Urgent = 3,
}

impl Priority {
    /// Convert from integer
    pub(crate) fn from_i64(value: i64) -> Option<Self> {
        match value {
            0 => Some(Priority::Low),
            1 => Some(Priority::Normal),
            2 => Some(Priority::High),
            3 => Some(Priority::Urgent),
            _ => None,
        }
    }
}

/// Queued message with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedMessage {
    /// The message itself
    pub message: Message,
    /// Priority level
    pub priority: Priority,
    /// Number of delivery attempts
    pub attempts: u32,
    /// Next retry timestamp (Unix milliseconds)
    pub next_retry: i64,
}

/// Message queue manager with SQLite persistence
pub struct MessageQueue {
    /// SQLite connection
    pub(crate) conn: Connection,
    /// Maximum retry attempts
    pub(crate) max_retries: u32,
    /// Base delay for exponential backoff (milliseconds)
    pub(crate) base_delay_ms: i64,
}

impl MessageQueue {
    /// Create a new message queue with in-memory database
    pub fn new() -> Result<Self> {
        Self::new_with_connection(Connection::open_in_memory()?)
    }

    /// Create a new message queue with a file-based database
    pub fn new_with_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::new_with_connection(conn)
    }

    /// Create a new message queue with a provided connection
    fn new_with_connection(conn: Connection) -> Result<Self> {
        let mut queue = Self {
            conn,
            max_retries: 5,
            base_delay_ms: 1000, // 1 second base delay
        };
        queue.init_schema()?;
        Ok(queue)
    }

    /// Initialize the database schema
    fn init_schema(&mut self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS message_queue (
                message_id TEXT PRIMARY KEY,
                target_uid TEXT NOT NULL,
                message_type TEXT NOT NULL,
                payload BLOB NOT NULL,
                last_attempt INTEGER,
                retry_count INTEGER NOT NULL DEFAULT 0,
                sender TEXT NOT NULL,
                recipient TEXT NOT NULL,
                content BLOB NOT NULL,
                timestamp INTEGER NOT NULL,
                priority INTEGER NOT NULL,
                next_retry INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create index for efficient priority-based fetching
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_queue_priority_retry
             ON message_queue(priority DESC, next_retry ASC)",
            [],
        )?;

        Ok(())
    }

    /// Add a message to the queue
    pub fn enqueue(&mut self, message: Message, priority: Priority) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        let message_type = "text"; // Default message type

        self.conn.execute(
            "INSERT INTO message_queue
             (message_id, target_uid, message_type, payload, last_attempt, retry_count,
              sender, recipient, content, timestamp, priority, next_retry, created_at)
             VALUES (?1, ?2, ?3, ?4, NULL, 0, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                message.id,
                message.recipient, // target_uid
                message_type,
                message.content, // payload
                message.sender,
                message.recipient,
                message.content,
                message.timestamp,
                priority as i64,
                now,
            ],
        )?;

        Ok(())
    }

    /// Add a message to the queue with custom message type
    pub fn enqueue_with_type(
        &mut self,
        message: Message,
        priority: Priority,
        message_type: &str,
    ) -> Result<()> {
        let now = Utc::now().timestamp_millis();

        self.conn.execute(
            "INSERT INTO message_queue
             (message_id, target_uid, message_type, payload, last_attempt, retry_count,
              sender, recipient, content, timestamp, priority, next_retry, created_at)
             VALUES (?1, ?2, ?3, ?4, NULL, 0, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                message.id,
                message.recipient, // target_uid
                message_type,
                message.content, // payload
                message.sender,
                message.recipient,
                message.content,
                message.timestamp,
                priority as i64,
                now,
            ],
        )?;

        Ok(())
    }

    /// Get pending messages ready for delivery (ordered by priority and retry time)
    pub fn fetch_pending(&self) -> Result<Vec<QueuedMessage>> {
        let now = Utc::now().timestamp_millis();

        let mut stmt = self.conn.prepare(
            "SELECT message_id, sender, recipient, content, timestamp, priority, retry_count, next_retry
             FROM message_queue
             WHERE next_retry <= ?1
             ORDER BY priority DESC, next_retry ASC",
        )?;

        let rows = stmt.query_map(params![now], |row| {
            let priority_val: i64 = row.get(5)?;
            let priority = Priority::from_i64(priority_val)
                .unwrap_or(Priority::Normal);

            Ok(QueuedMessage {
                message: Message::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ),
                priority,
                attempts: row.get(6)?,
                next_retry: row.get(7)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }

        Ok(messages)
    }

    /// Get all pending messages for startup retry (ignores retry time)
    ///
    /// This is used on app startup to immediately retry all queued messages
    /// regardless of their scheduled retry time.
    pub fn fetch_all_pending(&self) -> Result<Vec<QueuedMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT message_id, sender, recipient, content, timestamp, priority, retry_count, next_retry
             FROM message_queue
             ORDER BY priority DESC, next_retry ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let priority_val: i64 = row.get(5)?;
            let priority = Priority::from_i64(priority_val)
                .unwrap_or(Priority::Normal);

            Ok(QueuedMessage {
                message: Message::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ),
                priority,
                attempts: row.get(6)?,
                next_retry: row.get(7)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }

        Ok(messages)
    }

    /// Dequeue: fetch the next pending message and remove it from the queue
    pub fn dequeue(&mut self) -> Result<Option<QueuedMessage>> {
        let pending = self.fetch_pending()?;

        if let Some(msg) = pending.first() {
            // Remove from queue
            self.conn.execute(
                "DELETE FROM message_queue WHERE message_id = ?1",
                params![&msg.message.id],
            )?;
            Ok(Some(msg.clone()))
        } else {
            Ok(None)
        }
    }

    /// Mark a message as successfully delivered and remove from queue
    pub fn mark_delivered(&mut self, message_id: &str) -> Result<()> {
        let deleted = self.conn.execute(
            "DELETE FROM message_queue WHERE message_id = ?1",
            params![message_id],
        )?;

        if deleted == 0 {
            return Err(Error::Queue(format!(
                "Message not found in queue: {}",
                message_id
            )));
        }

        Ok(())
    }

    /// Mark a message as successfully delivered (alias for mark_delivered)
    pub fn mark_success(&mut self, message_id: &str) -> Result<()> {
        self.mark_delivered(message_id)
    }

    /// Mark a message as failed and schedule retry with exponential backoff
    pub fn mark_failed(&mut self, message_id: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();

        // Get current retry_count
        let retry_count: u32 = self.conn.query_row(
            "SELECT retry_count FROM message_queue WHERE message_id = ?1",
            params![message_id],
            |row| row.get(0),
        )?;

        let new_retry_count = retry_count + 1;

        // Check if we've exceeded max retries
        if new_retry_count >= self.max_retries {
            // Remove from queue - too many failures
            self.conn.execute(
                "DELETE FROM message_queue WHERE message_id = ?1",
                params![message_id],
            )?;
            return Ok(());
        }

        // Calculate next retry time using exponential backoff
        let delay = self.base_delay_ms * 2_i64.pow(new_retry_count);
        let next_retry = now + delay;

        // Update the message with new retry count, last attempt time, and next retry time
        self.conn.execute(
            "UPDATE message_queue
             SET retry_count = ?1, last_attempt = ?2, next_retry = ?3
             WHERE message_id = ?4",
            params![new_retry_count, now, next_retry, message_id],
        )?;

        Ok(())
    }

    /// Schedule a retry for a message with custom delay
    pub fn schedule_retry(&mut self, message_id: &str, delay_ms: i64) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        let next_retry = now + delay_ms;

        // Increment retry count and update last_attempt and next_retry
        self.conn.execute(
            "UPDATE message_queue
             SET retry_count = retry_count + 1, last_attempt = ?1, next_retry = ?2
             WHERE message_id = ?3",
            params![now, next_retry, message_id],
        )?;

        Ok(())
    }

    /// Schedule a retry using global retry interval from settings
    ///
    /// This method uses the global retry interval instead of exponential backoff.
    /// Useful for periodic retry attempts at a fixed interval.
    pub fn schedule_retry_global(&mut self, message_id: &str, global_interval_ms: u64) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        let next_retry = now + global_interval_ms as i64;

        // Increment retry count and update last_attempt and next_retry
        self.conn.execute(
            "UPDATE message_queue
             SET retry_count = retry_count + 1, last_attempt = ?1, next_retry = ?2
             WHERE message_id = ?3",
            params![now, next_retry, message_id],
        )?;

        Ok(())
    }

    /// Get the current queue size
    pub fn size(&self) -> Result<usize> {
        let count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM message_queue",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get count of pending messages (alias for size, for clarity in UI)
    pub fn count_pending(&self) -> Result<usize> {
        self.size()
    }

    /// Clear all messages from the queue
    pub fn clear(&mut self) -> Result<()> {
        self.conn.execute("DELETE FROM message_queue", [])?;
        Ok(())
    }

    /// Get all messages in the queue (for inspection/debugging)
    pub fn list(&self) -> Result<Vec<QueuedMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT message_id, sender, recipient, content, timestamp, priority, retry_count, next_retry
             FROM message_queue
             ORDER BY priority DESC, next_retry ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let priority_val: i64 = row.get(5)?;
            let priority = Priority::from_i64(priority_val)
                .unwrap_or(Priority::Normal);

            Ok(QueuedMessage {
                message: Message::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ),
                priority,
                attempts: row.get(6)?,
                next_retry: row.get(7)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }

        Ok(messages)
    }

    /// Get unique contact UIDs (target_uid) that have pending messages in the queue
    ///
    /// This is useful for syncing the `has_pending_messages` flag in chats.
    ///
    /// # Returns
    /// A HashSet of contact UIDs with pending messages
    ///
    /// # Example
    /// ```rust,no_run
    /// use pure2p::queue::MessageQueue;
    ///
    /// # fn example() -> pure2p::Result<()> {
    /// let queue = MessageQueue::new()?;
    /// let pending_uids = queue.get_pending_contact_uids()?;
    ///
    /// for uid in pending_uids {
    ///     println!("Contact {} has pending messages", uid);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_pending_contact_uids(&self) -> Result<std::collections::HashSet<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT target_uid FROM message_queue",
        )?;

        let rows = stmt.query_map([], |row| {
            let uid: String = row.get(0)?;
            Ok(uid)
        })?;

        let mut uids = std::collections::HashSet::new();
        for row in rows {
            uids.insert(row?);
        }

        Ok(uids)
    }

    /// Set maximum retry attempts
    pub fn set_max_retries(&mut self, max_retries: u32) {
        self.max_retries = max_retries;
    }

    /// Set base delay for exponential backoff (in milliseconds)
    pub fn set_base_delay_ms(&mut self, base_delay_ms: i64) {
        self.base_delay_ms = base_delay_ms;
    }

    /// Retry all pending messages on startup
    ///
    /// This method fetches all queued messages and attempts to deliver them
    /// by calling the provided async delivery function. Messages are processed
    /// in order of priority.
    ///
    /// # Arguments
    /// * `deliver_fn` - Async function that attempts to deliver a message.
    ///                  Should return Ok(()) on success, Err on failure.
    ///
    /// # Returns
    /// A tuple of (succeeded_count, failed_count)
    pub async fn retry_pending_on_startup<F, Fut>(
        &mut self,
        deliver_fn: F,
    ) -> Result<(usize, usize)>
    where
        F: Fn(Message, String) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let pending_messages = self.fetch_all_pending()?;
        let total = pending_messages.len();

        if total == 0 {
            return Ok((0, 0));
        }

        tracing::info!("Retrying {} pending messages on startup", total);

        let mut succeeded = 0;
        let mut failed = 0;

        for queued_msg in pending_messages {
            let message_id = queued_msg.message.id.clone();
            let recipient = queued_msg.message.recipient.clone();

            // Attempt delivery
            match deliver_fn(queued_msg.message.clone(), recipient).await {
                Ok(()) => {
                    // Success: mark as delivered and remove from queue
                    if let Err(e) = self.mark_success(&message_id) {
                        tracing::error!("Failed to mark message {} as delivered: {}", message_id, e);
                    } else {
                        succeeded += 1;
                        tracing::debug!("Message {} delivered successfully on startup", message_id);
                    }
                }
                Err(e) => {
                    // Failure: mark as failed (will schedule retry)
                    tracing::warn!("Failed to deliver message {} on startup: {}", message_id, e);
                    if let Err(e) = self.mark_failed(&message_id) {
                        tracing::error!("Failed to update retry status for {}: {}", message_id, e);
                    }
                    failed += 1;
                }
            }
        }

        tracing::info!(
            "Startup retry complete: {} succeeded, {} failed out of {} total",
            succeeded,
            failed,
            total
        );

        Ok((succeeded, failed))
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new().expect("Failed to create default MessageQueue")
    }
}

