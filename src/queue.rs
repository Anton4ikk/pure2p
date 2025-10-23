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
    fn from_i64(value: i64) -> Option<Self> {
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
    conn: Connection,
    /// Maximum retry attempts
    max_retries: u32,
    /// Base delay for exponential backoff (milliseconds)
    base_delay_ms: i64,
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
                message: Message {
                    id: row.get(0)?,
                    sender: row.get(1)?,
                    recipient: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                    delivered: false,
                },
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
                message: Message {
                    id: row.get(0)?,
                    sender: row.get(1)?,
                    recipient: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                    delivered: false,
                },
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
                message: Message {
                    id: row.get(0)?,
                    sender: row.get(1)?,
                    recipient: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                    delivered: false,
                },
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Helper to create a test message
    fn create_test_message(id: &str, sender: &str, recipient: &str) -> Message {
        Message {
            id: id.to_string(),
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            content: vec![1, 2, 3, 4],
            timestamp: Utc::now().timestamp_millis(),
            delivered: false,
        }
    }

    #[test]
    fn test_queue_creation() {
        let queue = MessageQueue::new().expect("Failed to create queue");
        assert_eq!(queue.size().expect("Failed to get size"), 0);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Urgent > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn test_priority_from_i64() {
        assert_eq!(Priority::from_i64(0), Some(Priority::Low));
        assert_eq!(Priority::from_i64(1), Some(Priority::Normal));
        assert_eq!(Priority::from_i64(2), Some(Priority::High));
        assert_eq!(Priority::from_i64(3), Some(Priority::Urgent));
        assert_eq!(Priority::from_i64(99), None);
    }

    #[test]
    fn test_enqueue_single_message() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg1", "alice", "bob");

        queue
            .enqueue(msg.clone(), Priority::Normal)
            .expect("Failed to enqueue");

        assert_eq!(queue.size().expect("Failed to get size"), 1);

        let messages = queue.list().expect("Failed to list messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message.id, "msg1");
        assert_eq!(messages[0].priority, Priority::Normal);
        // Note: attempts field is now exposed via QueuedMessage
    }

    #[test]
    fn test_enqueue_multiple_messages() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        let msg1 = create_test_message("msg1", "alice", "bob");
        let msg2 = create_test_message("msg2", "bob", "charlie");
        let msg3 = create_test_message("msg3", "charlie", "alice");

        queue
            .enqueue(msg1, Priority::Low)
            .expect("Failed to enqueue msg1");
        queue
            .enqueue(msg2, Priority::High)
            .expect("Failed to enqueue msg2");
        queue
            .enqueue(msg3, Priority::Normal)
            .expect("Failed to enqueue msg3");

        assert_eq!(queue.size().expect("Failed to get size"), 3);
    }

    #[test]
    fn test_fetch_pending_respects_priority() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        // Add messages with different priorities
        let low = create_test_message("low", "alice", "bob");
        let normal = create_test_message("normal", "alice", "bob");
        let high = create_test_message("high", "alice", "bob");
        let urgent = create_test_message("urgent", "alice", "bob");

        queue.enqueue(low, Priority::Low).expect("Failed to enqueue");
        queue
            .enqueue(normal, Priority::Normal)
            .expect("Failed to enqueue");
        queue.enqueue(high, Priority::High).expect("Failed to enqueue");
        queue
            .enqueue(urgent, Priority::Urgent)
            .expect("Failed to enqueue");

        // Fetch pending messages
        let pending = queue.fetch_pending().expect("Failed to fetch pending");

        // Verify they come out in priority order
        assert_eq!(pending.len(), 4);
        assert_eq!(pending[0].message.id, "urgent");
        assert_eq!(pending[1].message.id, "high");
        assert_eq!(pending[2].message.id, "normal");
        assert_eq!(pending[3].message.id, "low");
    }

    #[test]
    fn test_mark_delivered_removes_message() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg1", "alice", "bob");

        queue
            .enqueue(msg, Priority::Normal)
            .expect("Failed to enqueue");
        assert_eq!(queue.size().expect("Failed to get size"), 1);

        queue
            .mark_delivered("msg1")
            .expect("Failed to mark delivered");
        assert_eq!(queue.size().expect("Failed to get size"), 0);
    }

    #[test]
    fn test_mark_delivered_nonexistent_message() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        let result = queue.mark_delivered("nonexistent");
        assert!(result.is_err());
        match result {
            Err(Error::Queue(msg)) => assert!(msg.contains("not found")),
            _ => panic!("Expected Queue error"),
        }
    }

    #[test]
    fn test_mark_failed_increments_attempts() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg1", "alice", "bob");

        queue
            .enqueue(msg, Priority::Normal)
            .expect("Failed to enqueue");

        // First failure
        queue.mark_failed("msg1").expect("Failed to mark failed");

        let messages = queue.list().expect("Failed to list messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].attempts, 1);

        // Second failure
        queue.mark_failed("msg1").expect("Failed to mark failed");

        let messages = queue.list().expect("Failed to list messages");
        assert_eq!(messages[0].attempts, 2);
    }

    #[test]
    fn test_mark_failed_exponential_backoff() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        queue.set_base_delay_ms(100); // 100ms base delay for faster testing

        let msg = create_test_message("msg1", "alice", "bob");
        queue
            .enqueue(msg, Priority::Normal)
            .expect("Failed to enqueue");

        let initial_retry = queue.list().expect("Failed to list")[0].next_retry;

        // Mark as failed
        queue.mark_failed("msg1").expect("Failed to mark failed");

        let messages = queue.list().expect("Failed to list messages");
        let new_retry = messages[0].next_retry;

        // Verify backoff applied (should be roughly 200ms = 100 * 2^1 in the future)
        assert!(
            new_retry > initial_retry,
            "next_retry should increase after failure"
        );
    }

    #[test]
    fn test_mark_failed_removes_after_max_retries() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        queue.set_max_retries(3); // Set low max for testing

        let msg = create_test_message("msg1", "alice", "bob");
        queue
            .enqueue(msg, Priority::Normal)
            .expect("Failed to enqueue");

        // Fail 3 times (0 -> 1 -> 2 -> 3 attempts, remove at 3)
        queue.mark_failed("msg1").expect("Failed to mark failed");
        assert_eq!(queue.size().expect("Failed to get size"), 1);

        queue.mark_failed("msg1").expect("Failed to mark failed");
        assert_eq!(queue.size().expect("Failed to get size"), 1);

        queue.mark_failed("msg1").expect("Failed to mark failed");
        // Should be removed after reaching max_retries
        assert_eq!(queue.size().expect("Failed to get size"), 0);
    }

    #[test]
    fn test_fetch_pending_respects_retry_time() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        let msg = create_test_message("msg1", "alice", "bob");
        queue
            .enqueue(msg, Priority::Normal)
            .expect("Failed to enqueue");

        // Manually update next_retry to far future
        let far_future = Utc::now().timestamp_millis() + 100_000;
        queue
            .conn
            .execute(
                "UPDATE message_queue SET next_retry = ?1 WHERE message_id = ?2",
                params![far_future, "msg1"],
            )
            .expect("Failed to update retry time");

        // Should not be in pending
        let pending = queue.fetch_pending().expect("Failed to fetch pending");
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_clear_removes_all_messages() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        for i in 0..5 {
            let msg = create_test_message(&format!("msg{}", i), "alice", "bob");
            queue
                .enqueue(msg, Priority::Normal)
                .expect("Failed to enqueue");
        }

        assert_eq!(queue.size().expect("Failed to get size"), 5);

        queue.clear().expect("Failed to clear");
        assert_eq!(queue.size().expect("Failed to get size"), 0);
    }

    #[test]
    fn test_list_returns_all_messages_ordered() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        let msg1 = create_test_message("msg1", "alice", "bob");
        let msg2 = create_test_message("msg2", "bob", "charlie");
        let msg3 = create_test_message("msg3", "charlie", "alice");

        queue
            .enqueue(msg1, Priority::Low)
            .expect("Failed to enqueue");
        queue
            .enqueue(msg2, Priority::Urgent)
            .expect("Failed to enqueue");
        queue
            .enqueue(msg3, Priority::Normal)
            .expect("Failed to enqueue");

        let all = queue.list().expect("Failed to list");
        assert_eq!(all.len(), 3);

        // Should be ordered by priority DESC
        assert_eq!(all[0].message.id, "msg2"); // Urgent
        assert_eq!(all[1].message.id, "msg3"); // Normal
        assert_eq!(all[2].message.id, "msg1"); // Low
    }

    #[test]
    fn test_persistent_queue_with_file() {
        use std::fs;
        let db_path = "/tmp/test_queue.db";

        // Clean up if exists
        let _ = fs::remove_file(db_path);

        {
            let mut queue = MessageQueue::new_with_path(db_path).expect("Failed to create queue");
            let msg = create_test_message("msg1", "alice", "bob");
            queue
                .enqueue(msg, Priority::High)
                .expect("Failed to enqueue");
            assert_eq!(queue.size().expect("Failed to get size"), 1);
        }

        // Reopen and verify persistence
        {
            let queue = MessageQueue::new_with_path(db_path).expect("Failed to open queue");
            assert_eq!(queue.size().expect("Failed to get size"), 1);

            let messages = queue.list().expect("Failed to list");
            assert_eq!(messages[0].message.id, "msg1");
            assert_eq!(messages[0].priority, Priority::High);
        }

        // Clean up
        fs::remove_file(db_path).expect("Failed to remove test db");
    }

    #[test]
    fn test_set_max_retries() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        queue.set_max_retries(10);
        assert_eq!(queue.max_retries, 10);
    }

    #[test]
    fn test_set_base_delay_ms() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        queue.set_base_delay_ms(5000);
        assert_eq!(queue.base_delay_ms, 5000);
    }

    #[test]
    fn test_dequeue() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg1", "alice", "bob");

        queue
            .enqueue(msg.clone(), Priority::Normal)
            .expect("Failed to enqueue");

        assert_eq!(queue.size().expect("Failed to get size"), 1);

        // Dequeue the message
        let dequeued = queue.dequeue().expect("Failed to dequeue");
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().message.id, "msg1");

        // Queue should now be empty
        assert_eq!(queue.size().expect("Failed to get size"), 0);
    }

    #[test]
    fn test_dequeue_empty_queue() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        // Dequeue from empty queue should return None
        let dequeued = queue.dequeue().expect("Failed to dequeue");
        assert!(dequeued.is_none());
    }

    #[test]
    fn test_mark_success() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg_success", "alice", "bob");

        queue
            .enqueue(msg.clone(), Priority::Normal)
            .expect("Failed to enqueue");

        assert_eq!(queue.size().expect("Failed to get size"), 1);

        // Mark as success
        queue
            .mark_success("msg_success")
            .expect("Failed to mark success");

        // Message should be removed from queue
        assert_eq!(queue.size().expect("Failed to get size"), 0);
    }

    #[test]
    fn test_schedule_retry() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg_retry", "alice", "bob");

        queue
            .enqueue(msg.clone(), Priority::Normal)
            .expect("Failed to enqueue");

        // Schedule retry with 5 second delay
        queue
            .schedule_retry("msg_retry", 5000)
            .expect("Failed to schedule retry");

        // Message should still be in queue
        assert_eq!(queue.size().expect("Failed to get size"), 1);

        // The message should have updated retry_count (via schedule_retry)
        let messages = queue.list().expect("Failed to list messages");
        assert_eq!(messages[0].message.id, "msg_retry");
    }

    #[test]
    fn test_enqueue_with_type() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg_custom", "alice", "bob");

        queue
            .enqueue_with_type(msg.clone(), Priority::High, "file_transfer")
            .expect("Failed to enqueue with custom type");

        assert_eq!(queue.size().expect("Failed to get size"), 1);

        // Verify message is in queue
        let messages = queue.list().expect("Failed to list messages");
        assert_eq!(messages[0].message.id, "msg_custom");
        assert_eq!(messages[0].priority, Priority::High);
    }

    #[test]
    fn test_mark_failed_updates_retry_count() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg_fail", "alice", "bob");

        queue
            .enqueue(msg.clone(), Priority::Normal)
            .expect("Failed to enqueue");

        // Mark as failed once
        queue
            .mark_failed("msg_fail")
            .expect("Failed to mark as failed");

        // Message should still be in queue
        assert_eq!(queue.size().expect("Failed to get size"), 1);

        // Mark as failed again
        queue
            .mark_failed("msg_fail")
            .expect("Failed to mark as failed");

        assert_eq!(queue.size().expect("Failed to get size"), 1);
    }

    #[test]
    fn test_dequeue_respects_priority() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        let msg_low = create_test_message("msg_low", "alice", "bob");
        let msg_high = create_test_message("msg_high", "bob", "charlie");
        let msg_urgent = create_test_message("msg_urgent", "charlie", "dave");

        queue
            .enqueue(msg_low, Priority::Low)
            .expect("Failed to enqueue low");
        queue
            .enqueue(msg_high, Priority::High)
            .expect("Failed to enqueue high");
        queue
            .enqueue(msg_urgent, Priority::Urgent)
            .expect("Failed to enqueue urgent");

        // Dequeue should return highest priority first
        let first = queue.dequeue().expect("Failed to dequeue").unwrap();
        assert_eq!(first.message.id, "msg_urgent");

        let second = queue.dequeue().expect("Failed to dequeue").unwrap();
        assert_eq!(second.message.id, "msg_high");

        let third = queue.dequeue().expect("Failed to dequeue").unwrap();
        assert_eq!(third.message.id, "msg_low");
    }

    #[test]
    fn test_schedule_retry_global() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg_global", "alice", "bob");

        queue
            .enqueue(msg.clone(), Priority::Normal)
            .expect("Failed to enqueue");

        // Schedule retry with global interval (10 minutes = 600,000 ms)
        let global_interval: u64 = 600_000;
        queue
            .schedule_retry_global("msg_global", global_interval)
            .expect("Failed to schedule global retry");

        // Message should still be in queue
        assert_eq!(queue.size().expect("Failed to get size"), 1);

        // Verify the message is in the queue
        let messages = queue.list().expect("Failed to list messages");
        assert_eq!(messages[0].message.id, "msg_global");
    }

    #[test]
    fn test_schedule_retry_global_with_settings() {
        use crate::storage::Settings;

        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let settings = Settings::default();
        let msg = create_test_message("msg_settings", "alice", "bob");

        queue
            .enqueue(msg.clone(), Priority::Normal)
            .expect("Failed to enqueue");

        // Use global retry interval from settings
        queue
            .schedule_retry_global("msg_settings", settings.global_retry_interval_ms)
            .expect("Failed to schedule retry");

        assert_eq!(queue.size().expect("Failed to get size"), 1);
    }

    #[test]
    fn test_schedule_retry_global_custom_interval() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg_custom_interval", "alice", "bob");

        queue
            .enqueue(msg.clone(), Priority::Normal)
            .expect("Failed to enqueue");

        // Schedule with custom 30-minute interval
        let custom_interval: u64 = 1_800_000; // 30 minutes
        queue
            .schedule_retry_global("msg_custom_interval", custom_interval)
            .expect("Failed to schedule custom retry");

        assert_eq!(queue.size().expect("Failed to get size"), 1);
    }

    #[test]
    fn test_global_retry_vs_exponential_backoff() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        // Message 1: using exponential backoff
        let msg1 = create_test_message("msg_exponential", "alice", "bob");
        queue
            .enqueue(msg1.clone(), Priority::Normal)
            .expect("Failed to enqueue msg1");

        // Mark as failed - uses exponential backoff
        queue
            .mark_failed("msg_exponential")
            .expect("Failed to mark as failed");

        // Message 2: using global retry interval
        let msg2 = create_test_message("msg_global_retry", "bob", "charlie");
        queue
            .enqueue(msg2.clone(), Priority::Normal)
            .expect("Failed to enqueue msg2");

        // Schedule with global interval
        queue
            .schedule_retry_global("msg_global_retry", 600_000)
            .expect("Failed to schedule global retry");

        // Both messages should be in queue
        assert_eq!(queue.size().expect("Failed to get size"), 2);
    }

    #[tokio::test]
    async fn test_fetch_all_pending() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        // Add messages with future retry times
        let msg1 = create_test_message("msg1", "alice", "bob");
        let msg2 = create_test_message("msg2", "bob", "charlie");

        queue
            .enqueue(msg1, Priority::High)
            .expect("Failed to enqueue msg1");
        queue
            .enqueue(msg2, Priority::Normal)
            .expect("Failed to enqueue msg2");

        // Schedule both with future retry times (1 hour from now)
        queue
            .schedule_retry("msg1", 3_600_000)
            .expect("Failed to schedule");
        queue
            .schedule_retry("msg2", 3_600_000)
            .expect("Failed to schedule");

        // fetch_pending should return 0 (not ready yet)
        let pending = queue.fetch_pending().expect("Failed to fetch pending");
        assert_eq!(pending.len(), 0);

        // fetch_all_pending should return both (ignores retry time)
        let all_pending = queue.fetch_all_pending().expect("Failed to fetch all");
        assert_eq!(all_pending.len(), 2);
    }

    #[tokio::test]
    async fn test_retry_pending_on_startup_success() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        // Add test messages
        let msg1 = create_test_message("startup_msg1", "alice", "bob");
        let msg2 = create_test_message("startup_msg2", "bob", "charlie");

        queue
            .enqueue(msg1, Priority::High)
            .expect("Failed to enqueue msg1");
        queue
            .enqueue(msg2, Priority::Normal)
            .expect("Failed to enqueue msg2");

        assert_eq!(queue.size().expect("Failed to get size"), 2);

        // Mock delivery function that always succeeds
        let deliver_fn = |_msg: Message, _recipient: String| async move { Ok(()) };

        // Run startup retry
        let (succeeded, failed) = queue
            .retry_pending_on_startup(deliver_fn)
            .await
            .expect("Failed to retry on startup");

        // Both should succeed
        assert_eq!(succeeded, 2);
        assert_eq!(failed, 0);

        // Queue should be empty (all delivered)
        assert_eq!(queue.size().expect("Failed to get size"), 0);
    }

    #[tokio::test]
    async fn test_retry_pending_on_startup_failure() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        queue.set_max_retries(3); // Set low for testing

        let msg = create_test_message("startup_fail", "alice", "bob");
        queue
            .enqueue(msg, Priority::Normal)
            .expect("Failed to enqueue");

        // Mock delivery function that always fails
        let deliver_fn = |_msg: Message, _recipient: String| async move {
            Err(Error::Transport("Connection refused".to_string()))
        };

        // Run startup retry
        let (succeeded, failed) = queue
            .retry_pending_on_startup(deliver_fn)
            .await
            .expect("Failed to retry on startup");

        // Should fail
        assert_eq!(succeeded, 0);
        assert_eq!(failed, 1);

        // Message should still be in queue (for future retry)
        assert_eq!(queue.size().expect("Failed to get size"), 1);
    }

    #[tokio::test]
    async fn test_retry_pending_on_startup_mixed() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let mut queue = MessageQueue::new().expect("Failed to create queue");

        // Add multiple messages
        let msg1 = create_test_message("mixed_1", "alice", "bob");
        let msg2 = create_test_message("mixed_2", "bob", "charlie");
        let msg3 = create_test_message("mixed_3", "charlie", "dave");

        queue
            .enqueue(msg1, Priority::High)
            .expect("Failed to enqueue msg1");
        queue
            .enqueue(msg2, Priority::Normal)
            .expect("Failed to enqueue msg2");
        queue
            .enqueue(msg3, Priority::Low)
            .expect("Failed to enqueue msg3");

        // Track which messages were attempted
        let attempted = Arc::new(Mutex::new(Vec::new()));
        let attempted_clone = attempted.clone();

        // Mock delivery: succeed for msg1 and msg3, fail for msg2
        let deliver_fn = move |msg: Message, _recipient: String| {
            let attempted = attempted_clone.clone();
            async move {
                attempted.lock().await.push(msg.id.clone());
                if msg.id == "mixed_2" {
                    Err(Error::Transport("Failed".to_string()))
                } else {
                    Ok(())
                }
            }
        };

        // Run startup retry
        let (succeeded, failed) = queue
            .retry_pending_on_startup(deliver_fn)
            .await
            .expect("Failed to retry on startup");

        assert_eq!(succeeded, 2); // msg1 and msg3
        assert_eq!(failed, 1); // msg2

        // Only msg2 should remain in queue
        assert_eq!(queue.size().expect("Failed to get size"), 1);

        let remaining = queue.list().expect("Failed to list");
        assert_eq!(remaining[0].message.id, "mixed_2");

        // Verify all messages were attempted
        let attempted_msgs = attempted.lock().await;
        assert_eq!(attempted_msgs.len(), 3);
    }

    #[tokio::test]
    async fn test_retry_pending_on_startup_empty_queue() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        let deliver_fn = |_msg: Message, _recipient: String| async move { Ok(()) };

        // Run startup retry on empty queue
        let (succeeded, failed) = queue
            .retry_pending_on_startup(deliver_fn)
            .await
            .expect("Failed to retry on startup");

        assert_eq!(succeeded, 0);
        assert_eq!(failed, 0);
    }

    #[test]
    fn test_get_pending_contact_uids_empty() {
        let queue = MessageQueue::new().expect("Failed to create queue");
        let uids = queue.get_pending_contact_uids().expect("Failed to get pending UIDs");

        assert!(uids.is_empty());
    }

    #[test]
    fn test_get_pending_contact_uids_single() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let msg = create_test_message("msg1", "alice", "bob");

        queue.enqueue(msg, Priority::Normal).expect("Failed to enqueue");

        let uids = queue.get_pending_contact_uids().expect("Failed to get pending UIDs");

        assert_eq!(uids.len(), 1);
        assert!(uids.contains("bob")); // recipient is the target_uid
    }

    #[test]
    fn test_get_pending_contact_uids_multiple_same_contact() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        // Multiple messages to the same contact
        let msg1 = create_test_message("msg1", "alice", "bob");
        let msg2 = create_test_message("msg2", "alice", "bob");
        let msg3 = create_test_message("msg3", "charlie", "bob");

        queue.enqueue(msg1, Priority::Normal).expect("Failed to enqueue msg1");
        queue.enqueue(msg2, Priority::High).expect("Failed to enqueue msg2");
        queue.enqueue(msg3, Priority::Low).expect("Failed to enqueue msg3");

        let uids = queue.get_pending_contact_uids().expect("Failed to get pending UIDs");

        // Should only have one unique UID (bob)
        assert_eq!(uids.len(), 1);
        assert!(uids.contains("bob"));
    }

    #[test]
    fn test_get_pending_contact_uids_multiple_contacts() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        // Messages to different contacts
        let msg1 = create_test_message("msg1", "alice", "bob");
        let msg2 = create_test_message("msg2", "alice", "charlie");
        let msg3 = create_test_message("msg3", "bob", "david");

        queue.enqueue(msg1, Priority::Normal).expect("Failed to enqueue msg1");
        queue.enqueue(msg2, Priority::Normal).expect("Failed to enqueue msg2");
        queue.enqueue(msg3, Priority::Normal).expect("Failed to enqueue msg3");

        let uids = queue.get_pending_contact_uids().expect("Failed to get pending UIDs");

        // Should have three unique UIDs
        assert_eq!(uids.len(), 3);
        assert!(uids.contains("bob"));
        assert!(uids.contains("charlie"));
        assert!(uids.contains("david"));
    }

    #[test]
    fn test_get_pending_contact_uids_after_delivery() {
        let mut queue = MessageQueue::new().expect("Failed to create queue");

        let msg1 = create_test_message("msg1", "alice", "bob");
        let msg2 = create_test_message("msg2", "alice", "charlie");

        queue.enqueue(msg1, Priority::Normal).expect("Failed to enqueue msg1");
        queue.enqueue(msg2, Priority::Normal).expect("Failed to enqueue msg2");

        // Verify both contacts have pending messages
        let uids = queue.get_pending_contact_uids().expect("Failed to get pending UIDs");
        assert_eq!(uids.len(), 2);

        // Mark one as delivered
        queue.mark_delivered("msg1").expect("Failed to mark delivered");

        // Should only have charlie now
        let uids = queue.get_pending_contact_uids().expect("Failed to get pending UIDs");
        assert_eq!(uids.len(), 1);
        assert!(uids.contains("charlie"));
        assert!(!uids.contains("bob"));
    }

    #[test]
    fn test_integration_appstate_queue_sync() {
        use crate::storage::AppState;

        let mut queue = MessageQueue::new().expect("Failed to create queue");
        let mut app_state = AppState::new();

        // Create chats
        app_state.add_chat("alice".to_string());
        app_state.add_chat("bob".to_string());
        app_state.add_chat("charlie".to_string());

        // Enqueue messages to some contacts
        let msg1 = create_test_message("msg1", "me", "alice");
        let msg2 = create_test_message("msg2", "me", "alice");
        let msg3 = create_test_message("msg3", "me", "charlie");

        queue.enqueue(msg1, Priority::Normal).expect("Failed to enqueue");
        queue.enqueue(msg2, Priority::Normal).expect("Failed to enqueue");
        queue.enqueue(msg3, Priority::Normal).expect("Failed to enqueue");

        // Get pending UIDs from queue
        let pending_uids = queue.get_pending_contact_uids().expect("Failed to get pending UIDs");

        // Sync app state
        app_state.sync_pending_status(&pending_uids);

        // Verify flags
        assert!(app_state.get_chat("alice").unwrap().has_pending_messages);
        assert!(!app_state.get_chat("bob").unwrap().has_pending_messages);
        assert!(app_state.get_chat("charlie").unwrap().has_pending_messages);
    }
}
