# Pure2P Roadmap

Development timeline and planned features.

> **Quick Links**: [README](README.md) • [Development](DEVELOPMENT.md) • [Usage Guide](USAGE.md)

---

## 🔐 v0.3 - NAT Traversal & E2E Encryption ✅

**Focus:** P2P across NAT + E2E encryption

### Completed ✅
- **E2E Encryption**
  - X25519 ECDH key exchange + XChaCha20-Poly1305 AEAD
  - Ed25519 signatures for message authentication
  - Contact token signing and verification
  - Encrypted and plaintext message support

- **NAT Traversal**
  - IPv6 direct connectivity detection
  - PCP (Port Control Protocol, RFC 6887) with auto-renewal
  - NAT-PMP (RFC 6886) with external IP detection
  - UPnP IGD with auto-cleanup
  - CGNAT detection (RFC 6598)
  - Automatic fallback orchestration (IPv6 → PCP → NAT-PMP → UPnP)
  - Cross-platform gateway discovery (Linux, macOS, Windows)
  - Automatic connectivity on startup

### Future Enhancements (Post-1.0)
- Per-session ephemeral keys for forward secrecy
- UDP hole punching
- Peer-assisted discovery (optional)

### Philosophy
- NAT traversal is **automatic** with graceful fallback
- Users can manually port forward if preferred (most private)
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

- **Voice Calls**

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
| v0.3 | 2025-10 | ✅ Released |
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
- **[USAGE.md](USAGE.md)** — Implementation details
