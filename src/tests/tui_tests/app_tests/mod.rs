//! App business logic tests
//!
//! This module contains tests for the App struct's business logic organized by feature area:
//! - `helpers` - Shared test utilities
//! - `initialization` - App creation, state loading, settings (6 tests)
//! - `navigation` - Screen transitions, menu navigation (14 tests)
//! - `contact_import` - Import validation, duplicate detection (3 tests)
//! - `chat_management` - Chat creation, deletion, selection (14 tests)
//! - `messaging` - Message sending (3 tests)
//! - `startup` - Startup sync, connectivity (4 tests)
//!
//! Total: 44 tests

mod helpers;
mod initialization_tests;
mod navigation_tests;
mod contact_import_tests;
mod chat_management_tests;
mod messaging_tests;
mod startup_tests;
