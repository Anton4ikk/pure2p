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

**`protocol`** - Message envelope and serialization (ENHANCED)
- `MessageEnvelope`: Contains `id` (UUID), `version`, `from_uid`, `to_uid`, `timestamp`, `message_type`, `payload`
- `MessageType` enum: Text, Delete (extensible for future types)
- CBOR serialization for compact binary (production)
- JSON serialization for debugging/inspection
- Version compatibility checking
- Convenience constructors: `new_text()`, `new_delete()`

**`transport`** - Network layer (ENHANCED - `dev` branch)
- HTTP/1.1 server with multiple endpoints:
  - POST `/output` - Legacy message endpoint (MessageEnvelope)
  - POST `/ping` - Connectivity checks (returns UID + status)
  - POST `/message` - New message endpoint (MessageRequest with from_uid, message_type, payload)
- Direct peer-to-peer message sending with CBOR serialization
- Async message handler callback system (dual handlers for /output and /message)
- Peer management (add, remove, get, list)
- Delivery state tracking (Success, Queued, Retry, Failed)
- **New**: Ping functionality for contact validation and connectivity testing
- **New**: Flexible message types (text, delete, typing, etc.)

**`storage`** - Local persistence (FOUNDATION STRUCTURES - `dev` branch)
- Contact management structures with expiry and active status tracking
- Contact token generation/parsing (base64-encoded CBOR)
- Chat conversation structures with message history
- **New**: `has_pending_messages` flag for UI highlighting
- Application state persistence (JSON/CBOR serialization)
- Global settings management
- **New**: AppState methods for chat management (get_chat, get_or_create_chat, sync_pending_status)
- No sync, no cloud backups - intentionally local-only
- **Status**: Data structures complete, SQLite integration in progress for [v0.2](ROADMAP.md#-version-02---enhanced-core-q2-2025)

**`queue`** - Message retry logic (ENHANCED - `dev` branch)
- SQLite-backed persistent message queue
- Priority-based ordering (Urgent > High > Normal > Low)
- Exponential backoff for failed deliveries (base_delay * 2^attempts)
- Configurable max retries and base delay
- **New**: Startup retry - automatically resends all pending messages when app launches
- **New**: Enhanced schema with message_type, target_uid, and retry_count tracking
- **New**: `get_pending_contact_uids()` for syncing pending message flags with chats

**`messaging`** - High-level messaging API (ENHANCED - `dev` branch)
- User-facing message sending with automatic retry queueing
- Chat lifecycle management (create from ping, active/inactive states)
- Delete chat propagation with smart logic (inactive=immediate, active=notify+delete)
- **New**: Incoming message handling with automatic chat creation
- **Functions**:
  - `send_message()` - Send with auto-queue on failure
  - `send_message_with_type()` - Send with custom message type
  - `send_delete_chat()` - Send delete notification
  - `handle_delete_chat()` - Process incoming delete requests
  - `handle_incoming_message()` - Process received messages (create chat if needed, append to history, mark unread)
  - `create_chat_from_ping()` - Create chat based on ping success/failure
  - `create_active_chat()` / `create_inactive_chat()` - Direct chat creation
  - `delete_chat()` - Smart deletion (checks is_active flag)
  - `delete_inactive_chat_immediate()` - Force immediate delete
  - `delete_active_chat_with_notification()` - Force notification send

### Data Flow

```
Sender Client                           Recipient Client
─────────────                           ─────────────
[UI/App]                                [POST /message] ← HTTP/1.1
    ↓                                          ↓
[Messaging API]                         [NewMessageHandler]
    ↓                                          ↓
Try send via Transport ────────────→    [AppState/Chat]
    ↓         (CBOR/HTTP)                     ↓
Success? Yes → Done                          [UI]
    ↓
    No → [Queue]
         (SQLite)
           ↓
    [Retry with backoff]
           ↓
    Try send again...
```

**Message Types Supported:**
- `text` - Regular text messages
- `delete_chat` - Chat deletion notifications
- `typing` - Typing indicators (future)
- `read_receipt` - Read receipts (future)
- Custom types for extensibility

### Storage Structures

**`Contact`** - Peer information with expiry
- `uid`: Unique identifier (derived from public key)
- `ip`: IP address and port (e.g., "192.168.1.100:8080")
- `pubkey`: Ed25519 public key bytes
- `expiry`: Expiration timestamp
- `is_active`: Contact active status
- Methods: `is_expired()`, `activate()`, `deactivate()`

**`Chat`** - Conversation with a contact
- `contact_uid`: UID of the peer
- `messages`: Vector of Message objects
- `is_active`: Online/reachable status (true = online, false = offline)
- `has_pending_messages`: Flag for queued outgoing messages
- Methods: `append_message()`, `mark_unread()`, `mark_read()`, `mark_has_pending()`, `mark_no_pending()`

**`AppState`** - Global application state
- `contacts`: List of Contact objects
- `chats`: List of Chat objects
- `message_queue`: Message IDs awaiting delivery
- `settings`: Application settings
- Supports JSON and CBOR serialization
- Methods: `save()`, `load()`, `save_cbor()`, `load_cbor()`

**`Settings`** - Persistent configuration (JSON file)
- `retry_interval_minutes`: Retry interval in minutes (default 10)
- `storage_path`: Path for application data (default "./data")
- `default_contact_expiry_days`: Contact token validity (default 30)
- `max_message_retries`: Retry limit (default 5)
- `retry_base_delay_ms`: Initial retry delay (default 1000)
- `global_retry_interval_ms`: Periodic retry interval (default 600,000 = 10 min)
- `enable_notifications`: Notification toggle
- Methods:
  - `load(path)` - Load from JSON file (returns defaults if not found)
  - `save(path)` - Save to JSON file (creates parent dirs if needed)
  - `update_retry_interval(minutes, path)` - Update interval and auto-save
  - `get_retry_interval_minutes()` - Get retry interval in minutes
  - `set_global_retry_interval_ms(ms)` - Set interval in milliseconds (syncs minutes)

**`SettingsManager`** - Thread-safe settings API for UI layers
- Built on `Arc<RwLock<Settings>>` for concurrent access
- Async-first API with tokio::sync::RwLock
- Automatic persistence on all updates
- Cloneable for sharing across threads/tasks
- Methods:
  - `new(path)` - Create manager and load settings
  - `get_retry_interval_minutes()` - Async getter
  - `set_retry_interval_minutes(minutes)` - Async setter with auto-save
  - `get_storage_path()` / `set_storage_path(path)` - Storage path access
  - `get_notifications_enabled()` / `set_notifications_enabled(bool)` - Toggle notifications
  - `get_max_message_retries()` / `set_max_message_retries(u32)` - Retry config
  - `get_default_contact_expiry_days()` / `set_default_contact_expiry_days(u32)` - Contact expiry
  - `get_all()` - Get clone of all settings
  - `update(fn)` - Update multiple settings atomically
  - `reload()` - Reload from disk
  - `save()` - Manually trigger save

**Contact Tokens**
- `generate_contact_token(ip, pubkey, expiry)`: Creates base64-encoded CBOR token
- `parse_contact_token(token)`: Decodes token and validates expiry
- Used for manual peer exchange and identity sharing

**Transport Structures**

**`PingResponse`** - Connectivity check response
- `uid`: UID of the responding peer
- `status`: Status message (typically "ok")
- Used to validate contact availability and identity

**`MessageRequest`** - New message format for /message endpoint
- `from_uid`: Sender's UID
- `message_type`: Message type (e.g., "text", "delete", "typing")
- `payload`: Arbitrary binary payload
- Replaces MessageEnvelope for new message flows
- Enables flexible message types for rich functionality

### Error Handling

All operations return `Result<T>` with the `Error` enum from `lib.rs`:
- `Crypto(String)`: Cryptographic failures
- `JsonSerialization` / `CborSerialization`: Encoding issues
- `Transport(String)`: Network errors
- `Storage(String)`: Database failures, invalid tokens, expired contacts
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
- Message IDs are UUID v4 (`uuid::Uuid::new_v4()`)
- Timestamps are Unix milliseconds (`chrono::Utc::now().timestamp_millis()`)
- `MessageType` enum distinguishes between Text and Delete messages (extensible)
- CBOR is typically more compact than JSON - use CBOR for production transport
- Both serialization formats support full roundtrip without data loss
- Use `new_text()` or `new_delete()` convenience methods for common message types

### Transport (`transport.rs`)

- Uses `hyper` and `hyper-util` for HTTP/1.1 server and client
- **Endpoints**:
  - POST `/output` - Legacy endpoint, accepts CBOR-encoded MessageEnvelope
  - POST `/ping` - Returns CBOR-encoded PingResponse with UID and status
  - POST `/message` - New endpoint, accepts CBOR-encoded MessageRequest
- **Client methods**:
  - `send()` - Sends to /output endpoint (legacy)
  - `send_ping(contact)` - Pings a contact, returns PingResponse
  - `send_message(contact, from_uid, message_type, payload)` - Sends to /message endpoint
- **Handlers**:
  - `MessageHandler` - Legacy callback for /output messages
  - `NewMessageHandler` - Callback for /message endpoint (designed for AppState integration)
- `set_local_uid()` - Configures local UID for ping responses
- Peers stored with UID, address, and public key

### Queue (`queue.rs`)

- SQLite schema with indexed priority and retry time
- Enhanced schema: `message_id`, `target_uid`, `message_type`, `payload`, `retry_count`, `last_attempt`
- `enqueue()`, `enqueue_with_type()`, `fetch_pending()`, `fetch_all_pending()`, `dequeue()`
- `mark_delivered()`, `mark_failed()`, `retry_pending_on_startup()`
- Default: 5 max retries, 1000ms base delay
- Exponential backoff: delay = base_delay * 2^attempts
- Messages auto-removed after max retries exceeded
- **Startup retry**: On app launch, `retry_pending_on_startup()` attempts delivery of all queued messages

### Storage (`storage.rs`)

- Contact token system for easy peer exchange
- Base64-encoded CBOR tokens containing: IP, public key, expiry
- `Contact` struct tracks peer status and expiry
- `Chat` struct manages conversation history per contact
  - `has_pending_messages` flag for UI notification badges
  - `mark_has_pending()` / `mark_no_pending()` methods
  - `is_active` for online/offline status (managed by ping)
- `AppState` provides unified state management with JSON/CBOR persistence
  - `sync_pending_status(pending_uids)` - Updates chat pending flags from queue
  - `get_chat(uid)` / `get_chat_mut(uid)` - Retrieve chat by contact UID
  - `get_or_create_chat(uid)` - Get existing or create new chat
  - `add_chat(uid)` - Add new chat
- `Settings` struct provides persistent configuration management
  - Load/save from JSON file with `load()` and `save()` methods
  - Auto-save on updates via `update_retry_interval()`
  - Automatic sync between `retry_interval_minutes` and `global_retry_interval_ms`
  - Creates parent directories automatically on save
- Active/inactive status tracking for contacts and chats
- Expiry validation prevents use of outdated contact information

### Messaging (`messaging.rs`)

High-level API combining transport, queue, and storage for user-facing operations.

**Message Sending:**
- `send_message(transport, queue, contact, message, priority)` - Send with automatic queueing on failure
- `send_message_with_type()` - Send with custom message type (text, delete_chat, typing, etc.)
- Returns `true` if delivered immediately, `false` if queued for retry
- Failed messages automatically queued with specified priority

**Chat Lifecycle:**
- `create_chat_from_ping(transport, app_state, contact)` - Ping contact and create chat
  - Success: Creates active chat (contact is online)
  - Failure: Creates inactive chat (contact is offline)
- `create_active_chat(app_state, contact_uid)` - Create/activate chat without ping
- `create_inactive_chat(app_state, contact_uid)` - Create/deactivate chat without ping
- Chat state managed via `is_active` flag

**Chat Deletion:**
- `delete_chat(transport, queue, app_state, contact, local_uid)` - Smart deletion
  - Inactive chat: Delete immediately without notification
  - Active chat: Send delete_chat message, then delete locally
- `delete_inactive_chat_immediate(app_state, contact_uid)` - Force immediate delete (errors if active)
- `delete_active_chat_with_notification()` - Force notification send regardless of state
- `handle_delete_chat(app_state, sender_uid)` - Process incoming delete request

**Incoming Message Handling:**
- `handle_incoming_message(app_state, sender_uid, recipient_uid, message_id, content, timestamp)` - Process received messages
  - Automatically creates chat if not found using `get_or_create_chat()`
  - Appends message to chat history
  - Marks chat as unread (`is_active = true`) for TUI display
  - Logs message receipt for debugging

**Design Principles:**
- Automatic retry handling - transparent to caller
- Priority support for urgent messages (delete_chat uses Urgent)
- State preservation - messages/flags maintained during lifecycle changes
- Extensible message types for future features
- Automatic chat creation on first message from unknown peer

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
- `base64`: Contact token encoding
- `chrono`: Timestamp and expiry management
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
