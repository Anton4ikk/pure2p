# Development Guide

Setup instructions for Pure2P development.

> **Quick Links**: [README](README.md) • [Roadmap](ROADMAP.md) • [Claude Docs](CLAUDE.md)

---

## Quick Start

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/yourusername/pure2p.git
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
├── lib.rs         # Entry point, error types
├── crypto.rs      # Ed25519, UIDs
├── protocol.rs    # CBOR/JSON envelopes
├── transport.rs   # HTTP server/client
├── storage.rs     # Contacts, chats, AppState
├── queue.rs       # SQLite retry queue
├── messaging.rs   # High-level API
└── bin/tui.rs     # Terminal UI
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
cargo test                     # All tests
cargo test crypto::            # Specific module
cargo test -- --nocapture      # Show output

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
