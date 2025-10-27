//! Clipboard abstraction for testing
//!
//! Provides a trait-based interface to clipboard operations that can be mocked in tests.

use std::fmt;

/// Result type for clipboard operations
pub type ClipboardResult<T> = Result<T, ClipboardError>;

/// Clipboard error types
#[derive(Debug, Clone)]
pub enum ClipboardError {
    /// Clipboard initialization failed
    InitFailed(String),
    /// Copy/paste operation failed
    OperationFailed(String),
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardError::InitFailed(msg) => write!(f, "Clipboard init failed: {}", msg),
            ClipboardError::OperationFailed(msg) => write!(f, "Clipboard operation failed: {}", msg),
        }
    }
}

impl std::error::Error for ClipboardError {}

/// Trait for clipboard operations (allows mocking in tests)
pub trait ClipboardProvider {
    /// Copy text to clipboard
    fn set_text(&mut self, text: &str) -> ClipboardResult<()>;

    /// Get text from clipboard
    fn get_text(&mut self) -> ClipboardResult<String>;
}

/// Real clipboard implementation using arboard
pub struct RealClipboard {
    inner: arboard::Clipboard,
}

impl RealClipboard {
    /// Create new real clipboard instance
    pub fn new() -> ClipboardResult<Self> {
        match arboard::Clipboard::new() {
            Ok(clipboard) => Ok(Self { inner: clipboard }),
            Err(e) => Err(ClipboardError::InitFailed(e.to_string())),
        }
    }
}

impl ClipboardProvider for RealClipboard {
    fn set_text(&mut self, text: &str) -> ClipboardResult<()> {
        self.inner
            .set_text(text)
            .map_err(|e| ClipboardError::OperationFailed(e.to_string()))
    }

    fn get_text(&mut self) -> ClipboardResult<String> {
        self.inner
            .get_text()
            .map_err(|e| ClipboardError::OperationFailed(e.to_string()))
    }
}

#[cfg(test)]
/// Mock clipboard implementation for testing
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Mock clipboard for testing (thread-safe)
    #[derive(Clone)]
    pub struct MockClipboard {
        content: Arc<Mutex<Option<String>>>,
        should_fail: Arc<Mutex<bool>>,
    }

    impl MockClipboard {
        /// Create new mock clipboard
        pub fn new() -> Self {
            Self {
                content: Arc::new(Mutex::new(None)),
                should_fail: Arc::new(Mutex::new(false)),
            }
        }

        /// Create mock that always fails (simulates SSH environment)
        pub fn new_failing() -> Self {
            Self {
                content: Arc::new(Mutex::new(None)),
                should_fail: Arc::new(Mutex::new(true)),
            }
        }

        /// Set whether operations should fail
        pub fn set_should_fail(&mut self, should_fail: bool) {
            *self.should_fail.lock().unwrap() = should_fail;
        }

        /// Get current clipboard content (for test assertions)
        pub fn get_content(&self) -> Option<String> {
            self.content.lock().unwrap().clone()
        }
    }

    impl ClipboardProvider for MockClipboard {
        fn set_text(&mut self, text: &str) -> ClipboardResult<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(ClipboardError::OperationFailed("Mock clipboard unavailable".to_string()));
            }
            *self.content.lock().unwrap() = Some(text.to_string());
            Ok(())
        }

        fn get_text(&mut self) -> ClipboardResult<String> {
            if *self.should_fail.lock().unwrap() {
                return Err(ClipboardError::OperationFailed("Mock clipboard unavailable".to_string()));
            }
            self.content
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| ClipboardError::OperationFailed("Clipboard is empty".to_string()))
        }
    }
}
