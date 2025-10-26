//! Message structures and delivery status tracking

use serde::{Deserialize, Serialize};

/// Message delivery status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    /// Message sent but not yet delivered
    Sent,
    /// Message delivered to recipient
    Delivered,
    /// Message is queued for retry
    Pending,
    /// Message delivery failed after max retries
    Failed,
}

impl Default for DeliveryStatus {
    fn default() -> Self {
        Self::Sent
    }
}

/// Represents a stored message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID
    pub id: String,
    /// Sender peer ID
    pub sender: String,
    /// Recipient peer ID
    pub recipient: String,
    /// Message content (encrypted)
    pub content: Vec<u8>,
    /// Timestamp
    pub timestamp: i64,
    /// Delivery status (legacy - kept for backward compatibility)
    #[serde(default)]
    pub delivered: bool,
    /// Detailed delivery status
    #[serde(default)]
    pub delivery_status: DeliveryStatus,
    /// Next retry timestamp (Unix milliseconds), if pending
    #[serde(default)]
    pub next_retry_at: Option<i64>,
    /// Number of delivery attempts
    #[serde(default)]
    pub attempts: u32,
}

impl Message {
    /// Create a new message
    pub fn new(
        id: String,
        sender: String,
        recipient: String,
        content: Vec<u8>,
        timestamp: i64,
    ) -> Self {
        Self {
            id,
            sender,
            recipient,
            content,
            timestamp,
            delivered: false,
            delivery_status: DeliveryStatus::Sent,
            next_retry_at: None,
            attempts: 0,
        }
    }

    /// Mark message as delivered
    pub fn mark_delivered(&mut self) {
        self.delivered = true;
        self.delivery_status = DeliveryStatus::Delivered;
        self.next_retry_at = None;
    }

    /// Mark message as pending retry
    pub fn mark_pending(&mut self, next_retry_at: i64) {
        self.delivery_status = DeliveryStatus::Pending;
        self.next_retry_at = Some(next_retry_at);
        self.attempts += 1;
    }

    /// Mark message as failed
    pub fn mark_failed(&mut self) {
        self.delivery_status = DeliveryStatus::Failed;
        self.next_retry_at = None;
    }

    /// Get time until next retry in seconds
    pub fn time_until_retry(&self) -> Option<i64> {
        self.next_retry_at.map(|retry_at| {
            let now = chrono::Utc::now().timestamp_millis();
            let seconds = (retry_at - now) / 1000;
            seconds.max(0)
        })
    }

    /// Get human-readable delivery status indicator
    pub fn status_indicator(&self) -> &str {
        match self.delivery_status {
            DeliveryStatus::Sent => "✓",
            DeliveryStatus::Delivered => "✓✓",
            DeliveryStatus::Pending => "↻",
            DeliveryStatus::Failed => "✗",
        }
    }

    /// Get full status text with retry countdown if applicable
    pub fn status_text(&self) -> String {
        match self.delivery_status {
            DeliveryStatus::Sent => "sent".to_string(),
            DeliveryStatus::Delivered => "delivered".to_string(),
            DeliveryStatus::Pending => {
                if let Some(seconds) = self.time_until_retry() {
                    format!("retry in {}", format_retry_time(seconds))
                } else {
                    "pending".to_string()
                }
            }
            DeliveryStatus::Failed => "failed".to_string(),
        }
    }
}

/// Format retry time as human-readable string
fn format_retry_time(seconds: i64) -> String {
    if seconds >= 3600 {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    } else if seconds >= 60 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}s", seconds)
    }
}
