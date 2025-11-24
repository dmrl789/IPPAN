# IPPAN Testnet Operator Runbook

**Version:** 1.0.0-rc1  
**Network:** IPPAN Public Testnet  
**Last Updated:** 2025-11-24

---

## Overview

This runbook provides operational procedures for running an IPPAN testnet validator or RPC node.

**Testnet Characteristics:**
- **Network ID:** `ippan-testnet-v1`
- **Genesis Time:** 2025-12-01 00:00:00 UTC
- **Round Duration:** 200ms (target)
- **Finality:** 2 rounds (~400ms)
- **Minimum Bond:** 10 IPN (10,000,000,000 µIPN)

---

## Prerequisites

### Hardware Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **CPU** | 2 cores | 4+ cores |
| **RAM** | 4 GB | 8+ GB |
| **Disk** | 50 GB SSD | 100+ GB NVMe |
| **Network** | 10 Mbps | 100+ Mbps |

### Software Requirements

- **OS:** Linux (Ubuntu 22.04+, Debian 11+, or RHEL 8+)
- **Rust:** 1.78+ (for building from source)
- **Docker:** 24.0+ (for containerized deployment)
- **Ports:**
  - `8080/tcp` - RPC endpoint
  - `9615/tcp` - Metrics endpoint (Prometheus)
  - `9000/tcp` - P2P gossip
  - `9001/tcp` - DHT (optional)

---

## Installation

### Option 1: Pre-built Binary

```bash
# Download latest testnet release
wget https://github.com/dmrl789/IPPAN/releases/download/v1.0.0-rc1/ippan-v1.0.0-rc1-linux-x86_64.tar.gz

# Extract
tar -xzf ippan-v1.0.0-rc1-linux-x86_64.tar.gz

# Move to system path
sudo mv ippan-node /usr/local/bin/
sudo chmod +x /usr/local/bin/ippan-node

# Verify
ippan-node --version
```

### Option 2: Build from Source

```bash
# Clone repository
git clone https://github.com/dmrl789/IPPAN.git
cd IPPAN

# Checkout testnet tag
git checkout v1.0.0-rc1

# Build
cargo build --release -p ippan-node

# Install
sudo cp target/release/ippan-node /usr/local/bin/
```

### Option 3: Docker

```bash
# Pull image
docker pull ippan/node:v1.0.0-rc1

# Run
docker run -d \
  --name ippan-testnet \
  -p 8080:8080 \
  -p 9615:9615 \
  -p 9000:9000 \
  -v /var/lib/ippan:/data \
  ippan/node:v1.0.0-rc1 \
  --config /etc/ippan/testnet.toml
```

---

## Configuration

### Testnet Config File

**Location:** `/etc/ippan/testnet.toml`

```toml
[network]
network_id = "ippan-testnet-v1"
chain_id = "ippan-testnet"

# Bootstrap nodes
bootstrap_nodes = [
  "/ip4/seed1.testnet.ippan.io/tcp/9000/p2p/12D3KooWRm8J8W...",
  "/ip4/seed2.testnet.ippan.io/tcp/9000/p2p/12D3KooWSd7H9P...",
  "/ip4/seed3.testnet.ippan.io/tcp/9000/p2p/12D3KooWTq5N2K..."
]

[rpc]
bind_address = "0.0.0.0:8080"
enable_cors = true
allowed_origins = ["*"]
max_body_size = 1048576  # 1 MB

[p2p]
listen_address = "/ip4/0.0.0.0/tcp/9000"
max_peers = 50
min_peers = 3

[storage]
data_dir = "/var/lib/ippan/testnet"
backend = "sled"

[consensus]
round_duration_ms = 200
finality_depth = 2

[metrics]
bind_address = "0.0.0.0:9615"
enabled = true

[logging]
level = "info"
format = "json"
output = "/var/log/ippan/testnet.log"
```

### Genesis File

**Location:** `/etc/ippan/testnet-genesis.json`

Download from:
```bash
wget https://raw.githubusercontent.com/dmrl789/IPPAN/master/config/testnet-genesis.json \
  -O /etc/ippan/testnet-genesis.json
```

---

## Starting the Node

### Systemd Service

**File:** `/etc/systemd/system/ippan-node.service`

```ini
[Unit]
Description=IPPAN Testnet Node
After=network.target

[Service]
Type=simple
User=ippan
Group=ippan
WorkingDirectory=/var/lib/ippan
ExecStart=/usr/local/bin/ippan-node \
  --config /etc/ippan/testnet.toml \
  --genesis /etc/ippan/testnet-genesis.json
Restart=on-failure
RestartSec=10s
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

**Start Service:**
```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable on boot
sudo systemctl enable ippan-node

# Start node
sudo systemctl start ippan-node

# Check status
sudo systemctl status ippan-node

# View logs
sudo journalctl -u ippan-node -f
```

---

## Health Checks

### Basic Health Check

```bash
curl http://localhost:8080/health | jq
```

**Expected Output:**
```json
{
  "status": "ok",
  "peer_count": 12,
  "current_round": 12345,
  "finalized_round": 12343,
  "version": "1.0.0-rc1"
}
```

### Detailed Status

```bash
curl http://localhost:8080/status | jq
```

**Expected Output:**
```json
{
  "consensus": {
    "current_round": 12345,
    "finalized_round": 12343,
    "canonical_tip": "0xabc123...",
    "validator_count": 25
  },
  "network": {
    "peer_count": 12,
    "inbound_peers": 5,
    "outbound_peers": 7,
    "bootstrap_connected": true
  },
  "storage": {
    "disk_usage_bytes": 1073741824,
    "block_count": 50000
  }
}
```

### Metrics Endpoint

```bash
curl http://localhost:9615/metrics | grep ippan_
```

**Key Metrics:**
- `ippan_consensus_rounds_total` - Total rounds processed
- `ippan_consensus_finality_latency_ms` - Finality latency
- `ippan_network_peers_total` - Current peer count
- `ippan_mempool_tx_count` - Transactions in mempool
- `ippan_storage_blocks_total` - Total blocks stored

---

## Becoming a Validator

### Step 1: Generate Validator Key

```bash
cargo run -p ippan-keygen -- generate \
  --output /etc/ippan/validator-key.json \
  --name testnet-validator-1
```

### Step 2: Fund Validator Account

```bash
# Request testnet IPN from faucet
curl -X POST https://faucet.testnet.ippan.io/fund \
  -H "Content-Type: application/json" \
  -d '{"address":"YOUR_ADDRESS","amount":10000000000}'
```

**Alternative:** Join Discord and request testnet IPN from #faucet channel.

### Step 3: Bond as Validator

```bash
cargo run -p ippan-cli -- \
  --rpc-url http://localhost:8080 \
  validator bond \
  --amount 10000000000 \
  --key-file /etc/ippan/validator-key.json
```

### Step 4: Verify Bonding

```bash
curl http://localhost:8080/validators | jq '.[] | select(.id=="YOUR_ID")'
```

### Step 5: Monitor Validator Status

```bash
# Check reputation score
curl http://localhost:8080/validators/YOUR_ID | jq '.reputation'

# Check blocks proposed
curl http://localhost:8080/validators/YOUR_ID | jq '.blocks_proposed'

# Check rewards earned
curl http://localhost:8080/validators/YOUR_ID | jq '.rewards_earned'
```

---

## Monitoring & Alerts

### Prometheus Configuration

**File:** `/etc/prometheus/prometheus.yml`

```yaml
scrape_configs:
  - job_name: 'ippan-testnet'
    static_configs:
      - targets: ['localhost:9615']
    scrape_interval: 15s
```

### Grafana Dashboards

**Pre-built dashboards:**
- `grafana_dashboards/ippan-consensus.json` - Consensus metrics
- `grafana_dashboards/ippan-network.json` - Network metrics
- `grafana_dashboards/ippan-hashtimer.json` - HashTimer metrics
- `grafana_dashboards/ippan-dlc-fairness.json` - DLC fairness metrics

**Import:**
1. Open Grafana
2. Go to Dashboards → Import
3. Upload JSON file
4. Select Prometheus data source

### Alert Rules

**Critical Alerts:**
- **Node Down:** No scrape for 1 minute
- **Peer Count Low:** < 3 peers for 5 minutes
- **Finality Stalled:** No new finalized blocks for 1 minute
- **Memory High:** > 80% memory usage
- **Disk Full:** > 90% disk usage

---

## Troubleshooting

### Node Won't Start

**Symptom:** Service fails immediately

**Check:**
```bash
# View logs
sudo journalctl -u ippan-node -n 100

# Common issues:
# - Config file syntax error
# - Genesis file not found
# - Ports already in use
# - Permission denied (data dir)
```

**Fix:**
```bash
# Validate config
cat /etc/ippan/testnet.toml

# Check port availability
sudo netstat -tlnp | grep -E "8080|9615|9000"

# Fix permissions
sudo chown -R ippan:ippan /var/lib/ippan
```

### No Peers

**Symptom:** `peer_count: 0` in health check

**Check:**
```bash
# Verify P2P port is open
sudo ufw allow 9000/tcp

# Check logs for connection errors
sudo journalctl -u ippan-node | grep -i "peer\|connection"

# Test connectivity to bootstrap nodes
nc -zv seed1.testnet.ippan.io 9000
```

**Fix:**
- Update firewall rules
- Verify bootstrap node addresses in config
- Check NAT/router configuration

### Finality Stalled

**Symptom:** `finalized_round` not incrementing

**Check:**
```bash
# Check consensus status
curl http://localhost:8080/status | jq '.consensus'

# Check if validator is active
curl http://localhost:8080/validators | jq '.[] | select(.id=="YOUR_ID")'
```

**Fix:**
- Verify node is synced with network
- Check if bonded as validator (if applicable)
- Review logs for consensus errors

### High Memory Usage

**Symptom:** Memory > 80%

**Check:**
```bash
# Check memory usage
free -h

# Check IPPAN process
ps aux | grep ippan-node
```

**Fix:**
```bash
# Restart node
sudo systemctl restart ippan-node

# Consider upgrading hardware if persistent
```

---

## Upgrades

### Minor Upgrade (e.g., v1.0.0-rc1 → v1.0.0-rc2)

```bash
# Stop node
sudo systemctl stop ippan-node

# Backup data
sudo tar -czf /backup/ippan-data-$(date +%s).tar.gz /var/lib/ippan

# Download new binary
wget https://github.com/dmrl789/IPPAN/releases/download/v1.0.0-rc2/ippan-v1.0.0-rc2-linux-x86_64.tar.gz
tar -xzf ippan-v1.0.0-rc2-linux-x86_64.tar.gz
sudo mv ippan-node /usr/local/bin/

# Update config if needed
# vim /etc/ippan/testnet.toml

# Start node
sudo systemctl start ippan-node

# Verify version
curl http://localhost:8080/health | jq '.version'
```

### Major Upgrade (e.g., v1.0.0 → v2.0.0)

**Follow official upgrade guide:**
- `docs/operators/upgrades-and-migrations.md`

**May require:**
- Genesis file update
- Database migration
- Config format changes

---

## Backup & Recovery

### Backup

```bash
# Stop node
sudo systemctl stop ippan-node

# Backup data directory
sudo tar -czf /backup/ippan-testnet-$(date +%Y%m%d).tar.gz \
  /var/lib/ippan/testnet

# Backup config
sudo cp /etc/ippan/testnet.toml /backup/

# Backup validator key (if applicable)
sudo cp /etc/ippan/validator-key.json /backup/

# Start node
sudo systemctl start ippan-node
```

### Recovery

```bash
# Stop node
sudo systemctl stop ippan-node

# Clear current data
sudo rm -rf /var/lib/ippan/testnet

# Restore from backup
sudo tar -xzf /backup/ippan-testnet-20251124.tar.gz -C /

# Restore config
sudo cp /backup/testnet.toml /etc/ippan/

# Start node
sudo systemctl start ippan-node
```

---

## Security Best Practices

1. **Firewall:**
   - Block all ports except RPC, metrics, P2P
   - Whitelist trusted IPs for RPC access

2. **Validator Keys:**
   - Store validator keys securely (encrypted)
   - Use HSM for production validators (future)
   - Never share private keys

3. **Updates:**
   - Subscribe to security advisories
   - Apply patches promptly
   - Test upgrades on staging first

4. **Monitoring:**
   - Set up alerts for critical metrics
   - Review logs daily
   - Monitor for suspicious activity

---

## Support

**Resources:**
- Discord: https://discord.gg/ippan
- GitHub Issues: https://github.com/dmrl789/IPPAN/issues
- Documentation: https://docs.ippan.io

**Reporting Issues:**
- Use GitHub issue template: "RC Bug Report"
- Include: version, OS, logs, config (sanitized)

---

**Maintainers:**  
- Kambei Sapote (Network Engineer)
- Ugo Giuliani (Lead Architect)

**Last Updated:** 2025-11-24
