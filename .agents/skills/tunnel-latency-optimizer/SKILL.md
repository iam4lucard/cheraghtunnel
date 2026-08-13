---
name: tunnel-latency-optimizer
description: Comprehensive ultra-low latency, throughput, and stability optimization guide and automated scripts for Linux servers, network tunnels, proxies (Xray/V2Ray/Sing-box), and cross-border communication. Use when optimizing server ping, reducing jitter, eliminating packet drops, tuning TCP/BBR stacks, configuring Unbound recursive DNS, or deploying low-latency tunnel engines.
---

# Tunnel & Server Latency Optimizer

A complete, production-grade guide and execution handbook for minimizing latency, eliminating packet loss, and maximizing throughput across Linux proxy servers, cross-border tunnels (e.g. Iran-Europe), and networking infrastructure.

---

## 🎯 The 5-Layer Latency Optimization Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 1: Tunnel Engine (64KB Copy Buffers, 8+ Pipes, MTU)  │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: Socket Layer (TCP_QUICKACK, TCP_FASTOPEN, LOWAT)  │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: Kernel TCP Stack (BBR, FQ, Thin-Stream, No-Cork)  │
├─────────────────────────────────────────────────────────────┤
│  Layer 4: Hardware & NIC (TSO/GSO Offloading, 10K Queues)   │
├─────────────────────────────────────────────────────────────┤
│  Layer 5: Local Recursive DNS (Unbound <1ms RAM Resolver)   │
└─────────────────────────────────────────────────────────────┘
```

---

## Layer 1: Tunnel & Application Engine Optimizations

When building or configuring high-performance proxies/tunnels:

### 1. 64KB Bidirectional Copy Buffers
- **Problem**: Default runtime copy buffers (e.g. Tokio default 8KB) cause excessive system call interrupts on high-bandwidth transfers.
- **Fix**: Use `tokio::io::copy_bidirectional_with_sizes(&mut s1, &mut s2, 65536, 65536)`.
- **Result**: 8x reduction in kernel context switches and CPU interrupts.

### 2. Multi-Worker Multiplexing (8+ Parallel Pipes)
- **Problem**: Single TCP multiplexer streams suffer from Head-of-Line (HoL) blocking when a heavy download delays gaming or voice packets.
- **Fix**: Run a pool of 8 to 12 parallel multiplexed TCP channels per tunnel.

### 3. Safe Path MTU Clamping
- **Problem**: IP fragmentation by intermediate international transit routers adds latency and retransmissions.
- **Fix**: Clamp transport payload MTU to **1350–1380 bytes**.

### 4. Static Tunnel Ports over Aggressive Port Hopping
- **Problem**: Periodic port-hopping shifts (e.g. every 5 minutes) force Yamux session teardowns and recreate latency spikes.
- **Fix**: Keep persistent, multiplexed sessions on static ports for 100% continuous uptime.

---

## Layer 2: Socket-Level System Calls (`setsockopt`)

Ensure the following socket options are enabled in code on TCP sockets:

```rust
// 1. TCP_NODELAY: Disable Nagle's algorithm for instant packet dispatch
socket.set_nodelay(true)?;

// 2. TCP_QUICKACK: Suppress 40ms Linux kernel delayed-ACK timer
let quickack: libc::c_int = 1;
libc::setsockopt(fd, libc::IPPROTO_TCP, libc::TCP_QUICKACK, &quickack as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as libc::socklen_t);

// 3. TCP_NOTSENT_LOWAT: Prevent bufferbloat by limiting unsent byte buffer to 16KB
let notsent: libc::c_uint = 16384;
libc::setsockopt(fd, libc::IPPROTO_TCP, libc::TCP_NOTSENT_LOWAT, &notsent as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_uint>() as libc::socklen_t);

// 4. TCP_FASTOPEN_CONNECT: Send payload directly in SYN packet (saves 1 full RTT / 70-100ms)
let fastopen: libc::c_int = 1;
libc::setsockopt(fd, libc::IPPROTO_TCP, libc::TCP_FASTOPEN_CONNECT, &fastopen as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as libc::socklen_t);
```

---

## Layer 3: Kernel TCP Stack Tuning

Deploy this permanent sysctl configuration file to `/etc/sysctl.d/99-cheragh-lowlatency.conf`:

```ini
# /etc/sysctl.d/99-cheragh-lowlatency.conf

# 1. Congestion Control & Queue Discipline
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr

# 2. Thin-Stream Retransmissions (Instant retransmission for interactive/gaming packets)
net.ipv4.tcp_thin_linear_timeouts = 1

# 3. Disable Delayed Packet Dispatch (Immediate transmission)
net.ipv4.tcp_autocorking = 0

# 4. TCP Fast Open for Client & Server
net.ipv4.tcp_fastopen = 3

# 5. Prevent CWND Collapse on Idle Connection Gaps
net.ipv4.tcp_slow_start_after_idle = 0

# 6. TCP Timestamps and Selective ACKs
net.ipv4.tcp_timestamps = 1
net.ipv4.tcp_sack = 1

# 7. Fast Failover on Dead Handshakes
net.ipv4.tcp_syn_retries = 3
net.ipv4.tcp_synack_retries = 3

# 8. Aggressive Dead Socket Cleanup (90s instead of default 7200s)
net.ipv4.tcp_keepalive_time = 60
net.ipv4.tcp_keepalive_intvl = 10
net.ipv4.tcp_keepalive_probes = 3

# 9. Golden Memory Buffer Windows for Cross-Border BDP
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216

# 10. Queue & Backlog Limits to Eliminate Dropped Packets
net.core.netdev_max_backlog = 65535
net.core.somaxconn = 65535

# 11. Low Swappiness (Protect RAM from slow disk swapping)
vm.swappiness = 10
```

Apply immediately:
```bash
sysctl --system
```

---

## Layer 4: Hardware NIC & CPU Optimization

Run the following commands to tune the network interface and processor:

```bash
# 1. Expand Transmit Queue to 10,000 packets
IFACE=$(ip route show default | awk '{print $5}' | head -n 1)
if [ -n "$IFACE" ]; then
    ip link set dev "$IFACE" txqueuelen 10000 2>/dev/null || true
    
    # 2. Enable Hardware Segmentation Offloading (TSO/GSO/GRO)
    ethtool -K "$IFACE" tso on gso on gro on 2>/dev/null || true
fi

# 3. Lock CPU Governor into Performance Mode (Zero C-state wakeup delay)
for gov in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
    if [ -f "$gov" ]; then echo performance > "$gov" 2>/dev/null || true; fi
done
```

---

## Layer 5: High-Performance Unbound Recursive DNS

Replace bloated DNS services (e.g. Pi-hole consuming 500MB+ RAM/swap) with lightweight **Unbound** (< 8MB RAM, < 1ms resolution).

### 1. Purge Old Bloatware
```bash
systemctl stop pihole-FTL 2>/dev/null || true
systemctl disable pihole-FTL 2>/dev/null || true
apt-get purge -y pihole-FTL lighttpd 2>/dev/null || true
rm -rf /etc/pihole /etc/dnsmasq.d/01-pihole.conf 2>/dev/null || true
```

### 2. Disable systemd-resolved Conflict on Port 53
```bash
if [ -f /etc/systemd/resolved.conf ]; then
    sed -i 's/#DNSStubListener=yes/DNSStubListener=no/' /etc/systemd/resolved.conf
    sed -i 's/DNSStubListener=yes/DNSStubListener=no/' /etc/systemd/resolved.conf
    systemctl restart systemd-resolved 2>/dev/null || true
fi
```

### 3. Install & Configure Unbound
```bash
apt-get update -y && apt-get install -y unbound dnsutils

cat << 'EOF' > /etc/unbound/unbound.conf
server:
    verbosity: 1
    interface: 127.0.0.1
    interface: ::1
    port: 53
    do-ip4: yes
    do-ip6: yes
    do-udp: yes
    do-tcp: yes

    access-control: 127.0.0.0/8 allow
    access-control: ::1 allow

    num-threads: 1
    msg-cache-slabs: 2
    rrset-cache-slabs: 2
    infra-cache-slabs: 2
    key-cache-slabs: 2

    rrset-cache-size: 16m
    msg-cache-size: 8m

    so-rcvbuf: 4m
    so-sndbuf: 4m
    so-reuseport: yes

    hide-identity: yes
    hide-version: yes
    harden-glue: yes
    harden-dnssec-stripped: yes
    use-caps-for-id: no
    edns-buffer-size: 1232
    prefetch: yes
    prefetch-key: yes

forward-zone:
    name: "."
    forward-addr: 1.1.1.1
    forward-addr: 1.0.0.1
    forward-addr: 8.8.8.8
    forward-addr: 8.8.4.4
EOF

systemctl restart unbound
systemctl enable unbound

# Point /etc/resolv.conf to Unbound
chattr -i /etc/resolv.conf 2>/dev/null || true
rm -f /etc/resolv.conf
echo -e "nameserver 127.0.0.1\nnameserver 1.1.1.1" > /etc/resolv.conf
```

---

## ⚡ Automated 1-Click Server Optimization Script

To optimize any new Linux server in one shot, execute:

```bash
# One-liner to optimize any Linux server
curl -sSfL https://raw.githubusercontent.com/iam4lucard/cheraghtunnel/main/scripts/optimize-server.sh | bash
```

---

## ✅ Verification Checklist

Run these diagnostic commands to ensure 100% compliance:

```bash
# 1. Check BBR & FQ
sysctl net.ipv4.tcp_congestion_control net.core.default_qdisc

# 2. Check Thin-Stream & AutoCorking
sysctl net.ipv4.tcp_thin_linear_timeouts net.ipv4.tcp_autocorking net.ipv4.tcp_fastopen

# 3. Check Unbound Status & Port 53
systemctl is-active unbound
ss -lntup | grep ':53 '

# 4. Check DNS Latency (< 1ms)
dig @127.0.0.1 google.com +short

# 5. Check Interface Queue Length (10000)
ip link show $(ip route show default | awk '{print $5}' | head -n 1) | grep txqueuelen
```
