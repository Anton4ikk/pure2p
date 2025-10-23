# Pure2P Roadmap

Development timeline and planned features.

> **Quick Links**: [README](README.md) • [Development](DEVELOPMENT.md) • [Claude Docs](CLAUDE.md)

---

## ✅ v0.1 - Foundation (COMPLETED)

**Released:** Core P2P messaging with TUI client

### Implemented
- Ed25519 keypairs, UID derivation, CBOR/JSON serialization
- HTTP transport (`/output`, `/ping`, `/message`)
- SQLite queue with exponential backoff + startup retry
- Contact/Chat structures, token system (base64 CBOR)
- AppState persistence (JSON/CBOR)
- Settings management (auto-save, thread-safe)
- Messaging API (send, auto-queue, chat lifecycle, smart delete)

**TUI Features:**
- Main menu, share/import contacts
- Chat list (● ⌛ ⚠ ○ status badges)
- Chat view, delete with confirmation
- Settings editor, startup sync progress

### Limitations
- No encryption (plaintext) → v0.3
- No persistent storage (SQLite pending) → v0.2
- Text-only → v0.4
- No NAT traversal → v0.3

---

## 🔨 v0.2 - TUI Interface (COMPLETED)

**Focus:** Terminal user interface

### Completed
- **TUI Client**
  - [x] Screen-based state machine with ratatui
  - [x] Main menu navigation
  - [x] Contact share/import with QR-like tokens
  - [x] Chat list with status badges
  - [x] Chat view with message history
  - [x] Settings editor with auto-save
  - [x] Startup sync progress
  - [x] Comprehensive keyboard navigation

- **Storage Foundation**
  - [x] AppState persistence (JSON/CBOR)
  - [x] Contact/Chat structures
  - [x] Settings management
  - [x] Token generation system

---

## 🔐 v0.3 - NAT Traversal

**Focus:** P2P across NAT + E2E encryption

### Planned
- **E2E Encryption**
  - X25519 key exchange + ChaCha20-Poly1305
  - Ed25519 signatures
  - Per-session ephemeral keys
  - Forward secrecy

- **NAT Traversal**
  - STUN-like protocol (optional, self-hosted)
  - UDP hole punching
  - ICE-inspired negotiation
  - Manual port forwarding fallback

- **Peer-Assisted Discovery**
  - Reachable peers for coordination
  - Manual "introducer peer" selection
  - No DHT/bootstrap servers

### Philosophy
- NAT traversal is **optional**
- Users choose: manual port forward (most private) vs self-hosted STUN vs community STUN
- **Never** introduce relay servers

---

## 🖥️ v0.4 - Desktop Apps

**Focus:** Tauri-based desktop apps + rich media

### Planned
- **Desktop App**
  - Tauri (Rust + web frontend)
  - System tray, local notifications
  - Modern chat interface
  - Auto-start, background service

- **Platform Support**
  - macOS (Intel + Apple Silicon)
  - Windows x64
  - Linux (AppImage/Flatpak)

- **Rich Messages**
  - File attachments, images
  - Reactions, typing indicators
  - Media preview

- **Enhanced Transport**
  - HTTP/2, TLS, connection pooling

### Challenges
- Balancing "always-on" with privacy
- Firewall/port forwarding UX

---

## 📱 v0.5 - Mobile Apps

**Focus:** iOS and Android clients

### Planned
- **Native Apps**
  - iOS: Swift UI + Rust core (FFI)
  - Android: Kotlin UI + Rust core (JNI)
  - QR code UID exchange
  - Foreground service for reception

- **Mobile UX**
  - Clear "no push" messaging
  - Battery impact transparency
  - Network switching (WiFi ↔ Cellular)
  - Optimized for mobile constraints

### Known Limits
- No background reception (platform constraint)
- No push (by design)
- App must run to receive
- Battery usage when active

> Fundamental constraints of Pure2P architecture

---

## 🔮 Post-1.0 Ideas (No Timeline)

Exploratory features for future consideration.

- **Advanced Crypto**: Forward secrecy, post-quantum, zero-knowledge
- **Multi-Device**: Local network linking, manual auth, no cloud sync
- **Groups**: Mesh topology, no coordinator, full state per peer
- **Federation Alt**: Optional "bridge peers" for async (user-controlled, clear trade-offs)

---

## 📊 Status Legend

| Symbol | Status |
|--------|--------|
| ✅ | Completed |
| 🔨 | In Progress |
| 📝 | Planned |
| 🔮 | Future |

---

## 📅 Release Schedule

| Version | Target | Status |
|---------|--------|--------|
| v0.1 | 2025-10 | ✅ Released |
| v0.2 | 2025-10 | ✅ Released |
| v0.3 | TBD | 📝 Planning |
| v0.4 | TBD | 📝 Planning |
| v0.5 | TBD | 📝 Planning |

*Timeline subject to change based on community feedback.*

---

## 🤝 Contributing

**Get Involved:**
- GitHub Discussions for ideas
- GitHub Issues for bugs/features
- Pull Requests (see [DEVELOPMENT.md](DEVELOPMENT.md))

**Must Align With:**
- No servers/relays/intermediaries
- No compromises on P2P purity
- Transparency about limitations

---

## Related Docs

- **[README.md](README.md)** — Overview and philosophy
- **[DEVELOPMENT.md](DEVELOPMENT.md)** — Architecture and build
- **[CLAUDE.md](CLAUDE.md)** — Implementation details
