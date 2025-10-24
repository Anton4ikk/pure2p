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

**`connectivity`** - Modular NAT traversal system with IPv6 → PCP → NAT-PMP → UPnP orchestration:
- `types.rs` - Common types (PortMappingResult, MappingProtocol, MappingError, ConnectivityResult, StrategyAttempt, IpProtocol)
- `gateway.rs` - Cross-platform gateway discovery (Linux, macOS, Windows)
- `pcp.rs` - PCP (Port Control Protocol, RFC 6887) implementation
- `natpmp.rs` - NAT-PMP (RFC 6886) implementation
- `upnp.rs` - UPnP IGD implementation
- `ipv6.rs` - IPv6 direct connectivity detection
- `orchestrator.rs` - Main `establish_connectivity()` function with automatic fallback
- `manager.rs` - PortMappingManager (PCP auto-renewal), UpnpMappingManager (cleanup)

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

### Connectivity

**Module Architecture** (9 files, ~150-400 lines each):
- `types.rs` - Shared types: PortMappingResult, MappingError, ConnectivityResult, StrategyAttempt, IpProtocol
- `gateway.rs` - Cross-platform gateway discovery (Linux/macOS/Windows)
- `pcp.rs` - PCP implementation with PcpOpcode, PcpResultCode enums
- `natpmp.rs` - NAT-PMP implementation with NatPmpOpcode, NatPmpResultCode enums
- `upnp.rs` - UPnP IGD with blocking operations
- `ipv6.rs` - IPv6 detection helpers (check_ipv6_connectivity, is_ipv6_link_local)
- `orchestrator.rs` - Main `establish_connectivity()` function
- `manager.rs` - PortMappingManager (PCP), UpnpMappingManager (UPnP)
- `mod.rs` - Public API with re-exports

**Orchestrator Behavior**:
- `establish_connectivity(port)` tries IPv6 → PCP → NAT-PMP → UPnP sequentially
- Returns `ConnectivityResult` with full tracking of all attempts
- Each protocol gets `StrategyAttempt`: NotAttempted | Success(mapping) | Failed(error)
- Stops on first success, continues through all on failure
- `result.summary()` generates UX string: "IPv6: no → PCP: ok → external 203.0.113.5:60000"

**Protocol Details**:
- **PCP** (RFC 6887): 60-byte MAP requests, up to 1100-byte responses, UDP port 5351
- **NAT-PMP** (RFC 6886): 12-byte requests, 16-byte responses, requires separate external IP request
- **UPnP**: SSDP discovery + SOAP, blocking I/O spawned to tokio::task::spawn_blocking
- **IPv6**: Binds to `[::]`, connects to public IPv6 (2001:4860:4860::8888) to verify global address

**Lifecycle Management**:
- `PortMappingManager`: Auto-renews PCP mappings at 80% of lifetime (e.g., 48 min for 1 hour)
- `UpnpMappingManager`: Auto-cleanup on Drop (best-effort thread spawn)
- Gateway discovery: Platform-specific (Linux: /proc/net/route, macOS: netstat, Windows: route print)

## Testing

**Structure:**
- All tests extracted to `src/tests/` directory (285 total tests)
- Pattern: `test_<feature>_<scenario>`
- Test both success and failure paths

**Test Files:**
- `crypto_tests.rs` (7 tests) - Keypair generation, signing, UID derivation
- `protocol_tests.rs` (10 tests) - Message envelope serialization, versioning
- `transport_tests.rs` (26 tests) - HTTP endpoints, peer management, delivery
- `storage_tests.rs` (51 tests) - Contact tokens, AppState, Settings persistence
- `queue_tests.rs` (34 tests) - SQLite queue, priority, retry logic
- `messaging_tests.rs` (17 tests) - High-level messaging API
- `connectivity_tests.rs` (26 tests) - PCP, NAT-PMP, UPnP protocols, orchestrator, IPv6 detection
- `tui_tests.rs` (113 tests) - All TUI screens, App state, navigation
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
