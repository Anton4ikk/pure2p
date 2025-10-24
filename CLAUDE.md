# CLAUDE.md

Guide for Claude Code when working with this repository.

> **For Humans**: See [DEVELOPMENT.md](DEVELOPMENT.md) for setup, [README.md](README.md) for overview.

## Project Overview

**Pure2P** - Radically honest P2P messenger with no servers/relays. Direct peer-to-peer only.

- Each client exposes `POST /output` endpoint for receiving messages
- UIDs derived from Ed25519 public keys, shared manually
- Local-only storage (no sync/cloud)
- Messages queue locally until both peers online
- Rust library (`lib`, `staticlib`, `cdylib`) for cross-platform use

## Quick Commands

```bash
# Build & Run
cargo build --release
cargo run --bin pure2p-tui

# Test & Quality
cargo test
cargo clippy -- -D warnings
cargo fmt
```

## Core Modules

**`crypto`** - Ed25519 keypairs, SHA-256 UID generation, sign/verify

**`protocol`** - CBOR/JSON message envelopes with UUID, version, timestamps, message types (Text, Delete)

**`transport`** - HTTP/1.1 server with `/output`, `/ping`, `/message` endpoints. Peer management, delivery tracking.

**`storage`** - Contact/Chat structures, token generation (base64 CBOR), AppState persistence (JSON/CBOR), Settings with auto-save

**`queue`** - SQLite-backed retry queue, priority ordering, exponential backoff, startup retry

**`messaging`** - High-level API combining transport/queue/storage. Send with auto-queue, chat lifecycle, smart deletion

**`connectivity`** - Port forwarding protocols (PCP, NAT-PMP, UPnP). Auto-discovery, mapping management, diagnostics

**`tui`** - Terminal UI module (library, not binary). Reusable across platforms:
- `types.rs` - Screen and MenuItem enums
- `screens.rs` - All screen state structs (ShareContact, ImportContact, ChatList, ChatView, Settings, Diagnostics, StartupSync)
- `app.rs` - Main App struct with business logic
- `ui.rs` - Rendering functions (ratatui-based)

## Data Structures

**Contact** - `uid`, `ip`, `pubkey`, `expiry`, `is_active`. Methods: `is_expired()`, `activate()`, `deactivate()`

**Chat** - `contact_uid`, `messages[]`, `is_active`, `has_pending_messages`. Methods: `append_message()`, `mark_unread()`, `mark_has_pending()`

**AppState** - `contacts[]`, `chats[]`, `message_queue[]`, `settings`. Methods: `get_chat()`, `sync_pending_status()`, `save()`/`load()`

**Settings** - Retry intervals, storage path, contact expiry, max retries. Auto-save to JSON. Thread-safe SettingsManager for UI.

## TUI Architecture

**Binary (`src/bin/tui.rs`)** - Thin wrapper (~280 lines):
- `main()` - Terminal initialization/cleanup
- `run_app()` - Event loop with 100ms polling
- Keyboard mapping to App methods

**Library (`src/tui/`)** - Reusable UI logic:
- Used by TUI binary, future mobile/desktop UIs
- Fully tested (90 unit tests)
- Platform-agnostic business logic

**Screens:**
1. **StartupSync** - Progress bar for pending queue (✓/✗ counters, elapsed time)
2. **MainMenu** - Navigate features (↑↓/j/k, Enter)
3. **ShareContact** - Generate tokens (copy/save), shows UID/IP
4. **ImportContact** - Parse/validate tokens, expiry check
5. **ChatList** - Status badges (⚠ Expired | ⌛ Pending | ● New | ○ Read), delete with confirmation
6. **ChatView** - Message history (scroll ↑↓), send with Enter
7. **Settings** - Edit retry interval (1-1440 min), auto-save with toast
8. **Diagnostics** - Port forwarding status (PCP, NAT-PMP, UPnP)

**Keyboard:** q/Esc=back, ↑↓/j/k=nav, Enter=select, d/Del=delete, Backspace/Delete for input

**Colors:** Cyan=titles, Green=success/active, Yellow=warning/pending, Red=error/expired, Gray=inactive

## Implementation Notes

### Crypto
- UIDs deterministic (same pubkey → same UID)
- Ed25519 keys: 32 bytes (pub/priv), 64 bytes (signature)
- SHA-256 hash → first 16 bytes as hex

### Protocol
- Version 1, UUIDv4 message IDs, Unix ms timestamps
- CBOR for production, JSON for debug
- Convenience: `new_text()`, `new_delete()`

### Transport
- Hyper HTTP/1.1 server/client
- Endpoints: `/output` (legacy), `/ping` (connectivity), `/message` (new)
- Dual handlers: MessageHandler (legacy), NewMessageHandler (AppState)

### Queue
- Priority: Urgent > High > Normal > Low
- Backoff: base_delay * 2^attempts
- `retry_pending_on_startup()` returns (succeeded, failed)
- Auto-remove after max retries

### Storage
- Contact tokens: base64 CBOR (IP, pubkey, expiry)
- Settings: JSON file, auto-create parent dirs
- AppState: JSON/CBOR serialization
- No SQLite yet (placeholder `Storage` struct)

### Messaging
- `send_message()` → auto-queue on fail
- `create_chat_from_ping()` → active/inactive based on response
- `delete_chat()` → smart (active=notify, inactive=local)
- `handle_incoming_message()` → auto-create chat if missing

## Testing

**Structure:**
- All tests extracted to `src/tests/` directory (233 total tests)
- Pattern: `test_<feature>_<scenario>`
- Test both success and failure paths

**Test Files:**
- `crypto_tests.rs` (7 tests) - Keypair generation, signing, UID derivation
- `protocol_tests.rs` (10 tests) - Message envelope serialization, versioning
- `transport_tests.rs` (26 tests) - HTTP endpoints, peer management, delivery
- `storage_tests.rs` (51 tests) - Contact tokens, AppState, Settings persistence
- `queue_tests.rs` (34 tests) - SQLite queue, priority, retry logic
- `messaging_tests.rs` (17 tests) - High-level messaging API
- `connectivity_tests.rs` (15 tests) - Port forwarding protocols
- `tui_tests.rs` (90 tests) - All TUI screens, App state, navigation
- `lib_tests.rs` (1 test) - Library initialization

**Note:** Binary (`src/bin/tui.rs`) has no tests - it's just glue code. All logic is tested in `tui_tests.rs`.

## Dependencies

**Core:** `ed25519-dalek`, `ring`, `serde`, `serde_cbor`, `chrono`, `tokio`, `hyper`, `rusqlite`
**TUI:** `ratatui`, `crossterm`, `tempfile` (tests)

## Commit Style

```
feat(module): description    # New features
fix(module): description     # Bug fixes
chore: description           # Maintenance
```

## Additional Docs

- **[README.md](README.md)** - Overview, philosophy, platform support
- **[DEVELOPMENT.md](DEVELOPMENT.md)** - Setup, build, troubleshooting
- **[ROADMAP.md](ROADMAP.md)** - Version timeline, planned features
