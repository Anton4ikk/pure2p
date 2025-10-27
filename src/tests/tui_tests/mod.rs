// TUI Tests Module - Testing the public tui module
// Tests organized by TUI module structure:
// - app_tests: App struct and business logic, modularized (44 tests)
//   - initialization: App creation, state loading (6 tests)
//   - navigation: Screen transitions, menu navigation (14 tests)
//   - contact_import: Import validation, duplicate detection (3 tests)
//   - chat_management: Chat creation, deletion, selection (14 tests)
//   - messaging: Message sending (3 tests)
//   - startup: Startup sync, connectivity (4 tests)
// - screen_tests: All screen structs, modularized by screen type (76 tests)
//   - share_contact_tests: ShareContactScreen (5 tests)
//   - import_contact_tests: ImportContactScreen (10 tests)
//   - chat_list_tests: ChatListScreen (5 tests)
//   - chat_view_tests: ChatViewScreen (3 tests)
//   - settings_tests: SettingsScreen (10 tests)
//   - startup_sync_tests: StartupSyncScreen (10 tests)
//   - diagnostics_tests: DiagnosticsScreen (20 tests)
//   - status_indicators_tests: Status badges and expiry (10 tests)
// - types_tests: MenuItem enum and related types (3 tests)
// - ui_tests: UI helper functions (4 tests)

mod app_tests;
mod screen_tests;
mod types_tests;
mod ui_tests;
