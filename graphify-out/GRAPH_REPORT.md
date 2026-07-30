# Graph Report - .  (2026-07-28)

## Corpus Check
- Corpus is ~34,570 words - fits in a single context window. You may not need a graph.

## Summary
- 448 nodes · 1196 edges · 23 communities (19 shown, 4 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 11 edges (avg confidence: 0.75)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Transport Protocols & TLS State
- Multiplexing & Async Channels
- Web Panel API Handlers & Axum Router
- Nirvana & Custom Stream Transport
- SQLite Database & Data Models
- Multiplexer Traffic Tracking
- Frontend UI App Logic & Modals
- KCP & Low Latency Transport
- Tunnel Server Control Loop
- Network Socket Optimization
- Transport Protocol Unit Tests
- TLS Certificate Verification
- Traffic Obfuscation & Padding
- CLI Command Line Runner
- Network Telemetry & RTT Probing
- Crypto & Auth Headers
- Installation Installer Script
- English Documentation
- Persian Documentation
- HTML Dashboard Structure

## God Nodes (most connected - your core abstractions)
1. `UdpVirtualStreamInner` - 29 edges
2. `AppState` - 25 edges
3. `get_db_conn()` - 23 edges
4. `server_handshake()` - 23 edges
5. `client_handshake()` - 20 edges
6. `run_handshake_test()` - 20 edges
7. `TransportStream` - 19 edges
8. `UdpVirtualStream` - 19 edges
9. `run_server()` - 15 edges
10. `run_client()` - 14 edges

## Surprising Connections (you probably didn't know these)
- `spawn_protocol_listener()` --calls--> `server_handshake()`  [INFERRED]
  src/tunnel/mod.rs → src/tunnel/transport/mod.rs
- `run_server()` --calls--> `pipe_streams_monitored()`  [INFERRED]
  src/tunnel/mod.rs → src/tunnel/multiplex.rs
- `run_client()` --calls--> `connect_to_local()`  [INFERRED]
  src/tunnel/mod.rs → src/tunnel/multiplex.rs
- `run_client()` --calls--> `pipe_streams_monitored()`  [INFERRED]
  src/tunnel/mod.rs → src/tunnel/multiplex.rs
- `run_client()` --calls--> `client_handshake()`  [INFERRED]
  src/tunnel/mod.rs → src/tunnel/transport/mod.rs

## Import Cycles
- None detected.

## Communities (23 total, 4 thin omitted)

### Community 0 - "Transport Protocols & TLS State"
Cohesion: 0.08
Nodes (56): AtomicBool, ClientConfig, ClientTlsStream, ErrorResponse, FnOnce, Send, ServerConfig, ServerTlsStream (+48 more)

### Community 1 - "Multiplexing & Async Channels"
Cohesion: 0.09
Nodes (33): BTreeMap, HashMap, Receiver, FecDecoder, FecEncoder, Arc, AsyncRead, AsyncWrite (+25 more)

### Community 2 - "Web Panel API Handlers & Axum Router"
Cohesion: 0.10
Nodes (55): Extension, HeaderMap, IntoResponse, Json, Multipart, Next, AppState, Assets (+47 more)

### Community 3 - "Nirvana & Custom Stream Transport"
Cohesion: 0.23
Nodes (12): NirvanaStream<S>, ObfuscatedStream<S>, AsyncRead, AsyncWrite, Context, Pin, Poll, ReadBuf (+4 more)

### Community 4 - "SQLite Database & Data Models"
Cohesion: 0.21
Nodes (32): Connection, constant_time_eq(), create_node(), create_tunnel(), delete_node(), delete_tunnel(), get_db_conn(), get_node_by_id() (+24 more)

### Community 5 - "Multiplexer Traffic Tracking"
Cohesion: 0.14
Nodes (25): S1, S2, connect_to_local(), get_traffic_tracker(), MonitoredStream, MonitoredStream<S>, pipe_streams(), pipe_streams_monitored() (+17 more)

### Community 6 - "Frontend UI App Logic & Modals"
Cohesion: 0.16
Nodes (27): clearSessionToken(), DECOY_PROTOCOLS, deleteNode(), deleteTunnel(), downloadBackup(), escapeHtml(), formatBytes(), generateToken() (+19 more)

### Community 7 - "KCP & Low Latency Transport"
Cohesion: 0.15
Nodes (16): Ipv4Addr, KcpConfig, KcpListener, SocketAddrV4, apply_iptables_drop(), craft_tcp_ip_packet(), FakeTcpClient, FakeTcpServer (+8 more)

### Community 8 - "Tunnel Server Control Loop"
Cohesion: 0.19
Nodes (20): Control, get_hopped_port(), get_udp_mode(), is_faketcp_protocol(), is_udp_protocol(), LoopGuard, Arc, Box (+12 more)

### Community 9 - "Network Socket Optimization"
Cohesion: 0.13
Nodes (15): AtomicUsize, bind_listener(), enable_ebpf_fastpath(), optimize_socket(), Result, SocketAddr, TcpStream, set_tcp_mss_clamp() (+7 more)

### Community 10 - "Transport Protocol Unit Tests"
Cohesion: 0.20
Nodes (18): get_free_port(), run_handshake_test(), test_protocol_aura(), test_protocol_beacon(), test_protocol_beam(), test_protocol_flash(), test_protocol_glimmer(), test_protocol_halo() (+10 more)

### Community 11 - "TLS Certificate Verification"
Cohesion: 0.21
Nodes (9): CertificateDer, DigitallySignedStruct, HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier, ServerName, SignatureScheme, NoCertificateVerification (+1 more)

### Community 12 - "Traffic Obfuscation & Padding"
Cohesion: 0.29
Nodes (6): Sleep, add_padding(), remove_padding(), Result, String, Vec

### Community 13 - "CLI Command Line Runner"
Cohesion: 0.38
Nodes (5): Cli, Commands, Option, PathBuf, String

### Community 14 - "Network Telemetry & RTT Probing"
Cohesion: 0.83
Nodes (3): main(), measure_tunnel_rtt(), ping_host()

### Community 15 - "Crypto & Auth Headers"
Cohesion: 0.67
Nodes (3): generate_auth_header(), String, verify_auth_header()

## Knowledge Gaps
- **6 isolated node(s):** `install.sh script`, `Assets`, `SystemStats`, `CheraghTunnel Persian Documentation`, `CheraghTunnel Documentation` (+1 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **4 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `run_server()` connect `Tunnel Server Control Loop` to `Web Panel API Handlers & Axum Router`, `Multiplexer Traffic Tracking`?**
  _High betweenness centrality (0.283) - this node is a cross-community bridge._
- **Why does `spawn_protocol_listener()` connect `Tunnel Server Control Loop` to `Transport Protocols & TLS State`?**
  _High betweenness centrality (0.169) - this node is a cross-community bridge._
- **Why does `TransportStream` connect `Transport Protocols & TLS State` to `Tunnel Server Control Loop`, `Multiplexing & Async Channels`, `Nirvana & Custom Stream Transport`?**
  _High betweenness centrality (0.134) - this node is a cross-community bridge._
- **What connects `install.sh script`, `Assets`, `SystemStats` to the rest of the system?**
  _6 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Transport Protocols & TLS State` be split into smaller, more focused modules?**
  _Cohesion score 0.08499743983614952 - nodes in this community are weakly interconnected._
- **Should `Multiplexing & Async Channels` be split into smaller, more focused modules?**
  _Cohesion score 0.09148598625066102 - nodes in this community are weakly interconnected._
- **Should `Web Panel API Handlers & Axum Router` be split into smaller, more focused modules?**
  _Cohesion score 0.1005260081823495 - nodes in this community are weakly interconnected._