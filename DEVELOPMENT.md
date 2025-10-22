# Development Guide

This guide provides instructions for setting up the Pure2P development environment and running tests.

> **Quick Links**: [README](README.md) • [Quick Start](QUICKSTART.md) • [Roadmap](ROADMAP.md) • [Claude Docs](CLAUDE.md)

## Prerequisites

### Required Tools

- **Rust** (1.70 or later)
  - Install via [rustup](https://rustup.rs/):
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
  - Verify installation:
    ```bash
    rustc --version
    cargo --version
    ```

### Platform-Specific Requirements

#### macOS
```bash
# Install Xcode Command Line Tools
xcode-select --install
```

#### Linux
```bash
# Debian/Ubuntu
sudo apt-get update
sudo apt-get install build-essential pkg-config libssl-dev

# Fedora/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel
```

#### Windows
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
- Ensure C++ build tools are selected during installation

## Project Setup

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/pure2p.git
cd pure2p
```

### 2. Build the Project

```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Release build (optimized)
cargo build --release
```

### 3. Verify Setup

```bash
# Check that the project compiles
cargo check

# Run all tests
cargo test

# Try the CLI
cargo run --bin pure2p-cli
```

See [QUICKSTART.md](QUICKSTART.md) for your first P2P message tutorial.

## Running Tests

### Run All Tests

```bash
cargo test
```

## Code Quality

### Format Code

```bash
# Check formatting
cargo fmt -- --check

# Auto-format code
cargo fmt
```

### Lint with Clippy

```bash
# Run clippy linter
cargo clippy

# Run clippy with all warnings
cargo clippy -- -W clippy::all

# Fix auto-fixable clippy warnings
cargo clippy --fix
```

### Check for Security Vulnerabilities

```bash
# Install cargo-audit
cargo install cargo-audit

# Run security audit
cargo audit
```

## Building for Different Targets

### Library Types

Pure2P supports multiple library types for cross-platform compatibility:

```bash
# Build static library
cargo build --lib

# The output includes:
# - libpure2p.a (static library)
# - libpure2p.dylib/.so/.dll (dynamic library)
# - libpure2p.cdylib (C-compatible dynamic library)
```

### Cross-Platform Builds

```bash
# Install cross-compilation tool
cargo install cross

# Build for Android (example)
cross build --target aarch64-linux-android

# Build for iOS (macOS only)
cargo build --target aarch64-apple-ios
```

## Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/your-feature-name
```

### 2. Make Changes and Test

```bash
# Make your changes...

# Format code
cargo fmt

# Check for issues
cargo clippy

# Run tests
cargo test
```

### 3. Commit Changes

```bash
git add .
git commit -m "feat(module): description of changes"
```

### 4. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

## Project Structure

```
pure2p/
├── src/
│   ├── lib.rs          # Library entry point and error types
│   ├── crypto.rs       # Cryptographic operations (Ed25519, UIDs)
│   ├── protocol.rs     # MessageEnvelope, CBOR/JSON serialization
│   ├── transport.rs    # HTTP/1.1 server/client, POST /output endpoint
│   ├── storage.rs      # Local data storage (stub for v0.2)
│   ├── queue.rs        # SQLite message queue with exponential backoff
│   └── bin/
│       └── cli.rs      # Command-line client (netcat-style)
├── Cargo.toml          # Project dependencies and metadata
├── README.md           # User documentation and project overview
├── QUICKSTART.md       # Get started with CLI in 5 minutes
├── DEVELOPMENT.md      # This file - dev setup and workflow
├── ROADMAP.md          # Version timeline and planned features
└── CLAUDE.md           # Architecture details for AI assistants
```

> **Implementation details**: See [CLAUDE.md](CLAUDE.md#core-modules) for detailed module documentation.

## Common Commands Reference

| Command | Description |
|---------|-------------|
| `cargo build` | Build the project (debug mode) |
| `cargo build --release` | Build with optimizations |
| `cargo test` | Run all tests |
| `cargo test <name>` | Run specific test or module |
| `cargo check` | Check code without building |
| `cargo fmt` | Format code |
| `cargo clippy` | Run linter |
| `cargo clean` | Remove build artifacts |
| `cargo doc --open` | Generate and open documentation |

## Debugging

### Enable Logging

```rust
// In your code or tests
pure2p::init(); // Initializes tracing subscriber
```

```bash
# Set log level via environment variable
RUST_LOG=debug cargo test
RUST_LOG=trace cargo test

# Run CLI with logging
RUST_LOG=debug cargo run --bin pure2p-cli
```

See [CLAUDE.md](CLAUDE.md#build--test-commands) for more test options.

### Run Tests with Backtrace

```bash
# Show backtrace on panic
RUST_BACKTRACE=1 cargo test

# Show full backtrace
RUST_BACKTRACE=full cargo test
```

## Continuous Integration

Tests are automatically run on:
- Pull requests
- Commits to main branch
- Tag creation

Ensure all tests pass locally before pushing:

```bash
# Full CI check
cargo fmt -- --check && \
cargo clippy -- -D warnings && \
cargo test && \
cargo build --release
```

## Troubleshooting

### Build Failures

```bash
# Clean and rebuild
cargo clean
cargo build
```

### Dependency Issues

```bash
# Update dependencies
cargo update

# Check for outdated dependencies
cargo install cargo-outdated
cargo outdated
```

### Test Failures

```bash
# Run failing test in isolation with output
cargo test <test_name> -- --nocapture --test-threads=1
```

## Additional Resources

### Project Documentation
- **[README.md](README.md)** — Project overview and philosophy
- **[QUICKSTART.md](QUICKSTART.md)** — CLI usage tutorial
- **[ROADMAP.md](ROADMAP.md)** — Future features (v0.2+: encryption, GUI, mobile)
- **[CLAUDE.md](CLAUDE.md)** — Detailed architecture and implementation notes

### External Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)
- [Ed25519 Dalek Docs](https://docs.rs/ed25519-dalek/)
- [Ring Cryptography Docs](https://docs.rs/ring/)
- [Hyper HTTP Docs](https://docs.rs/hyper/)

## Getting Help

- Open an issue on GitHub
- Check existing issues and discussions
- Review the project documentation

## Contributing

Please read our contributing guidelines before submitting pull requests. Ensure:

1. All tests pass (`cargo test`)
2. Code is formatted (`cargo fmt`)
3. No clippy warnings (`cargo clippy`)
4. New features include tests
5. Commit messages follow [conventional commits](CLAUDE.md#commit-style) format

### Before Contributing

- Review [README.md](README.md#-contributing) for our core principles
- Check [ROADMAP.md](ROADMAP.md) to see planned features
- Discuss significant changes in GitHub issues first

### Feature Ideas

See [ROADMAP.md](ROADMAP.md) for v0.2-v0.5 planned features:
- **v0.2**: Encryption, persistent storage, rich messages
- **v0.3**: Desktop GUI (Tauri)
- **v0.4**: Mobile apps (iOS/Android)
- **v0.5**: NAT traversal
