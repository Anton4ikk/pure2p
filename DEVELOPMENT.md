# Development Guide

Setup instructions for Pure2P development.

> **Quick Links**: [README](README.md) • [Usage Guide](USAGE.md) • [Roadmap](ROADMAP.md)
---

## Quick Start

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/Anton4ikk/pure2p.git
cd pure2p
cargo build --release

# Test and run
cargo test
cargo run --bin pure2p-tui

# Clean build folder
cargo clean
```

---

## Platform Requirements

**macOS:** `xcode-select --install`

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get install build-essential pkg-config libssl-dev
```

**Windows:** [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with C++ tools

---

## Project Structure

```
src/
├── lib.rs              # Entry point, error types
├── crypto.rs           # Ed25519 (signing), X25519 (key exchange), UIDs, ECDH
├── protocol.rs         # CBOR/JSON envelopes
├── transport.rs        # HTTP server/client
├── queue.rs            # SQLite retry queue
├── messaging.rs        # High-level API
├── storage/            # SQLite-based persistent storage
│   ├── mod.rs          # Public API, re-exports
│   ├── contact.rs      # Contact struct, signed token generation/verification
│   ├── message.rs      # Message struct, delivery status tracking
│   ├── chat.rs         # Chat conversation management
│   ├── settings.rs     # Settings struct
│   ├── settings_manager.rs # Thread-safe SettingsManager (legacy, unused in TUI)
│   ├── app_state.rs    # AppState with SQLite persistence methods
│   └── storage_db.rs   # SQLite storage backend (schema + CRUD)
├── connectivity/       # NAT traversal (modular)
│   ├── mod.rs          # Public API, re-exports
│   ├── types.rs        # Common types (PortMappingResult, MappingError, etc.)
│   ├── gateway.rs      # Cross-platform gateway discovery
│   ├── pcp.rs          # PCP (Port Control Protocol, RFC 6887)
│   ├── natpmp.rs       # NAT-PMP (RFC 6886)
│   ├── upnp.rs         # UPnP IGD implementation
│   ├── ipv6.rs         # IPv6 direct connectivity detection
│   ├── cgnat.rs        # CGNAT detection (RFC 6598, 100.64.0.0/10)
│   ├── orchestrator.rs # establish_connectivity() - IPv6→PCP→NAT-PMP→UPnP
│   └── manager.rs      # PortMappingManager, UpnpMappingManager
├── tui/                # TUI module (library)
│   ├── mod.rs          # Module exports
│   ├── types.rs        # Screen, MenuItem enums
│   ├── screens.rs      # Screen state structs
│   ├── app.rs          # App business logic with automatic background connectivity and retry worker
│   └── ui/             # Modular rendering (8 files - StartupSync screen removed)
│       ├── mod.rs      # Main ui() dispatcher
│       ├── main_menu.rs
│       ├── share_contact.rs
│       ├── import_contact.rs
│       ├── chat_list.rs
│       ├── chat_view.rs
│       ├── settings.rs
│       ├── diagnostics.rs
│       └── helpers.rs
├── tests/              # Unit tests (379 tests)
│   ├── mod.rs
│   ├── crypto_tests.rs
│   ├── protocol_tests.rs
│   ├── transport_tests.rs
│   ├── queue_tests.rs
│   ├── messaging_tests.rs
│   ├── connectivity_tests.rs  # Includes CGNAT detection
│   ├── lib_tests.rs
│   ├── storage_tests/  # Storage module tests (66 tests)
│   │   ├── mod.rs
│   │   ├── contact_tests.rs
│   │   ├── token_tests.rs
│   │   ├── chat_tests.rs
│   │   ├── app_state_tests.rs
│   │   └── settings_tests.rs
│   └── tui_tests/      # TUI module tests (120 tests)
│       ├── mod.rs
│       ├── app_tests/        # Modularized app tests (42 tests)
│       │   ├── mod.rs
│       │   ├── helpers.rs
│       │   ├── initialization_tests.rs
│       │   ├── navigation_tests.rs
│       │   ├── contact_import_tests.rs
│       │   ├── chat_management_tests.rs
│       │   ├── messaging_tests.rs
│       │   └── startup_tests.rs
│       ├── screen_tests/     # Modularized screen tests (76 tests)
│       │   ├── mod.rs
│       │   ├── share_contact_tests.rs
│       │   ├── import_contact_tests.rs
│       │   ├── chat_list_tests.rs
│       │   ├── chat_view_tests.rs
│       │   ├── settings_tests.rs
│       │   ├── diagnostics_tests.rs
│       │   └── status_indicators_tests.rs
│       ├── types_tests.rs
│       └── ui_tests.rs
└── bin/
    └── tui.rs          # TUI binary (thin wrapper, starts transport server, triggers connectivity, retry worker starts after connectivity)
```

See [CLAUDE.md](CLAUDE.md#core-modules) for implementation details.

---

## Common Commands

```bash
# Build
cargo build                    # Debug
cargo build --release          # Optimized
cargo check                    # Fast compile check

# Test
cargo test                     # All tests (381 total)

# Quality
cargo fmt                      # Format
cargo clippy                   # Lint
cargo clippy -- -D warnings    # Fail on warnings

# Run
cargo run --bin pure2p-tui     # TUI app

# Docs
cargo doc --open               # Generate docs
```

---

## Development Workflow

```bash
# 1. Create branch
git checkout -b feature/name

# 2. Make changes
cargo fmt && cargo clippy && cargo test

# 3. Commit
git commit -m "feat(module): description"

# 4. Push
git push origin feature/name
```

**Commit Prefixes:** `feat`, `fix`, `chore`, `docs`, `test`

---

## Testing Architecture

**All tests are in `src/tests/`** (not inline in modules):

```
src/tests/
├── crypto_tests.rs       (27 tests)  - Keypair, signing, UID, X25519 ECDH, AEAD encryption, token signing
├── protocol_tests.rs     (25 tests)  - Envelopes, serialization, E2E encryption
├── transport_tests.rs    (26 tests)  - HTTP, peers, delivery
├── queue_tests.rs        (34 tests)  - SQLite queue, retries
├── messaging_tests.rs    (17 tests)  - High-level messaging API
├── connectivity_tests.rs (30 tests)  - PCP, NAT-PMP, UPnP, IPv6, CGNAT detection
├── lib_tests.rs          (1 test)    - Library init
├── storage_tests/        (66 tests)  - Organized by storage module
│   ├── contact_tests.rs  (11 tests)  - Contact struct, expiry, activation
│   ├── token_tests.rs    (16 tests)  - Token generation/parsing, signature verification
│   ├── chat_tests.rs     (9 tests)   - Chat/Message structs, pending flags
│   ├── app_state_tests.rs (21 tests) - AppState JSON/CBOR + SQLite (save/load, messages, updates, migration)
│   └── settings_tests.rs (16 tests)  - Settings, SettingsManager, concurrency
└── tui_tests/            (120 tests) - Organized by TUI components
    ├── app_tests/        (42 tests)  - Modularized by feature area
    │   ├── helpers.rs                - Shared test utilities
    │   ├── initialization_tests.rs   (6 tests)   - App creation, state loading, settings
    │   ├── navigation_tests.rs       (14 tests)  - Screen transitions, menu navigation
    │   ├── contact_import_tests.rs   (3 tests)   - Import validation, duplicate detection, self-import prevention
    │   ├── chat_management_tests.rs  (14 tests)  - Chat creation, deletion, selection
    │   ├── messaging_tests.rs        (3 tests)   - Message sending
    │   └── startup_tests.rs          (2 tests)   - Startup screen, connectivity
    ├── screen_tests/     (76 tests)  - Modularized by screen type
    │   ├── share_contact_tests.rs    (5 tests)   - ShareContactScreen
    │   ├── import_contact_tests.rs   (10 tests)  - ImportContactScreen
    │   ├── chat_list_tests.rs        (5 tests)   - ChatListScreen
    │   ├── chat_view_tests.rs        (3 tests)   - ChatViewScreen
    │   ├── settings_tests.rs         (10 tests)  - SettingsScreen
    │   ├── diagnostics_tests.rs      (20 tests)  - DiagnosticsScreen (IPv4/IPv6, external endpoint, RTT, queue size, CGNAT)
    │   └── status_indicators_tests.rs (10 tests) - Status badges and contact expiry
    ├── types_tests.rs    (3 tests)   - MenuItem enum
    └── ui_tests.rs       (4 tests)   - UI helper functions (format_duration_until)
```

**Run specific tests:**
```bash
cargo test --lib                        # All library tests
cargo test crypto_tests                 # Crypto tests
```

---

## Cross-Platform Builds

```bash
# Static/dynamic libraries
cargo build --lib

# Cross-compilation
cargo install cross
cross build --target aarch64-linux-android   # Android
cargo build --target aarch64-apple-ios       # iOS (macOS only)
```

---

## Debugging

```bash
# Logging
RUST_LOG=debug cargo test
RUST_LOG=trace cargo run --bin pure2p-tui

# Backtraces
RUST_BACKTRACE=1 cargo test
RUST_BACKTRACE=full cargo test

# Security audit
cargo install cargo-audit
cargo audit
```

---

## Troubleshooting

**macOS "xcrun: error":**
```bash
xcode-select --install
sudo xcode-select --reset
```

**Linux "openssl not found":**
```bash
sudo apt-get install pkg-config libssl-dev
```

**Windows build errors:**
- Install Visual Studio Build Tools
- Enable "Desktop development with C++"
- Restart terminal

**Tests hang:**
- Check SQLite locks: `rm -rf target/` and rebuild
- Use `-- --test-threads=1` to run sequentially

**Contact token invalid after restart:**
- App now intelligently reuses the same port when on the same network
- Port only changes when your external IP changes (different network)
- Check diagnostics screen (press `n`) to verify external IP:port
- If you need to force a new port, delete `./app_data/pure2p.db`

**Chat stuck in ⌛ Pending status:**
- Chat becomes Active (●) only when ping response is received
- If contact is offline/unreachable, chat stays Pending until they come online
- Retry worker automatically keeps trying to ping them
- Chat will auto-transition to Active when ping finally succeeds
- Check queue size in diagnostics (press `n`) to see pending messages

---

## Contributing

1. Open issue to discuss
2. Fork and create feature branch
3. Make changes with tests
4. Run `cargo fmt && cargo clippy && cargo test`
5. Submit PR with clear description

**Must maintain:**
- Direct P2P only (no servers/relays)
- Local-only storage (SQLite databases in `./app_data/`)
- Transparency about limitations

**Storage Architecture:**
- **Production**: `./app_data/pure2p.db` (all app data) + `./app_data/message_queue.db` (retry queue)
- **Tests**: In-memory SQLite databases (no filesystem pollution)
- **Migration**: Legacy `app_state.json` auto-migrated to SQLite on first run (backed up as `.json.bak`)
- **Data**: User identity (keypair, UID), contacts, chats, messages, settings, network info (IP, port)
- **Auto-save**: State saved to SQLite after every modification
- **Concurrent access**: Transport handlers create separate connections to same database file
- **State reload**: App reloads from DB when navigating to pick up incoming messages
- **Port persistence**: Smart port selection reuses saved port when IP unchanged (maintains contact token validity across restarts)
- **Chat status**: Chats marked as Active only when ping response received (confirms two-way connectivity)
- Safe to delete `./app_data/` for full reset (will recreate with defaults)

See [ROADMAP.md](ROADMAP.md#-contributing) for details.

---

## Related Docs

- **[README.md](README.md)** — Overview and quick start
- **[USAGE.md](USAGE.md)** — User guide and troubleshooting
- **[ROADMAP.md](ROADMAP.md)** — Version timeline
