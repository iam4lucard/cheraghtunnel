<div align="center">
 
# 🕯️ CheraghTunnel

**Advanced, High-Performance Reverse Tunneling System & Stealth Proxy Engine — Built with Rust**

[🇮🇷 راهنمای فارسی (Persian Documentation)](README_FA.md)
 
[![GitHub Release](https://img.shields.io/github/v/release/iam4lucard/cheraghtunnel?style=for-the-badge&logo=github&color=f59e0b)](https://github.com/iam4lucard/cheraghtunnel/releases/latest)
[![Build Status](https://img.shields.io/github/actions/workflow/status/iam4lucard/cheraghtunnel/release.yml?style=for-the-badge&logo=github-actions&label=CI)](https://github.com/iam4lucard/cheraghtunnel/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
 
<br/>
 
**CheraghTunnel** is an all-in-one, ultra-fast, security-focused reverse tunneling system engineered to bypass severe network censorship and establish stealthy server-to-server connections. It packages a multi-protocol proxy engine, zero-latency handshakes, AI/DPI-evasion transports, automated SSH node deployment, and a modern Glassmorphism Web Panel into a **single static binary with zero external dependencies**.
 
<br/>
 
**`< 15 MB RAM`** &nbsp;•&nbsp; **`< 7 MB Binary`** &nbsp;•&nbsp; **`Zero External Dependencies`** &nbsp;•&nbsp; **`Single Binary`**
 
</div>
 
---

## 📑 Table of Contents

- [Key Features](#-key-features)
- [Transport Protocols](#-transport-protocols)
- [Web Management Panel](#-web-management-panel)
- [Quick Start](#-quick-start)
- [CLI Usage Guide](#-cli-usage-guide)
- [Security & Anti-Censorship Mechanisms](#-security--anti-censorship-mechanisms)
- [Building from Source](#-building-from-source)
- [License](#-license)

---

## ✨ Key Features

* 🚀 **16 Advanced Transport Protocols:** From multiplexed TCP (`Beam`) to Reality TLS (`Mirage`), WebRTC (`Halo`), QUIC (`Pulsar`), and specialized stealth game transports (`Spectre`, `Oracle`, `Vortex`, `Nirvana`).
* ⚡ **Zero-RTT (0-RTT) Handshakes:** Instant security key exchange embedded directly within the first packet, eliminating initial round-trip connection latency in `Spectre`, `Mirage`, `Nirvana`, and `Beam`.
* 🔒 **Dynamic Packet Length Padding:** Injects 0–256 randomized padding bytes into payload frames to dismantle DPI packet-size signatures.
* 📡 **Dummy Chaffing Traffic:** Emits periodic background dummy traffic during connection silences to neutralize ML-based timing analysis.
* 🛡️ **Encrypted ClientHello (ECH) Simulation:** Appends ECH extension (`0xfe0d`) and authentic Chrome 126+ / Firefox 128+ TLS extension order in `Spectre` & `Mirage`.
* 🔀 **Multipath IP Spraying:** Evenly distributes client worker connections across multiple remote server IPs simultaneously.
* 🔄 **Automated Node Health & Live Failover:** 10-second background TCP/RTT health monitor that automatically reroutes active tunnels to backup nodes upon server unreachability.
* 🔀 **Dynamic Port Hopping:** Time-based, cryptographically synchronized control port rotation (every 5 mins) to defeat active port-blocking firewalls.
* ⚙️ **One-Click Automated SSH Deployment:** Provision remote nodes via embedded SSH automation (supports password and private key authentication).
* ⏳ **Quota, Speed & Expiry Limits:** Set strict data caps (GB), bandwidth limits (KB/s), and expiration dates per tunnel.
* 📊 **Real-Time WebSocket Telemetry:** Live streaming dashboard for RTT latency, packet loss, bandwidth (Down/Up), and CPU/RAM usage.
* 🛡️ **Decoy Defense System:** Intelligent active probing defense simulating legitimate web domain responses for unauthorized scanner probes.

---

## 🔌 Transport Protocols

| Profile | Technical ID | Transport Layer | Features | Ideal Use Case |
|:---:|:---:|:---:|:---|:---|
| 🔵 **Beam** | `tcpmux` | TCP (0-RTT) | High-speed parallel TCP multiplexing with 0-RTT auth | General high-throughput |
| 🟢 **Aura** | `httpmux` | HTTP | HTTP/1.1 header and request masquerading | Highly restricted networks |
| 🟡 **Nova** | `httpsmux` | HTTPS | Pure TLS encrypted transport stream | Maximum confidentiality |
| 🟣 **Glimmer** | `wsmux` | WebSocket | Standard WebSocket framing for CDN routing | CDN traversal |
| 🔴 **Beacon** | `wssmux` | WSS | Secure WebSocket with TLS layer (Cloudflare compatible) | High-security CDN |
| ⚡ **Flash** | `kcpmux` | KCP/UDP | Low-latency sliding-window UDP protocol | Online gaming & low ping |
| 🌊 **Ray** | `rawmux` | Raw UDP | Direct KCP socket binding with minimal overhead | Real-time audio/video |
| ⚛️ **Photon** | `quantummux` | TCP+FEC | Hybrid TCP & KCP with Forward Error Correction without UDP | Bypassing UDP blocks |
| 🏮 **Lantern** | `tunmux` | TUN L2/L3 | System-level network interface virtualization | Full system tunneling |
| 🌫️ **Mirage** | `realitymux` | Reality TLS (0-RTT) | Real TLS 1.3 certificate spoofing with zero-RTT handshake | Advanced DPI firewalls |
| 👼 **Halo** | `webrtcmux` | WebRTC | Masquerades packets as P2P voice/video calls | Strict DPI censorship |
| 💫 **Pulsar** | `pulsar` | QUIC/UDP | QUIC-based pulse protocol with adaptive flow control | Lossy & noisy networks |
| 🔮 **Oracle** | `oracle` | DNS/UDP | Valid DNS query/response mimicry with EDNS0 on port 53 | Severe UDP censorship |
| 🌀 **Vortex** | `vortex` | Steam/UDP | Source Engine game query/ping packet mimicry | Gaming with high QoS |
| 🕉️ **Nirvana** | `nirvana` | HTTP/TCP (0-RTT) | Chunked HTTP POST request obfuscation with XOR cipher | High-speed stealth TCP |
| 👻 **Spectre** | `spectre` | Multipath TLS (0-RTT) | Ultra-fast gaming transport with 0-RTT Reality TLS & Multipath Spraying | Competitive gaming with flat ping |

---

## 🎨 Web Management Panel

The built-in Web Management Panel provides a sleek, code-free administration experience:
* **Live Telemetry Dashboard:** Full-duplex WebSocket streaming of CPU, RAM, bandwidth, and latency matrix.
* **Node Management:** Register Iran and Kharej nodes with SSH credentials for automated remote deployment.
* **Tunnel Configuration:** Toggle protocols, ports, Decoy URLs, Dynamic Padding, ECH, Chaffing, and Port Hopping in seconds.
* **Backup & Restore:** One-click SQLite database export and instant web restore without system restarts.

---

## 🚀 Quick Start

### Method 1: Automated Installer (Recommended)

Run the automated interactive installation script as `root` on your main server:

```bash
curl -sSf https://raw.githubusercontent.com/iam4lucard/cheraghtunnel/main/install.sh | bash
```

The script will guide you through:
* Setting the Web Panel port (default: `8000`).
* Creating administrative username and password credentials.
* Registering `cheraghtunnel` as a background `systemd` daemon.

### Method 2: Direct Binary Download

Download pre-compiled static binaries directly from GitHub releases:

```bash
# Linux (amd64)
curl -sSfL -o /usr/local/bin/cheraghtunnel \
  https://github.com/iam4lucard/cheraghtunnel/releases/latest/download/cheraghtunnel-linux-amd64
chmod +x /usr/local/bin/cheraghtunnel

# Linux (arm64)
curl -sSfL -o /usr/local/bin/cheraghtunnel \
  https://github.com/iam4lucard/cheraghtunnel/releases/latest/download/cheraghtunnel-linux-arm64
chmod +x /usr/local/bin/cheraghtunnel
```

---

## 💻 CLI Usage Guide

If you prefer running CheraghTunnel core manually via command line:

### 1. Launch Web Panel

```bash
cheraghtunnel panel --port 8000 --db-path /var/lib/cheraghtunnel/cheraghtunnel.db
```

### 2. Launch Server (Iran Node)

```bash
cheraghtunnel server \
  --control-port 8090 \
  --public-port 443 \
  --token SECRET_TOKEN \
  --protocol spectre \
  --decoy https://www.microsoft.com \
  --port-hopping
```

### 3. Launch Client (Kharej Node)

```bash
cheraghtunnel client \
  --server-ip 62.60.202.4 \
  --control-port 8090 \
  --public-port 443 \
  --local-service 127.0.0.1:1080 \
  --token SECRET_TOKEN \
  --protocol spectre \
  --tunnel-id 1 \
  --port-hopping
```

---

## 🔒 Security & Anti-Censorship Mechanisms

CheraghTunnel incorporates defense-in-depth security principles:
* **Constant-Time Operations:** Protects token verification against timing side-channel attacks.
* **Active Probing Defense (Decoy):** Unauthorized probes sent to the control port are greeted with simulated decoy web responses or redirects to target domains.
* **Brute-Force Protection:** Rate limits failed login attempts to safeguard the administrative panel.
* **Instant Port Release:** Employs socket options (`SO_REUSEADDR` & `SO_REUSEPORT`) for immediate socket re-binding without TIME_WAIT port lockups.

---

## 🛠 Building from Source

### Prerequisites
* [Rust & Cargo](https://rustup.rs/) 1.75 or later
* SQLite development headers (`libsqlite3-dev` on Debian/Ubuntu)

### Build Steps

```bash
# Clone the repository
git clone https://github.com/iam4lucard/cheraghtunnel.git
cd cheraghtunnel

# Build release binary
cargo build --release

# Run executable
./target/release/cheraghtunnel panel --port 8000
```

---

## 📜 License

CheraghTunnel is open-source software licensed under the **[MIT License](LICENSE)**.

<div align="center">

**Built with ❤️ and the power of Rust**

[🐛 Report Issues](https://github.com/iam4lucard/cheraghtunnel/issues) &nbsp;•&nbsp; [💡 Feature Requests](https://github.com/iam4lucard/cheraghtunnel/issues) &nbsp;•&nbsp; [📦 Releases](https://github.com/iam4lucard/cheraghtunnel/releases)

</div>
