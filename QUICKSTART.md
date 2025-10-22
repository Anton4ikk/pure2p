# Quick Start Guide

Get up and running with Pure2P CLI in 5 minutes.

> **New to Pure2P?** Read the [README.md](README.md) for project overview and philosophy.
> **Developer?** See [DEVELOPMENT.md](DEVELOPMENT.md) for architecture and build details.

## Prerequisites

- **Rust toolchain** (1.70+)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustc --version && cargo --version
  ```

> Platform-specific requirements? See [DEVELOPMENT.md](DEVELOPMENT.md#platform-specific-requirements)

## Installation

```bash
git clone https://github.com/yourusername/pure2p.git
cd pure2p
cargo build --release
cargo test  # Verify installation
```

## Running the CLI

```bash
# Start on default port (8080)
cargo run --bin pure2p-cli

# Custom port
cargo run --bin pure2p-cli -- --port 9000
```

## Your First P2P Message

### Terminal 1 - Alice

```bash
$ cargo run --bin pure2p-cli -- --port 8080

> /whoami
Your identity:
  UID:     a1b2c3d4e5f60708...
  Address: 127.0.0.1:8080
  PubKey:  1a2b3c4d5e6f7a8b...
```

### Terminal 2 - Bob

```bash
$ cargo run --bin pure2p-cli -- --port 8081

> /whoami
Your identity:
  UID:     x9y8z7w6v5u43210...
  Address: 127.0.0.1:8081
  PubKey:  9a8b7c6d5e4f3a2b...

# Connect to Alice
> /connect 127.0.0.1:8080 a1b2c3d4e5f60708... 1a2b3c4d5e6f7a8b...
✓ Added peer

# Send message
> /send a1b2c3d4e5f60708... Hello Alice!
✓ Message delivered
```

### Terminal 1 - Alice receives

```
◀ [14:32:15] x9y8z7w6v5u43210...: Hello Alice!

> /connect 127.0.0.1:8081 x9y8z7w6v5u43210... 9a8b7c6d5e4f3a2b...
> /send x9y8z7w6v5u43210... Hi Bob!
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `/connect <addr> <uid> <pubkey>` | Add a peer |
| `/send <uid> <message>` | Send message |
| `/peers` | List peers |
| `/whoami` | Show your identity |
| `/help` | Show help |
| `/quit` | Exit |

## Testing on LAN

**Computer 1:**
```bash
cargo run --bin pure2p-cli -- --bind 0.0.0.0 --port 8080
```

**Computer 2:**
```bash
# Use Computer 1's IP address
> /connect 192.168.1.100:8080 <uid> <pubkey>
```

## Troubleshooting

**Port in use:**
```bash
cargo run --bin pure2p-cli -- --port 9000
```

**Can't connect:**
- Verify peer is running
- Check firewall allows incoming connections
- Use `--bind 0.0.0.0` to accept external connections

**Build issues:**
- See [DEVELOPMENT.md](DEVELOPMENT.md#troubleshooting) for build and dependency problems

**Find your IP:**
```bash
# macOS/Linux
ifconfig | grep "inet "

# Windows
ipconfig
```

## Next Steps

- **Understand the architecture**: Read [DEVELOPMENT.md](DEVELOPMENT.md#project-structure) for module details
- **Learn about limitations**: See current v0.1 constraints in [README.md](README.md#limitations-)
- **Explore upcoming features**: Check [ROADMAP.md](ROADMAP.md) for v0.2+ (encryption, storage, GUI)
- **Contribute**: Review [DEVELOPMENT.md](DEVELOPMENT.md#development-workflow) for contribution workflow
