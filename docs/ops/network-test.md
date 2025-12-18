## Network testing (safe + bounded)

This runbook is for **legitimate testing between systems you control** (e.g., your laptop ↔ your own server). It focuses on **bounded** checks: latency, loss, path quality, and throughput with **hard caps** and **short durations**.

### Safety rules (do these every time)

- **Only test hosts you own/operate** (or have explicit written permission to test).
- **Prefer low durations** (10–30s) and **conservative caps** (e.g., 10–100 Mbps) first.
- **Never use many parallel streams** unless you’re debugging a known issue.
- **Stop immediately** if you see instability (timeouts, high loss, CPU pegged, services affected).

---

## 1) Basic reachability + latency (Windows)

Replace `<SERVER_IP_OR_NAME>` with your server.

```powershell
ping -n 20 <SERVER_IP_OR_NAME>
```

If you want hop-by-hop info:

```powershell
tracert <SERVER_IP_OR_NAME>
```

If you want a combined latency + loss view (slower, but useful):

```powershell
pathping <SERVER_IP_OR_NAME>
```

---

## 2) Throughput test (bounded) with iperf3

`iperf3` is the simplest way to test raw network throughput **without** involving your app stack.

### 2.1 Install iperf3

#### Windows (PowerShell)

If you have winget:

```powershell
winget install -e --id ESnet.iperf3
```

Verify:

```powershell
iperf3 --version
```

#### Ubuntu server

```bash
sudo apt-get update
sudo apt-get install -y iperf3
```

### 2.2 Start iperf3 server (Ubuntu)

This listens on TCP/5201 by default.

```bash
sudo iperf3 -s
```

If you use a firewall, ensure **TCP 5201** is allowed from your laptop’s public IP only.

### 2.3 Run capped throughput tests (Windows → Server)

Start with a short run, one stream:

```powershell
iperf3 -c <SERVER_IP_OR_NAME> -t 15 -P 1
```

If you want to **cap** the test rate (UDP mode), pick a conservative value first:

```powershell
iperf3 -c <SERVER_IP_OR_NAME> -u -b 50M -t 15
```

Notes:
- `-u` uses UDP (gives loss/jitter). The `-b` flag is a **hard send cap** you control.
- If loss is high, reduce `-b` (e.g., `10M`, `20M`) and retest.

### 2.4 Test download direction (Server → Windows)

This uses TCP reverse mode:

```powershell
iperf3 -c <SERVER_IP_OR_NAME> -R -t 15 -P 1
```

---

## 3) Quick “is the server struggling?” checks (Ubuntu)

In another SSH session while you run tests:

```bash
uptime
free -h
top -o %CPU
```

If you have `sysstat`:

```bash
sudo apt-get install -y sysstat
sar -u 1 30
sar -n DEV 1 30
```

---

## 4) What to record (so the results are actionable)

- **When** (timestamp) and **where** (client ISP/network, server DC/region)
- `ping` avg/min/max and loss %
- `iperf3`:
  - TCP: sender/receiver Mbps and retransmits
  - UDP: Mbps, loss %, jitter
- Any server CPU/memory spikes during the test

---

## 5) Common pitfalls

- **Testing the app** instead of the network: use `iperf3` first to establish baseline.
- **Too aggressive caps**: jumping straight to high rates can create misleading loss/latency and impact services.
- **Multiple parallel streams** (`-P`): can hide single-flow problems; keep `-P 1` until you need it.


