#!/usr/bin/env bash
# ==============================================================================
# CheraghTunnel Server & Network Latency Optimizer
# Optimized for Ultra Low-Latency, High-Throughput Cross-Border Tunnels & Proxies
# ==============================================================================

set -e

echo "=========================================================="
echo "⚡ Starting Ultra Low-Latency Server & Network Optimizer"
echo "=========================================================="

# 1. Purge Pi-hole if present
echo "[1/5] Checking and purging Pi-hole/bloatware..."
systemctl stop pihole-FTL 2>/dev/null || true
systemctl disable pihole-FTL 2>/dev/null || true
apt-get purge -y pihole-FTL lighttpd 2>/dev/null || true
rm -rf /etc/pihole /etc/dnsmasq.d/01-pihole.conf 2>/dev/null || true

# 2. Configure Unbound Lightweight Recursive DNS
echo "[2/5] Installing and configuring Unbound DNS Resolver..."
if [ -f /etc/systemd/resolved.conf ]; then
    sed -i 's/#DNSStubListener=yes/DNSStubListener=no/' /etc/systemd/resolved.conf
    sed -i 's/DNSStubListener=yes/DNSStubListener=no/' /etc/systemd/resolved.conf
    systemctl restart systemd-resolved 2>/dev/null || true
fi

apt-get update -y && apt-get install -y unbound dnsutils curl ethtool

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

chattr -i /etc/resolv.conf 2>/dev/null || true
rm -f /etc/resolv.conf
echo -e "nameserver 127.0.0.1\nnameserver 1.1.1.1" > /etc/resolv.conf

# 3. Kernel TCP Low-Latency Stack
echo "[3/5] Applying Kernel TCP Low-Latency Suite (BBR, FQ, Thin-Stream, FastOpen)..."
cat << 'EOF' > /etc/sysctl.d/99-cheragh-lowlatency.conf
net.ipv4.tcp_thin_linear_timeouts = 1
net.ipv4.tcp_slow_start_after_idle = 0
net.ipv4.tcp_autocorking = 0
net.ipv4.tcp_fastopen = 3
net.ipv4.tcp_timestamps = 1
net.ipv4.tcp_sack = 1
net.ipv4.tcp_syn_retries = 3
net.ipv4.tcp_synack_retries = 3
net.ipv4.tcp_keepalive_time = 60
net.ipv4.tcp_keepalive_intvl = 10
net.ipv4.tcp_keepalive_probes = 3
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.core.netdev_max_backlog = 65535
net.core.somaxconn = 65535
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr
vm.swappiness = 10
EOF

sysctl --system >/dev/null 2>&1 || true

# 4. Hardware NIC & CPU Frequency Optimization
echo "[4/5] Tuning Hardware NIC Ring Buffers, Offloading and CPU Scaling..."
IFACE=$(ip route show default 2>/dev/null | awk '{print $5}' | head -n 1)
if [ -n "$IFACE" ]; then
    ip link set dev "$IFACE" txqueuelen 10000 2>/dev/null || true
    ethtool -K "$IFACE" tso on gso on gro on 2>/dev/null || true
fi

for gov in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
    if [ -f "$gov" ]; then echo performance > "$gov" 2>/dev/null || true; fi
done

# 5. Summary and Verification
echo "[5/5] Running Verification Tests..."
echo "----------------------------------------------------------"
echo -n "BBR Congestion Control : " && sysctl -n net.ipv4.tcp_congestion_control
echo -n "TCP Fast Open Level    : " && sysctl -n net.ipv4.tcp_fastopen
echo -n "TCP AutoCorking        : " && sysctl -n net.ipv4.tcp_autocorking
echo -n "Thin-Stream Mode       : " && sysctl -n net.ipv4.tcp_thin_linear_timeouts
echo -n "Unbound Status         : " && systemctl is-active unbound
echo -n "DNS Query Test         : " && dig @127.0.0.1 google.com +short | head -n 1
echo "----------------------------------------------------------"
echo "✅ Server successfully optimized for Ultra Low Latency!"
