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

**`crypto`** - Ed25519 keypairs (signing/verification), X25519 keypairs (key exchange), SHA-256 UID generation, ECDH shared secret derivation, XChaCha20-Poly1305 AEAD encryption, Ed25519 token signing

**`protocol`** - CBOR/JSON message envelopes with UUID, version, timestamps, message types (Text, Delete), E2E encryption support (encrypted flag + ciphertext)

**`transport`** - HTTP/1.1 server with `/output`, `/ping`, `/message` endpoints. Peer management, delivery tracking.

**`storage`** - Modular storage system for persistent data:
- `contact.rs` - Contact struct and signed token generation/verification (base64 CBOR + Ed25519 signature)
- `message.rs` - Message struct and delivery status tracking
- `chat.rs` - Chat conversation management
- `settings.rs` - Application settings with auto-save (JSON)
- `settings_manager.rs` - Thread-safe settings access for UI
- `app_state.rs` - AppState persistence (JSON/CBOR)
- `storage_db.rs` - Low-level SQLite storage (unimplemented)
- `mod.rs` - Public API with re-exports

**`queue`** - SQLite-backed retry queue, priority ordering, exponential backoff, startup retry

**`messaging`** - High-level API combining transport/queue/storage. Send with auto-queue, chat lifecycle, smart deletion

**`connectivity`** - Modular NAT traversal system with IPv6 → PCP → NAT-PMP → UPnP orchestration:
- `types.rs` - Common types (PortMappingResult, MappingProtocol, MappingError, ConnectivityResult, StrategyAttempt, IpProtocol)
- `gateway.rs` - Cross-platform gateway discovery (Linux, macOS, Windows)
- `pcp.rs` - PCP (Port Control Protocol, RFC 6887) implementation
- `natpmp.rs` - NAT-PMP (RFC 6886) implementation
- `upnp.rs` - UPnP IGD implementation
- `ipv6.rs` - IPv6 direct connectivity detection
- `cgnat.rs` - CGNAT detection (RFC 6598, 100.64.0.0/10 range), private IP helpers
- `orchestrator.rs` - Main `establish_connectivity()` function with automatic fallback
- `manager.rs` - PortMappingManager (PCP auto-renewal), UpnpMappingManager (cleanup)

**`tui`** - Terminal UI module (library, not binary). Reusable across platforms:
- `types.rs` - Screen and MenuItem enums
- `screens.rs` - All screen state structs (ShareContact, ImportContact, ChatList, ChatView, Settings, Diagnostics, StartupSync)
- `app.rs` - Main App struct with business logic, automatic background connectivity on startup
- `ui.rs` - Rendering functions (ratatui-based)

## Data Structures

**Contact** - `uid`, `ip`, `pubkey`, `x25519_pubkey`, `expiry`, `is_active`. Methods: `is_expired()`, `activate()`, `deactivate()`

**Chat** - `contact_uid`, `messages[]`, `is_active`, `has_pending_messages`. Methods: `append_message()`, `mark_unread()`, `mark_has_pending()`

**AppState** - `contacts[]`, `chats[]`, `message_queue[]`, `settings`. Methods: `get_chat()`, `sync_pending_status()`, `save()`/`load()`

**Settings** - Retry intervals, storage path, contact expiry, max retries. Auto-save to JSON. Thread-safe SettingsManager for UI.

**App (TUI)** - Main application state with automatic connectivity:
- `connectivity_result` - Stores startup/latest connectivity test results
- `local_ip` - Automatically updated from connectivity results (external IP:port)
- `local_port` - Port for listening and connectivity tests (default: 8080)
- `diagnostics_refresh_handle` - Background thread handle for async connectivity tests
- Startup: Automatically runs `establish_connectivity()` in background on launch
- ShareContact: Uses detected external IP for accurate contact tokens

## TUI Architecture

**Binary (`src/bin/tui.rs`)** - Thin wrapper (~320 lines):
- `main()` - Terminal initialization/cleanup, triggers startup connectivity
- `run_app()` - Event loop with 100ms polling
- Polls for startup connectivity completion (updates `local_ip` when ready)
- Polls for diagnostics refresh completion (when on Diagnostics screen)
- Keyboard mapping to App methods

**Library (`src/tui/`)** - Reusable UI logic:
- Used by TUI binary, future mobile/desktop UIs
- Fully tested (294 unit tests)
- Platform-agnostic business logic
- Modular UI rendering (`ui/` directory with per-screen modules)
- Background async connectivity via spawned threads with tokio runtime

**UI Module Structure (`src/tui/ui/`):**
- `mod.rs` - Main `ui()` dispatcher and re-exports
- `startup_sync.rs` - Startup sync progress screen
- `main_menu.rs` - Main menu with hotkey navigation (c/s/i/n)
- `share_contact.rs` - Contact token generation screen (uses auto-detected external IP)
- `import_contact.rs` - Contact token import screen
- `chat_list.rs` - Chat list with delete confirmation popup
- `chat_view.rs` - Individual chat conversation view
- `settings.rs` - Settings configuration screen
- `diagnostics.rs` - Network diagnostics with manual refresh (IPv4/IPv6, external endpoint, mapping lifecycle, RTT, queue size, CGNAT detection)
- `helpers.rs` - Shared UI utilities (`format_duration_until`)

**Screens:**
1. **StartupSync** - Progress bar for pending queue (✓/✗ counters, elapsed time), automatic on startup if messages pending
2. **MainMenu** - Navigate features (↑↓/j/k, Enter), quick access hotkeys (c/s/i/n)
3. **ShareContact** - Generate tokens (copy/save), shows UID/IP (auto-detected external IP), expiry countdown
4. **ImportContact** - Parse/validate tokens, expiry check, signature verification
5. **ChatList** - Status badges (⚠ Expired | ⌛ Pending | ● New | ○ Read), delete with confirmation
6. **ChatView** - Message history (scroll ↑↓), send with Enter, E2E encrypted messages
7. **Settings** - Edit retry interval (1-1440 min), auto-save with toast
8. **Diagnostics** - Two-column layout: Protocol status (PCP/NAT-PMP/UPnP) + System info (IPv4/IPv6, external endpoint, mapping lifetime & renewal countdown, ping RTT, queue size), CGNAT detection, manual refresh (r/F5) triggers background async tests

**Keyboard:**
- Global: q/Esc=back/quit, ↑↓/j/k=nav, Enter=select, d/Del=delete, Backspace/Delete for input
- Main menu: c=chats, s=share, i=import, n=diagnostics
- Diagnostics: r/F5=refresh

**Colors:** Cyan=titles, Green=success/active, Yellow=warning/pending, Red=error/expired, Gray=inactive

## Implementation Notes

### Crypto
- **Dual keypairs**: Ed25519 (signing) + X25519 (key exchange), both generated from random bytes
- **UIDs**: Deterministic SHA-256(Ed25519_pubkey) → first 16 bytes as hex
- **Ed25519**: 32 bytes (pub/priv), 64 bytes (signature). Used for message authentication and token signing
- **X25519**: 32 bytes (pub/secret), used for ECDH key exchange
- **Key derivation**: Public key = `x25519(secret, basepoint)` with proper clamping
- **Shared secrets**: `derive_shared_secret(my_x25519_secret, their_x25519_public)` → 32-byte symmetric key
- **AEAD Encryption**: XChaCha20-Poly1305 with 24-byte nonces, 16-byte Poly1305 auth tags
- **Token signing**: `sign_contact_token()` creates 64-byte Ed25519 signatures, `verify_contact_token()` verifies integrity
- **EncryptedEnvelope**: `{nonce: [u8; 24], ciphertext: Vec<u8>}` with embedded auth tag

### Protocol
- Version 1, UUIDv4 message IDs, Unix ms timestamps
- CBOR for production, JSON for debug
- **Encryption support**: `encrypted: bool` flag, payload contains `EncryptedEnvelope` (CBOR) when true
- Convenience: `new_text()`, `new_delete()` (plaintext), `new_text_encrypted()`, `new_delete_encrypted()`
- Methods: `decrypt_payload(secret)` for decryption, `get_payload(optional_secret)` for transparent access

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

**Module Architecture** (8 files, ~150-400 lines each):
- `contact.rs` - Contact struct with token generation/parsing
- `message.rs` - Message struct with delivery status (Sent, Delivered, Pending, Failed)
- `chat.rs` - Chat conversation management
- `settings.rs` - Settings struct with JSON persistence
- `settings_manager.rs` - Thread-safe SettingsManager (Arc<RwLock<Settings>>)
- `app_state.rs` - AppState struct with save/load (JSON/CBOR)
- `storage_db.rs` - Low-level SQLite Storage (unimplemented)
- `mod.rs` - Public API with re-exports

**Contact Tokens**:
- Signed with Ed25519, base64 CBOR format: `{payload: {ip, pubkey, x25519_pubkey, expiry}, signature: [u8; 64]}`
- Signature verified on import, rejects tampered/forged tokens
- Contact struct stores both pubkeys for dual-purpose: identity (Ed25519) and encryption (X25519)

**Settings & AppState**:
- Settings: JSON file, auto-create parent dirs, thread-safe access via SettingsManager
- AppState: JSON/CBOR serialization, sync_pending_status() updates chat flags

### Messaging
- `send_message()` → auto-queue on fail
- `create_chat_from_ping()` → active/inactive based on response
- `delete_chat()` → smart (active=notify, inactive=local)
- `handle_incoming_message()` → auto-create chat if missing

### Connectivity

**Module Architecture** (10 files, ~150-400 lines each):
- `types.rs` - Shared types: PortMappingResult, MappingError, ConnectivityResult (with cgnat_detected field), StrategyAttempt, IpProtocol
- `gateway.rs` - Cross-platform gateway discovery (Linux/macOS/Windows)
- `pcp.rs` - PCP implementation with PcpOpcode, PcpResultCode enums
- `natpmp.rs` - NAT-PMP implementation with NatPmpOpcode, NatPmpResultCode enums
- `upnp.rs` - UPnP IGD with blocking operations
- `ipv6.rs` - IPv6 detection helpers (check_ipv6_connectivity, is_ipv6_link_local)
- `cgnat.rs` - CGNAT detection: detect_cgnat(ip) checks 100.64.0.0/10 range, is_private_ip(ip) helper
- `orchestrator.rs` - Main `establish_connectivity()` function
- `manager.rs` - PortMappingManager (PCP), UpnpMappingManager (UPnP)
- `mod.rs` - Public API with re-exports

**Orchestrator Behavior**:
- `establish_connectivity(port)` tries IPv6 → PCP → NAT-PMP → UPnP sequentially
- Returns `ConnectivityResult` with full tracking of all attempts + CGNAT detection
- Each protocol gets `StrategyAttempt`: NotAttempted | Success(mapping) | Failed(error)
- Stops on first success, continues through all on failure
- `result.summary()` generates UX string: "⚠️ CGNAT → IPv6: no → PCP: ok" (if CGNAT detected)
- CGNAT detection runs automatically after each successful mapping
- **Automatic on startup**: TUI triggers connectivity test in background thread on app launch
- **Manual refresh**: Diagnostics screen 'r'/F5 keys trigger new background test
- Results stored in `App.connectivity_result`, external IP auto-updates `App.local_ip`

**Protocol Details**:
- **PCP** (RFC 6887): 60-byte MAP requests, up to 1100-byte responses, UDP port 5351
- **NAT-PMP** (RFC 6886): 12-byte requests, 16-byte responses, requires separate external IP request
- **UPnP**: SSDP discovery + SOAP, blocking I/O spawned to tokio::task::spawn_blocking
- **IPv6**: Binds to `[::]`, connects to public IPv6 (2001:4860:4860::8888) to verify global address
- **CGNAT** (RFC 6598): Detects 100.64.0.0/10 range, warns user that relay is required for P2P

**Lifecycle Management**:
- `PortMappingManager`: Auto-renews PCP mappings at 80% of lifetime (e.g., 48 min for 1 hour)
- `UpnpMappingManager`: Auto-cleanup on Drop (best-effort thread spawn)
- Gateway discovery: Platform-specific (Linux: /proc/net/route, macOS: netstat, Windows: route print)

## Testing

**Structure:**
- All tests in `src/tests/` directory (294 total tests
- Pattern: `test_<feature>_<scenario>`
- Test both success and failure paths
- Organized in subdirectories mirroring module structure

**Test Organization:**
- `crypto_tests.rs` (27 tests) - Keypair generation, signing, UID derivation, X25519 shared secret, AEAD encryption (roundtrip, tampering), token signing (valid, invalid, corrupted)
- `protocol_tests.rs` (25 tests) - Message envelope serialization, versioning, E2E encryption (roundtrip, wrong key, CBOR/JSON, plaintext vs encrypted)
- `transport_tests.rs` (26 tests) - HTTP endpoints, peer management, delivery
- `queue_tests.rs` (34 tests) - SQLite queue, priority, retry logic
- `messaging_tests.rs` (17 tests) - High-level messaging API
- `connectivity_tests.rs` (30 tests) - PCP, NAT-PMP, UPnP, orchestrator, IPv6, CGNAT
- `lib_tests.rs` (1 test) - Library initialization

**`storage_tests/` (56 tests):**
- `contact_tests.rs` (11 tests) - Contact struct (creation, expiry, activation, serialization)
- `token_tests.rs` (16 tests) - Signed token generation/parsing (roundtrip, validation, signature verification, tampering detection, wrong signer)
- `chat_tests.rs` (9 tests) - Chat/Message structs (append, active management, pending flags)
- `app_state_tests.rs` (11 tests) - AppState (save/load, sync, chat management)
- `settings_tests.rs` (16 tests) - Settings/SettingsManager (defaults, persistence, concurrency)

**`tui_tests/` (118 tests):**
- `app_tests.rs` (32 tests) - App struct and business logic
- `screen_tests/` (75 tests) - All screens, modularized by screen type (consent screen removed):
  - `share_contact_tests.rs` (5 tests) - ShareContactScreen (token generation, file save)
  - `import_contact_tests.rs` (10 tests) - ImportContactScreen (parsing, validation)
  - `chat_list_tests.rs` (5 tests) - ChatListScreen (navigation, delete popup)
  - `chat_view_tests.rs` (3 tests) - ChatViewScreen (input, scrolling)
  - `settings_tests.rs` (9 tests) - SettingsScreen (validation, persistence)
  - `startup_sync_tests.rs` (10 tests) - StartupSyncScreen (progress tracking)
  - `diagnostics_tests.rs` (20 tests) - DiagnosticsScreen (IPv4/IPv6, external endpoint, lifetime/renewal, RTT, queue size, CGNAT)
  - `status_indicators_tests.rs` (10 tests) - Status badges and contact expiry
  - `mod.rs` - Module organization
- `types_tests.rs` (3 tests) - MenuItem enum
- `ui_tests.rs` (4 tests) - UI helper functions (format_duration_until)

**Note:** Binary (`src/bin/tui.rs`) has no tests - it's glue code. All logic tested in `tui_tests/`. UI rendering functions in `src/tui/ui/` are modular (9 files - consent screen removed) for maintainability. Screen tests are modularized in `screen_tests/` subdirectory for easier navigation and maintenance.

## Dependencies

**Core:** `ed25519-dalek`, `x25519-dalek`, `chacha20poly1305`, `ring`, `serde`, `serde_cbor`, `chrono`, `tokio`, `hyper`, `rusqlite`
**TUI:** `ratatui`, `crossterm`, `arboard` (clipboard), `tempfile` (tests)

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
