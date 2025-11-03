// Storage Tests Module - Testing the storage module
// Tests organized by storage module functionality:
// - contact_tests: Contact struct and methods (creation, expiry, activation, serialization)
// - token_tests: Token generation and parsing (roundtrip, validation, crypto integration)
// - chat_tests: Chat and Message structs (append, active management, pending flags)
// - app_state_tests: AppState struct (save/load, sync, chat management)
// - settings_tests: Settings and SettingsManager (defaults, persistence, concurrent access)
// - request_log_tests: Request logging for network debugging (log CRUD, filtering, cleanup)

mod contact_tests;
mod token_tests;
mod chat_tests;
mod app_state_tests;
mod settings_tests;
mod request_log_tests;
