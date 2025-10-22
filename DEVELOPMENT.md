# Development Guide

This guide provides instructions for setting up the Pure2P development environment and running tests.

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
```

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Tests for Specific Module

```bash
# Crypto module tests
cargo test crypto::

# Storage module tests
cargo test storage::

# Transport module tests
cargo test transport::

# Queue module tests
cargo test queue::
```

### Run a Specific Test

```bash
# Run a single test by name
cargo test test_keypair_generation

# Run tests matching a pattern
cargo test uid
```

### Run Tests with Output

```bash
# Show println! output for passing tests
cargo test -- --nocapture

# Show output only for failed tests (default)
cargo test
```

### Run Tests in Verbose Mode

```bash
# Show detailed information about test execution
cargo test -- --test-threads=1 --nocapture
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
│   ├── lib.rs          # Library entry point
│   ├── crypto.rs       # Cryptographic operations (Ed25519, UIDs)
│   ├── transport.rs    # Network transport layer
│   ├── storage.rs      # Local data storage
│   └── queue.rs        # Message queue management
├── Cargo.toml          # Project dependencies and metadata
├── DEVELOPMENT.md      # This file
└── README.md           # Project overview
```

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
```

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

- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)
- [Pure2P Project Documentation](https://github.com/yourusername/pure2p)
- [Ed25519 Dalek Docs](https://docs.rs/ed25519-dalek/)
- [Ring Cryptography Docs](https://docs.rs/ring/)

## Getting Help

- Open an issue on GitHub
- Check existing issues and discussions
- Review the project documentation

## Contributing

Please read our contributing guidelines before submitting pull requests. Ensure:

1. All tests pass
2. Code is formatted (`cargo fmt`)
3. No clippy warnings (`cargo clippy`)
4. New features include tests
5. Commit messages follow conventional commits format
