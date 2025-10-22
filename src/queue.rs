//! Message queue module
//!
//! This module handles message queuing and delivery including:
//! - Outgoing message queue
//! - Retry logic for failed deliveries
//! - Priority handling
//! - Queue persistence

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
                sender TEXT NOT NULL,
                recipient TEXT NOT NULL,
                content BLOB NOT NULL,
                timestamp INTEGER NOT NULL,
                priority INTEGER NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
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

        self.conn.execute(
            "INSERT INTO message_queue
             (message_id, sender, recipient, content, timestamp, priority, attempts, next_retry, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?7)",
            params![
                message.id,
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
            "SELECT message_id, sender, recipient, content, timestamp, priority, attempts, next_retry
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

    /// Mark a message as failed and schedule retry with exponential backoff
    pub fn mark_failed(&mut self, message_id: &str) -> Result<()> {
        // Get current attempts
        let attempts: u32 = self.conn.query_row(
            "SELECT attempts FROM message_queue WHERE message_id = ?1",
            params![message_id],
            |row| row.get(0),
        )?;

        let new_attempts = attempts + 1;

        // Check if we've exceeded max retries
        if new_attempts >= self.max_retries {
            // Remove from queue - too many failures
            self.conn.execute(
                "DELETE FROM message_queue WHERE message_id = ?1",
                params![message_id],
            )?;
            return Ok(());
        }

        // Calculate next retry time using exponential backoff
        let delay = self.base_delay_ms * 2_i64.pow(new_attempts);
        let next_retry = Utc::now().timestamp_millis() + delay;

        // Update the message with new attempt count and retry time
        self.conn.execute(
            "UPDATE message_queue
             SET attempts = ?1, next_retry = ?2
             WHERE message_id = ?3",
            params![new_attempts, next_retry, message_id],
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
            "SELECT message_id, sender, recipient, content, timestamp, priority, attempts, next_retry
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

    /// Set maximum retry attempts
    pub fn set_max_retries(&mut self, max_retries: u32) {
        self.max_retries = max_retries;
    }

    /// Set base delay for exponential backoff (in milliseconds)
    pub fn set_base_delay_ms(&mut self, base_delay_ms: i64) {
        self.base_delay_ms = base_delay_ms;
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
        assert_eq!(messages[0].attempts, 0);
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
}
