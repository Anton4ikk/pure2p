# Pure2P Roadmap

Development timeline and planned features.

> **Quick Links**: [README](README.md) • [Quick Start](QUICKSTART.md) • [Development](DEVELOPMENT.md) • [Claude Docs](CLAUDE.md)

---

## ✅ Version 0.1 - CLI Prototype (COMPLETED)

**Status:** Released (v0.2 foundation work in progress on `dev` branch)

Foundational P2P messaging with command-line interface.

### Implemented Features (v0.1)

- [x] Ed25519 keypair generation and UID derivation
- [x] CBOR/JSON message serialization
- [x] HTTP/1.1 transport with POST `/output` endpoint
- [x] SQLite message queue with exponential backoff retry
- [x] Netcat-style CLI REPL
- [x] Cross-platform support (macOS, Linux, Windows)

### Foundation Work for v0.2 (on `dev` branch)

- [x] Message queue startup retry (resends pending messages on app launch)
- [x] Contact management structures with expiry tracking
- [x] Contact token generation/parsing (base64-encoded CBOR)
- [x] Chat conversation structures
- [x] Chat pending message tracking (`has_pending_messages` flag)
- [x] Application state persistence (JSON/CBOR)
- [x] Global settings management
- [x] AppState chat management methods (get_chat, get_or_create_chat, sync_pending_status)
- [x] Queue method to get pending contact UIDs
- [x] Transport `/ping` endpoint for connectivity checks
- [x] Transport `/message` endpoint with flexible message types
- [x] Transport client methods: `send_ping()`, `send_message()`
- [x] **Messaging module** - High-level API combining transport + queue + storage
  - [x] Message sending with automatic retry queueing
  - [x] Chat lifecycle management (ping-based creation, active/inactive states)
  - [x] Delete chat propagation (smart deletion based on online status)
  - [x] Extensible message type support (text, delete_chat, typing, etc.)
  - [x] Incoming message handling (`handle_incoming_message()`) with auto-chat creation
- [x] **Enhanced MessageEnvelope** with UUID and MessageType enum
  - [x] Unique message IDs (UUID v4)
  - [x] MessageType enum (Text, Delete) for extensible message types
  - [x] Convenience constructors (`new_text()`, `new_delete()`)

### Current Limitations

- Manual UID/address exchange required
- No NAT traversal (LAN or port-forwarded only) — see [v0.5](#-version-05---nat-traversal-q1-2026)
- No persistent storage (foundation structures ready, SQLite integration pending) — see [v0.2](#-version-02---enhanced-core-q2-2025)
- No encryption (plaintext payloads) — see [v0.2](#-version-02---enhanced-core-q2-2025)
- Text-only messaging — see [v0.2](#-version-02---enhanced-core-q2-2025)

> Learn how to use the CLI: [QUICKSTART.md](QUICKSTART.md)

---

## 🔨 Version 0.2 - Enhanced Core (Q2 2025)

**Status:** In Development

Focus: Storage, encryption, and rich messages.

### In Progress Features

- [ ] **Persistent Storage**
  - [x] Foundation: AppState, Contact, Chat, Settings structures (on `dev`)
  - [x] Contact token generation/parsing (on `dev`)
  - [x] Message queue with startup retry (on `dev`)
  - [x] Chat pending message tracking (on `dev`)
  - [ ] SQLite integration for contacts and chats
  - [ ] Message history persistence
  - [ ] Search and filtering
  - [ ] Export/import functionality

- [ ] **Enhanced Transport & Messaging**
  - [x] `/ping` endpoint for connectivity checks (on `dev`)
  - [x] `/message` endpoint with flexible message types (on `dev`)
  - [x] Client methods for ping and message sending (on `dev`)
  - [x] High-level messaging API with auto-retry (on `dev`)
  - [x] Chat lifecycle (ping-based creation, active/inactive) (on `dev`)
  - [x] Delete chat propagation (on `dev`)
  - [ ] Message type handlers (typing indicators, read receipts)
  - [ ] Integration with persistent storage (SQLite)

- [ ] **End-to-End Encryption**
  - X25519 key exchange + ChaCha20-Poly1305
  - Ed25519 message signatures (see [CLAUDE.md](CLAUDE.md#cryptography-cryptors) for Ed25519 implementation)
  - Per-session ephemeral keys

- [ ] **Enhanced Transport**
  - HTTP/2 multiplexing
  - Connection pooling
  - TLS support
  - Bandwidth optimization

- [ ] **Rich Message Types**
  - File attachments
  - Image previews
  - Message reactions
  - Typing indicators (optional)

---

## 🖥️ Version 0.3 - Desktop Clients (Q3 2025)

Focus: GUI applications for desktop platforms.

### Planned Features

- [ ] **Cross-Platform Desktop App**
  - Tauri-based UI (Rust + web frontend)
  - Native system tray
  - Desktop notifications (local)
  - Modern chat interface
  - Built on [Rust core](DEVELOPMENT.md#project-structure) from v0.1-0.2

- [ ] **Desktop Features**
  - Always-on background service
  - Auto-start on boot
  - Contact book with QR codes
  - Clipboard integration

- [ ] **Platform Support**
  - macOS (Intel + Apple Silicon)
  - Windows (x64)
  - Linux (AppImage / Flatpak)

### Challenges

- Balancing "always-on" with privacy
- Firewall/port forwarding UX
- Cross-platform background services

---

## 📱 Version 0.4 - Mobile Clients (Q4 2025)

Focus: iOS and Android applications.

### Planned Features

- [ ] **Native Mobile Apps**
  - iOS: Swift UI + Rust core (FFI)
  - Android: Kotlin UI + Rust core (JNI)
  - In-app UID exchange (QR codes)
  - Foreground service for reception

- [ ] **Mobile UX**
  - Clear "no push" messaging
  - Battery impact transparency
  - Foreground service indicator
  - Network switching (WiFi ↔ Cellular)

### Known Limitations

- No background message reception (platform constraint)
- No push notifications (by design — see [README.md](README.md#core-principles))
- App must be running to receive messages
- Battery usage when active

> These are fundamental constraints of Pure2P's architecture — see [README.md](README.md#what-this-means)

---

## 🌐 Version 0.5 - NAT Traversal (Q1 2026)

Focus: P2P connectivity across NAT without central servers.

### Planned Features

- [ ] **Hole Punching**
  - STUN-like protocol (optional, self-hosted)
  - UDP hole punching
  - ICE-inspired negotiation
  - Manual port forwarding fallback

- [ ] **Peer-Assisted Discovery**
  - Use reachable peers for coordination
  - Manual "introducer peer" selection
  - No DHT, no bootstrap servers

### Philosophy

- NAT traversal must be **optional**
- Users choose privacy vs. convenience:
  - Manual port forwarding (most private)
  - Self-hosted STUN (semi-private)
  - Community STUN (convenient, less private)
- **Never** introduce relay servers

> This maintains Pure2P's core principle: [no servers or intermediaries](README.md#core-principles)

---

## 🔮 Post-1.0 Ideas (No Timeline)

Exploratory features for future consideration.

### Advanced Cryptography

- Forward secrecy (Double Ratchet)
- Post-quantum key exchange
- Zero-knowledge proofs

### Multi-Device Support

- Local network device linking
- Manual device authorization
- No cloud sync

### Group Messaging

- Fully decentralized groups
- Mesh topology (no coordinator)
- Each peer maintains full state

### Federation Alternative

- Optional "bridge peers" for async delivery
- User-controlled, self-hosted only
- Clear privacy trade-offs

---

## 📊 Version Status

| Symbol | Status | Description |
|--------|--------|-------------|
| ✅ | Completed | Feature implemented and tested |
| 🔨 | In Progress | Currently being developed |
| 📝 | Planned | Design phase, not started |
| 🔮 | Future | Post-1.0 consideration |

---

## 🤝 Contributing

We welcome community input on priorities and features.

**Get Involved:**
- GitHub Discussions for ideas and feedback
- GitHub Issues for bugs and feature requests
- Pull Requests for code contributions (see [DEVELOPMENT.md](DEVELOPMENT.md#development-workflow))

**Philosophy First:**
All proposals must align with Pure2P's core principles:
- No servers, relays, or intermediaries (see [README.md](README.md#core-principles))
- No compromises on P2P purity
- Transparency about limitations

---

## 📅 Release Schedule

| Version | Target | Status |
|---------|--------|--------|
| v0.1 | 2025-01 | ✅ Released |
| v0.2 | 2025 Q2 | 📝 Planning |
| v0.3 | 2025 Q3 | 📝 Planning |
| v0.4 | 2025 Q4 | 📝 Planning |
| v0.5 | 2026 Q1 | 📝 Planning |

*Timeline subject to change based on community feedback and development progress.*

---

## Related Documentation

- **[README.md](README.md)** — Project overview and philosophy
- **[QUICKSTART.md](QUICKSTART.md)** — Get the CLI running in 5 minutes
- **[DEVELOPMENT.md](DEVELOPMENT.md)** — Architecture, build instructions, API reference
- **[CLAUDE.md](CLAUDE.md)** — Implementation details for AI assistants
