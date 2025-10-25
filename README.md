<div align="center">

# [Pure2P](https://pure2p.com)

**True P2P Messenger - No Servers, No Compromises**

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![P2P](https://img.shields.io/badge/architecture-P2P-green.svg)]()
[![No Servers](https://img.shields.io/badge/servers-none-red.svg)]()

*Direct peer-to-peer messaging with radical honesty about trade-offs.*

[Quick Start](#-quick-start) • [How It Works](#-how-it-works) • [Status](#-status) • [Docs](DEVELOPMENT.md)

</div>

---

## 📖 What Is Pure2P?

A **radically honest P2P messenger** that prioritizes privacy over convenience.

### You Get
- ✅ Absolute privacy — no metadata leaks
- ✅ No trust required in operators/intermediaries
- ✅ Full control of your data

### You Accept
- ⚠️ Delivery delays (both peers must be online)
- ⚠️ No push notifications
- ⚠️ Manual peer management
- ⚠️ No message history if device is lost

### Core Principles
- **Direct P2P only** - No servers, relays, DHT, or push services
- **Local-only storage** - No sync, no cloud
- **Manual contact exchange** - UIDs shared through external channels
- **Online-only delivery** - Messages require simultaneous peer presence

---

## 🚀 Quick Start

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build and run
git clone https://github.com/yourusername/pure2p.git
cd pure2p
cargo run --bin pure2p-tui
```

**Navigate:** ↑↓ or j/k | **Select:** Enter | **Back:** b/Esc | **Quit:** q

---

## 🏗️ How It Works

### Architecture

```
Alice's Client          Bob's Client
──────────────         ──────────────
[Send Message]   ───→  [POST /output]
      ↓                      ↓
   [Queue]              [Delivered]
   (retry)
```

**Flow:**
1. Alice POSTs message to Bob's `/output` endpoint
2. **Success?** Bob responds 200 → delivered
3. **Offline?** Message queued with exponential backoff
4. **Retry** when Bob comes online

### Key Tech
- **Crypto**: Ed25519 (signing/identity), X25519 (key exchange), SHA-256 UIDs, ECDH shared secrets
- **Protocol**: CBOR message envelopes
- **Transport**: HTTP/1.1 (`/output`, `/ping`, `/message`)
- **Queue**: SQLite with retry backoff
- **Storage**: Local-only (contacts, chats, settings)

---

## 💻 Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| **macOS** | ✅ | TUI client (Intel + Apple Silicon) |
| **Linux** | ✅ | TUI client (x86_64 + ARM64) |
| **Windows** | ✅ | TUI client (x86_64) |
| **Android** | 🔄 | Core ready, GUI pending |
| **iOS** | 🔄 | Core ready, GUI pending |

**Planned:** NAT Traversal, E2E Encryption (v0.3) • Desktop apps (v0.4) • Mobile apps (v0.5)

See [ROADMAP.md](ROADMAP.md) for timeline.

---

## 🎯 Status (v0.3 - In Progress)

### Implemented ✅

**Core:**
- Ed25519 keypairs (signing/identity), X25519 keypairs (key exchange)
- ECDH shared secret derivation (encryption-ready)
- SHA-256 UID generation from Ed25519 pubkeys
- CBOR serialization, HTTP transport
- SQLite queue with retry
- Contact tokens with dual pubkeys (Ed25519 + X25519)
- State persistence

**NAT Traversal:**
- IPv6 direct connectivity detection
- PCP (Port Control Protocol, RFC 6887) with auto-renewal
- NAT-PMP (RFC 6886) with external IP detection
- UPnP IGD with auto-cleanup
- CGNAT detection (RFC 6598, 100.64.0.0/10)
- Automatic fallback orchestration (IPv6 → PCP → NAT-PMP → UPnP)
- Cross-platform gateway discovery (Linux, macOS, Windows)

**TUI:**
- Contact share/import with validation
- Chat list with status badges (● ⌛ ⚠ ○)
- Real-time messaging
- Delete with confirmation
- Settings with auto-save
- Startup sync progress
- Diagnostics screen (port forwarding status, CGNAT warnings)

### In Progress 🔄

- End-to-end message encryption (using X25519 shared secrets)
- Message authentication/signing

### Limitations ⚠️

- Messages currently unencrypted (E2E encryption in progress)
- Text only — rich media in v0.4
- Manual peer management
- CGNAT users need relay (future consideration)

**This is a prototype.** See [ROADMAP.md](ROADMAP.md) for planned features.

---

## 🤝 Contributing

1. **Discuss**: Open an issue
2. **Develop**: Fork, branch, code
3. **Test**: `cargo test && cargo clippy`
4. **Submit**: PR with clear description

See [DEVELOPMENT.md](DEVELOPMENT.md) for setup.

### Must Maintain
- Direct P2P only
- No servers/relays/intermediaries
- Local-only storage
- Transparency about limitations

---

## 🎯 Why Pure2P?

**Problem:** Modern messengers compromise privacy
- Signal/WhatsApp: Servers see metadata (who, when)
- Telegram/Matrix: Federation requires server trust
- "P2P" apps: Often use hidden relays

**Solution:** Different trade-off
- **Privacy first**: No metadata, no trust needed
- **Honest about cost**: Delays, no push, manual setup

**For those who value privacy over convenience.**

---

## 📚 Documentation

- **[DEVELOPMENT.md](DEVELOPMENT.md)** - Setup, architecture, API
- **[ROADMAP.md](ROADMAP.md)** - Version timeline
- **[CLAUDE.md](CLAUDE.md)** - Implementation notes

---

## 📄 License

MIT License - see [LICENSE](LICENSE)

---

<div align="center">

**Privacy-first messaging 🔒**

[Get Started](#-quick-start) • [Contribute](#-contributing) • [Roadmap](ROADMAP.md)

</div>
