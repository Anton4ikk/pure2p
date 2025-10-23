//! Pure2P - A pure peer-to-peer messaging system
//!
//! This library provides the core functionality for Pure2P, a decentralized
//! messaging system designed for cross-platform compatibility (Android, iOS, Desktop).

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod crypto;
pub mod protocol;
pub mod transport;
pub mod storage;
pub mod queue;

/// Result type alias for Pure2P operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for Pure2P operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Cryptographic operation error
    #[error("Crypto error: {0}")]
    Crypto(String),

    /// Transport layer error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Storage operation error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Message queue error
    #[error("Queue error: {0}")]
    Queue(String),

    /// General I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    /// CBOR serialization error
    #[error("CBOR serialization error: {0}")]
    CborSerialization(String),

    /// SQLite database error
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// HTTP/Hyper error
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),
}

/// Initialize the Pure2P library with logging
pub fn init() {
    tracing_subscriber::fmt::init();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_library_init() {
        // Basic test to ensure library compiles
        assert!(true);
    }
}
