// TUI Tests Module - Testing the public tui module
// Tests organized by TUI module structure:
// - app_tests: App struct and business logic (36 tests)
// - screen_tests: All screen structs, modularized by screen type (82 tests)
//   - share_contact_tests: ShareContactScreen (5 tests)
//   - import_contact_tests: ImportContactScreen (10 tests)
//   - chat_list_tests: ChatListScreen (5 tests)
//   - chat_view_tests: ChatViewScreen (3 tests)
//   - settings_tests: SettingsScreen (9 tests)
//   - startup_sync_tests: StartupSyncScreen (10 tests)
//   - diagnostics_tests: DiagnosticsScreen (20 tests)
//   - status_indicators_tests: Status badges and expiry (10 tests)
// - types_tests: MenuItem enum and related types (3 tests)
// - ui_tests: UI helper functions (4 tests)

mod app_tests;
mod screen_tests;
mod types_tests;
mod ui_tests;
