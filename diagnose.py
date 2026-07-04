#!/usr/bin/env python3
import sys
import os
import subprocess
import time
import socket
import json
import statistics
import argparse
from datetime import datetime

def ping_host(host, count=30):
    print(f"[*] Pinging {host} {count} times to measure raw network latency...")
    pings = []
    lost = 0
    
    param = '-n' if sys.platform.lower() == 'windows' else '-c'
    timeout_param = '-w' if sys.platform.lower() == 'windows' else '-W'
    timeout_val = '1000' if sys.platform.lower() == 'windows' else '1'
    
    for i in range(count):
        start = time.time()
        try:
            cmd = ['ping', param, '1', timeout_param, timeout_val, host]
            result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            duration = (time.time() - start) * 1000  # ms
            
            if result.returncode == 0:
                rtt = duration
                for line in result.stdout.split('\n'):
                    if 'time=' in line:
                        try:
                            parts = line.split('time=')[1].split()[0]
                            parts = parts.replace('ms', '')
                            rtt = float(parts)
                        except:
                            pass
                pings.append(rtt)
            else:
                lost += 1
        except Exception as e:
            lost += 1
        time.sleep(0.03)
        
    loss_rate = (lost / count) * 100
    if pings:
        avg_ping = statistics.mean(pings)
        min_ping = min(pings)
        max_ping = max(pings)
        jitter = statistics.stdev(pings) if len(pings) > 1 else 0
    else:
        avg_ping, min_ping, max_ping, jitter = 0, 0, 0, 0
        
    return {
        "loss_rate": loss_rate,
        "avg": avg_ping,
        "min": min_ping,
        "max": max_ping,
        "jitter": jitter,
        "pings": pings
    }

def measure_tunnel_rtt(port, count=15):
    print(f"[*] Measuring end-to-end RTT through tunnel on port {port} ({count} samples)...")
    rtts = []
    failed = 0
    
    for _ in range(count):
        start = time.time()
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(4.0)
            
            # Connect locally (Iran proxy entry point)
            sock.connect(("127.0.0.1", port))
            
            # Send dummy 4-byte payload
            sock.sendall(b"ping")
            
            # Initiate half-close (send FIN). This forces the FIN to travel to Kharej backend.
            sock.shutdown(socket.SHUT_WR)
            
            # Block until Kharej backend closes connection (returns EOF or RST)
            try:
                _ = sock.recv(1024)
            except socket.error:
                # Connection reset is also a valid RTT signal
                pass
                
            duration = (time.time() - start) * 1000  # ms
            rtts.append(duration)
            sock.close()
        except Exception as e:
            failed += 1
        time.sleep(0.05)
        
    fail_rate = (failed / count) * 100
    if rtts:
        avg_rtt = statistics.mean(rtts)
        min_rtt = min(rtts)
        max_rtt = max(rtts)
        jitter = statistics.stdev(rtts) if len(rtts) > 1 else 0
    else:
        avg_rtt, min_rtt, max_rtt, jitter = 0, 0, 0, 0
        
    return {
        "port": port,
        "fail_rate": fail_rate,
        "avg": avg_rtt,
        "min": min_rtt,
        "max": max_rtt,
        "jitter": jitter,
        "samples": rtts
    }

def main():
    parser = argparse.ArgumentParser(description="CheraghTunnel Diagnostics")
    parser.add_argument("--tunnel-port", type=int, help="The public port of the tunnel on Iran server to test end-to-end")
    parser.add_argument("--compare", nargs=2, type=int, metavar=('PORT1', 'PORT2'), help="Compare the quality of two active tunnels on different ports")
    args = parser.parse_args()
    
    print("==================================================")
    print("      CheraghTunnel Diagnostics & Quality Test")
    print("==================================================")
    
    iran_ip = "62.60.202.4"
    kharej_ip = "91.107.181.217"
    
    # Run comparison mode if requested
    if args.compare:
        port1, port2 = args.compare
        print(f"[*] Starting Comparison between Tunnel Port {port1} and Tunnel Port {port2}...")
        
        # Test raw ping to Kharej first
        raw_ping = ping_host(kharej_ip, count=20)
        
        # Test Tunnel 1 RTT
        t1_res = measure_tunnel_rtt(port1)
        
        # Test Tunnel 2 RTT
        t2_res = measure_tunnel_rtt(port2)
        
        report_filename = "tunnel_comparison.md"
        
        report = f"""# Tunnel Quality Comparison Report
Generated on: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}

## 1. Raw International Route Quality (Iran -> Germany)
This is the baseline ping of your raw internet routing without a tunnel:
* **Avg Raw Latency:** `{raw_ping['avg']:.1f}ms`
* **Raw Route Jitter:** `{raw_ping['jitter']:.1f}ms`
* **Raw Packet Loss:** `{raw_ping['loss_rate']:.1f}%`

---

## 2. End-to-End Tunnel Performance Comparison
This measures connection latency and packet loss **through the tunnels** from the Iran server to the Kharej backend.

| Metric | Tunnel Port {port1} | Tunnel Port {port2} | Winner |
| :--- | :---: | :---: | :---: |
| **Connection Success Rate** | {100 - t1_res['fail_rate']:.1f}% | {100 - t2_res['fail_rate']:.1f}% | {"Draw" if t1_res['fail_rate'] == t2_res['fail_rate'] else f"Port {port1}" if t1_res['fail_rate'] < t2_res['fail_rate'] else f"Port {port2}"} |
| **Minimum Tunnel RTT** | {t1_res['min']:.1f}ms | {t2_res['min']:.1f}ms | {f"Port {port1}" if t1_res['min'] < t2_res['min'] else f"Port {port2}"} |
| **Average Tunnel RTT** | {t1_res['avg']:.1f}ms | {t2_res['avg']:.1f}ms | {f"Port {port1}" if t1_res['avg'] < t2_res['avg'] else f"Port {port2}"} |
| **Maximum Tunnel RTT** | {t1_res['max']:.1f}ms | {t2_res['max']:.1f}ms | {f"Port {port1}" if t1_res['max'] < t2_res['max'] else f"Port {port2}"} |
| **Jitter (Latency Fluctuation)** | {t1_res['jitter']:.1f}ms | {t2_res['jitter']:.1f}ms | {f"Port {port1}" if t1_res['jitter'] < t2_res['jitter'] else f"Port {port2}"} |

---

## 3. Latency Samples (ms)
Check the individual samples to see if there are sudden spikes or lag:

* **Tunnel Port {port1} Samples:**
  `{", ".join([f"{p:.1f}" for p in t1_res['samples']])}`
  
* **Tunnel Port {port2} Samples:**
  `{", ".join([f"{p:.1f}" for p in t2_res['samples']])}`

---

## 4. Diagnostics & Verdict
"""
        # Formulate recommendations/verdict
        recommendations = []
        if t1_res['fail_rate'] > t2_res['fail_rate']:
            recommendations.append(f"- **Port {port2} is more stable:** Port {port1} experienced connection drops ({t1_res['fail_rate']:.1f}%) through the tunnel.")
        elif t2_res['fail_rate'] > t1_res['fail_rate']:
            recommendations.append(f"- **Port {port1} is more stable:** Port {port2} experienced connection drops ({t2_res['fail_rate']:.1f}%) through the tunnel.")
            
        if t1_res['avg'] < t2_res['avg'] - 10:
            recommendations.append(f"- **Port {port1} is faster:** Port {port1} has lower average round-trip latency by {t2_res['avg'] - t1_res['avg']:.1f}ms.")
        elif t2_res['avg'] < t1_res['avg'] - 10:
            recommendations.append(f"- **Port {port2} is faster:** Port {port2} has lower average round-trip latency by {t1_res['avg'] - t2_res['avg']:.1f}ms.")
            
        if t1_res['jitter'] < t2_res['jitter'] - 5:
            recommendations.append(f"- **Port {port1} is more stable for gaming:** Port {port1} has lower latency fluctuation (jitter) by {t2_res['jitter'] - t1_res['jitter']:.1f}ms.")
        elif t2_res['jitter'] < t1_res['jitter'] - 5:
            recommendations.append(f"- **Port {port2} is more stable for gaming:** Port {port2} has lower latency fluctuation (jitter) by {t1_res['jitter'] - t2_res['jitter']:.1f}ms.")
            
        if not recommendations:
            recommendations.append("- **Both tunnels perform similarly:** Latency and stability are neck-and-neck on both ports.")
            
        report += "\n".join(recommendations)
        report += "\n"
        
        with open(report_filename, "w") as f:
            f.write(report)
            
        print(f"\n[+] Comparison completed! Report written to: {report_filename}")
        print("==================================================")
        return

    # Basic single-port test mode
    if args.tunnel_port:
        raw_ping = ping_host(kharej_ip, count=20)
        t_res = measure_tunnel_rtt(args.tunnel_port)
        
        report_filename = "tunnel_diagnostics.md"
        report = f"""# CheraghTunnel Diagnostic Report
Generated on: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}

## 1. Raw International Route Quality (Iran -> Germany)
This is the baseline ping of your raw internet routing without a tunnel:
* **Avg Raw Latency:** `{raw_ping['avg']:.1f}ms`
* **Raw Route Jitter:** `{raw_ping['jitter']:.1f}ms`
* **Raw Packet Loss:** `{raw_ping['loss_rate']:.1f}%`

## 2. End-to-End Tunnel Performance (Port {args.tunnel_port})
This measures connection quality **through** the active CheraghTunnel to the Kharej backend.

* **Target Local Port:** `{args.tunnel_port}` (routes to Kharej backend)
* **Tunnel Connection Loss Rate:** `{t_res['fail_rate']:.1f}%`
* **Avg Tunnel Connection Latency:** `{t_res['avg']:.1f}ms`
* **Tunnel Jitter (Latency Fluctuation):** `{t_res['jitter']:.1f}ms`
* **Tunnel Status:** {"🟢 Stable Connection" if t_res['fail_rate'] == 0 else "🟡 Packet Drops / Instability" if t_res['fail_rate'] < 50 else "🔴 Broken / Closed Tunnel"}

* **Individual Tunnel Latency Samples (ms):**
  `{", ".join([f"{p:.1f}" for p in t_res['samples']])}`
"""
        with open(report_filename, "w") as f:
            f.write(report)
        print(f"\n[+] Diagnostics completed! Report written to: {report_filename}")
        print("==================================================")

if __name__ == "__main__":
    main()
