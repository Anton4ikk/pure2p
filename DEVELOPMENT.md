# Development Guide

Setup instructions for Pure2P development.

> **Quick Links**: [README](README.md) • [Roadmap](ROADMAP.md) • [Claude Docs](CLAUDE.md)

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
├── storage.rs          # Contacts, chats, AppState
├── queue.rs            # SQLite retry queue
├── messaging.rs        # High-level API
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
│   ├── app.rs          # App business logic
│   └── ui/             # Modular rendering (10 files)
│       ├── mod.rs      # Main ui() dispatcher
│       ├── startup_sync.rs
│       ├── main_menu.rs
│       ├── share_contact.rs
│       ├── import_contact.rs
│       ├── chat_list.rs
│       ├── chat_view.rs
│       ├── settings.rs
│       ├── diagnostics.rs
│       └── helpers.rs
├── tests/              # Unit tests (301 tests)
│   ├── mod.rs
│   ├── crypto_tests.rs
│   ├── protocol_tests.rs
│   ├── transport_tests.rs
│   ├── queue_tests.rs
│   ├── messaging_tests.rs
│   ├── connectivity_tests.rs  # Includes CGNAT detection
│   ├── lib_tests.rs
│   ├── storage_tests/  # Storage module tests (51 tests)
│   │   ├── mod.rs
│   │   ├── contact_tests.rs
│   │   ├── token_tests.rs
│   │   ├── chat_tests.rs
│   │   ├── app_state_tests.rs
│   │   └── settings_tests.rs
│   └── tui_tests/      # TUI module tests (121 tests)
│       ├── mod.rs
│       ├── app_tests.rs
│       ├── screens_tests.rs  # Includes enhanced diagnostics tests
│       ├── types_tests.rs
│       └── ui_tests.rs
└── bin/
    └── tui.rs          # TUI binary (thin wrapper)
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
cargo test                     # All tests (301 total)

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
├── crypto_tests.rs       (11 tests)  - Keypair, signing, UID, X25519 ECDH
├── protocol_tests.rs     (10 tests)  - Envelopes, serialization
├── transport_tests.rs    (26 tests)  - HTTP, peers, delivery
├── queue_tests.rs        (34 tests)  - SQLite queue, retries
├── messaging_tests.rs    (17 tests)  - High-level messaging API
├── connectivity_tests.rs (30 tests)  - PCP, NAT-PMP, UPnP, IPv6, CGNAT detection
├── lib_tests.rs          (1 test)    - Library init
├── storage_tests/        (51 tests)  - Organized by functionality
│   ├── contact_tests.rs  (11 tests)  - Contact struct, expiry, activation
│   ├── token_tests.rs    (8 tests)   - Token generation/parsing, validation
│   ├── chat_tests.rs     (9 tests)   - Chat/Message structs, pending flags
│   ├── app_state_tests.rs (11 tests) - AppState save/load, sync
│   └── settings_tests.rs (22 tests)  - Settings, SettingsManager, concurrency
└── tui_tests/            (121 tests) - Organized by TUI components
    ├── app_tests.rs      (36 tests)  - App business logic
    ├── screens_tests.rs  (82 tests)  - All screens + enhanced Diagnostics (IPv4/IPv6, external endpoint, RTT, queue size)
    ├── types_tests.rs    (3 tests)   - MenuItem enum
    └── ui_tests.rs       (4 tests)   - UI helper functions (format_duration_until)
```

**Run specific tests:**
```bash
cargo test --lib                  # All library tests
cargo test crypto_tests           # Crypto tests
cargo test storage_tests          # All storage tests
cargo test tui_tests              # All TUI tests
cargo test storage_tests::contact # Just contact tests
cargo test tui_tests::app         # Just app tests
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

---

## Contributing

1. Open issue to discuss
2. Fork and create feature branch
3. Make changes with tests
4. Run `cargo fmt && cargo clippy && cargo test`
5. Submit PR with clear description

**Must maintain:**
- Direct P2P only (no servers/relays)
- Local-only storage
- Transparency about limitations

See [ROADMAP.md](ROADMAP.md#-contributing) for details.

---

## Related Docs

- **[README.md](README.md)** — Overview and quick start
- **[ROADMAP.md](ROADMAP.md)** — Version timeline
- **[CLAUDE.md](CLAUDE.md)** — Implementation details
