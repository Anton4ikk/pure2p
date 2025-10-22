# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Pure2P is a **radically honest P2P messenger** with no servers, relays, or intermediaries. Key architectural principles:

- **Direct peer-to-peer only**: Each client exposes a `POST /output` endpoint for receiving messages
- **Manual UID exchange**: UIDs are derived from Ed25519 public key fingerprints and shared manually
- **Local-only storage**: No sync, no cloud - device loss means history loss
- **Online-only delivery**: Messages queue locally until both peers are online simultaneously
- **No push notifications**: Mobile apps can't wake on external requests

This is a Rust library (`lib`, `staticlib`, `cdylib`) for cross-platform use (Android, iOS, Desktop).

## Build & Test Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Optimized release build
cargo check                    # Fast compile check without building

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

**`transport`** - Network layer (STUB)
- Will handle direct HTTP/2 POST connections between peers
- Peer discovery via manual address exchange
- Online presence broadcasts

**`storage`** - Local persistence (STUB)
- SQLite-based message and peer storage
- No sync, no cloud backups - intentionally local-only

**`queue`** - Message retry logic (STUB)
- Priority-based message queuing
- Exponential backoff for failed deliveries
- Queues messages when peer is offline

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

### Stub Modules

The `transport`, `storage`, and `queue` modules have placeholder implementations marked with `// TODO`. When implementing:

1. **Transport**: Use `hyper` for HTTP/2, expose POST endpoint, maintain peer connection state
2. **Storage**: Use `rusqlite` with bundled SQLite, create schema for messages/peers/queue
3. **Queue**: Implement priority queue with exponential backoff (base delay * 2^attempts)

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
- `hyper`: HTTP/2 transport
- `rusqlite`: SQLite storage
- `thiserror`: Error derivation

## Commit Style

Use conventional commits:
- `feat(module): description` - New features
- `fix(module): description` - Bug fixes
- `chore: description` - Maintenance
