//! TUI (Terminal User Interface) module
//!
//! This module contains all TUI logic separated from the binary for better testability
//! and potential reuse in other UI implementations.

pub mod types;
pub mod screens;
pub mod app;
pub mod ui;

// Re-export main types for convenience
pub use types::{Screen, MenuItem};
pub use screens::*;
pub use app::App;
