# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **For Humans**: See [DEVELOPMENT.md](DEVELOPMENT.md) for setup and workflow, or [README.md](README.md) for project overview.

## Project Overview

Pure2P is a **radically honest P2P messenger** with no servers, relays, or intermediaries. Key architectural principles:

- **Direct peer-to-peer only**: Each client exposes a `POST /output` endpoint for receiving messages
- **Manual UID exchange**: UIDs are derived from Ed25519 public key fingerprints and shared manually
- **Local-only storage**: No sync, no cloud - device loss means history loss
- **Online-only delivery**: Messages queue locally until both peers are online simultaneously
- **No push notifications**: Mobile apps can't wake on external requests

This is a Rust library (`lib`, `staticlib`, `cdylib`) for cross-platform use (Android, iOS, Desktop).

**See also:**
- [README.md](README.md) for user-facing documentation and philosophy
- [ROADMAP.md](ROADMAP.md) for planned features (v0.2: encryption/storage, v0.3: desktop, v0.4: mobile)

## Build & Test Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Optimized release build
cargo check                    # Fast compile check without building

# Run CLI
cargo run --bin pure2p-cli     # Run CLI client (default port 8080)
cargo run --bin pure2p-cli -- --port 9000  # Run on custom port

# Test
cargo test                     # Run all tests
cargo test crypto::            # Test specific module
cargo test test_keypair_generation  # Test specific function
cargo test -- --nocapture      # Show println! output
cargo test -- --test-threads=1 --nocapture  # Sequential with output

# Code Quality
cargo fmt                      # Auto-format code
cargo fmt -- --check           # Check formatting only
cargo clippy                   # Run linter
cargo clippy -- -D warnings    # Fail on warnings
```

## Architecture

### Core Modules

**`crypto`** - Cryptographic primitives (IMPLEMENTED)
- `KeyPair`: Ed25519 signing keypair generation
- `UID`: Unique identifier derived from SHA-256 hash of public key (first 16 bytes as hex)
- Sign/verify operations using `ed25519-dalek`

**`protocol`** - Message envelope and serialization (IMPLEMENTED)
- `MessageEnvelope`: Contains `version`, `from_uid`, `to_uid`, `timestamp`, `payload`
- CBOR serialization for compact binary (production)
- JSON serialization for debugging/inspection
- Version compatibility checking

**`transport`** - Network layer (IMPLEMENTED)
- HTTP/1.1 server with POST `/output` endpoint for receiving messages
- Direct peer-to-peer message sending with CBOR serialization
- Async message handler callback system
- Peer management (add, remove, get, list)
- Delivery state tracking (Success, Queued, Retry, Failed)

**`storage`** - Local persistence (STUB)
- SQLite-based message and peer storage
- No sync, no cloud backups - intentionally local-only
- Full implementation planned for [v0.2](ROADMAP.md#-version-02---enhanced-core-q2-2025)

**`queue`** - Message retry logic (IMPLEMENTED)
- SQLite-backed persistent message queue
- Priority-based ordering (Urgent > High > Normal > Low)
- Exponential backoff for failed deliveries (base_delay * 2^attempts)
- Configurable max retries and base delay

### Data Flow

```
Sender Client                    Recipient Client
─────────────                    ─────────────
[UI] → [Queue]                   [POST /output] ← HTTP/2
         ↓                              ↓
   [Transport] ─── POST /output ───→ [Storage]
         ↓                              ↓
   [Storage] ← (retry if failed)     [UI]
```

### Error Handling

All operations return `Result<T>` with the `Error` enum from `lib.rs`:
- `Crypto(String)`: Cryptographic failures
- `JsonSerialization` / `CborSerialization`: Encoding issues
- `Transport(String)`: Network errors
- `Storage(String)`: Database failures
- `Queue(String)`: Message queue issues
- `Io`: Standard I/O errors

## Implementation Notes

### Cryptography (`crypto.rs`)

- UIDs are **deterministic**: same public key always produces the same UID
- Ed25519 keys are 32 bytes (public and private)
- Signatures are 64 bytes
- Use `ring` for SHA-256 hashing, `ed25519-dalek` for signing

### Protocol (`protocol.rs`)

- Protocol version is currently `1`
- Timestamps are Unix milliseconds (`chrono::Utc::now().timestamp_millis()`)
- CBOR is typically more compact than JSON - use CBOR for production transport
- Both serialization formats support full roundtrip without data loss

### Transport (`transport.rs`)

- Uses `hyper` and `hyper-util` for HTTP/1.1 server and client
- POST `/output` endpoint accepts CBOR-encoded MessageEnvelope
- Message handler callback receives incoming messages
- `send()` returns DeliveryState for retry logic integration
- Peers stored with UID, address, and public key

### Queue (`queue.rs`)

- SQLite schema with indexed priority and retry time
- `enqueue()`, `fetch_pending()`, `mark_delivered()`, `mark_failed()`
- Default: 5 max retries, 1000ms base delay
- Exponential backoff: delay = base_delay * 2^attempts
- Messages auto-removed after max retries exceeded

### CLI Client (`src/bin/cli.rs`)

A netcat-style REPL for testing P2P messaging. See [QUICKSTART.md](QUICKSTART.md) for full tutorial.

```bash
# Start first peer
cargo run --bin pure2p-cli -- --port 8080

# In another terminal, start second peer
cargo run --bin pure2p-cli -- --port 8081
```

**Commands:**
- `/connect <addr> <uid> <pubkey_hex>` - Add a peer
- `/send <uid> <message>` - Send a message
- `/peers` - List known peers
- `/whoami` - Show your UID and address
- `/help` - Show help
- `/quit` - Exit

**Example session:**
```
> /whoami
Your identity:
  UID:     a1b2c3d4e5f6...
  Address: 127.0.0.1:8080
  PubKey:  1a2b3c4d5e6f...

> /connect 127.0.0.1:8081 x9y8z7w6v5u4... 9a8b7c6d5e4f...
✓ Added peer x9y8z7w6v5u4... at 127.0.0.1:8081

> /send x9y8z7w6v5u4... Hello, peer!
✓ Message delivered to x9y8z7w6v5u4...
```

## Testing Conventions

- Every module has a `#[cfg(test)] mod tests` section
- Test names follow pattern: `test_<feature>_<scenario>`
- Crypto tests verify roundtrip (encode → decode → compare)
- Use `expect()` with descriptive messages for test setup that should never fail
- Test both success and failure cases (invalid signatures, corrupted data, etc.)

## Dependencies

Key crates:
- `ed25519-dalek`, `x25519-dalek`, `ring`: Cryptography
- `serde`, `serde_json`, `serde_cbor`: Serialization
- `tokio`: Async runtime
- `hyper`, `hyper-util`: HTTP transport
- `rusqlite`: SQLite storage
- `thiserror`: Error derivation
- `rustyline`: CLI REPL
- `clap`: Command-line argument parsing
- `colored`: Terminal colors

## Commit Style

Use conventional commits:
- `feat(module): description` - New features
- `fix(module): description` - Bug fixes
- `chore: description` - Maintenance

See [DEVELOPMENT.md](DEVELOPMENT.md#development-workflow) for full contribution workflow.

## Additional Documentation

- **[README.md](README.md)** — Project overview, philosophy, platform support
- **[QUICKSTART.md](QUICKSTART.md)** — CLI tutorial for new users
- **[DEVELOPMENT.md](DEVELOPMENT.md)** — Setup, build commands, troubleshooting
- **[ROADMAP.md](ROADMAP.md)** — Version timeline and planned features
