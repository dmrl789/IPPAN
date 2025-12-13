# Hetzner Devnet-1 Deployment Guide

**Status:** Ready for deployment  
**Network:** Devnet (non-production validation environment)  
**Topology:** 4 nodes (3 validators + 1 observer/RPC)

---

## Phase A Status ✅

**Latest Readiness Pulse Run:**
- **Run ID:** 20001036008
- **Status:** ✅ **SUCCESS** (completed)
- **URL:** https://github.com/dmrl789/IPPAN/actions/runs/20001036008
- **Date:** 2025-12-07T07:41:35Z

**Audit Pack Evidence:** ✅ Confirmed (see `docs/audit/LAST_AUDIT_PACK_RUN.md`)

**Decision:** **GO for Hetzner Devnet-1 deployment**

---

## Server Requirements

### Minimum Specifications (per node)
- **OS:** Ubuntu 24.04 LTS (recommended)
- **CPU:** 4 vCPU
- **RAM:** 8 GB
- **Disk:** 80 GB SSD
- **Network:** Private network enabled (preferred)

### Server Roles
- **node1:** Validator + Bootstrap peer
- **node2:** Validator
- **node3:** Validator
- **node4:** Observer/RPC (rate-limited if exposed)

---

## Deployment Steps

### STEP C1 — Provision Servers

Provision 4 Hetzner servers via Hetzner Cloud Console or API:
- Ubuntu 24.04 LTS
- Private network enabled (recommended)
- SSH key access configured

**Record the following for each node:**
- Public IP address
- Private IP address (if using private network)
- Hostname

---

### STEP C2 — Prepare Each Server

**Run on EACH host (node1, node2, node3, node4):**

```bash
# Update system
sudo apt update && sudo apt -y upgrade

# Install dependencies
sudo apt -y install build-essential pkg-config libssl-dev clang cmake git ufw jq curl

# Install Rust
curl https://sh.rustup.rs -sSf | sh -s -- -y
source $HOME/.cargo/env
rustup default stable

# Verify installation
rustc --version
cargo --version
```

---

### STEP C3 — Clone Repo and Build Binary

**Run on EACH host:**

```bash
# Create directory structure
sudo mkdir -p /opt/ippan
sudo chown $USER:$USER /opt/ippan
cd /opt/ippan

# Clone repository
git clone https://github.com/dmrl789/IPPAN .
git checkout master

# Build release binary
cargo build --release -p ippan-node

# Verify binary exists
ls -lh target/release/ippan-node
```

**Expected build time:** 15-30 minutes depending on server specs.

---

### STEP C4 — Create Config Directories and Node Identity

**Run on EACH host:**

```bash
# Create directories
sudo mkdir -p /etc/ippan /var/lib/ippan /var/log/ippan
sudo chown $USER:$USER /etc/ippan /var/lib/ippan /var/log/ippan

# Copy baseline config
cp -r config /etc/ippan/config

# Verify dlc.toml model configuration
grep -A 5 "\[dgbdt.model\]" /etc/ippan/config/dlc.toml
```

**Node-specific configuration:**

#### Node 1 (Bootstrap Validator)
```bash
cat > /etc/ippan/config/node.toml << 'EOF'
[node]
id = "ippan-devnet-node1"

[network]
id = "ippan-devnet"

[rpc]
host = "0.0.0.0"
port = 8080

[p2p]
host = "0.0.0.0"
port = 9000
bootstrap_nodes = ""

[storage]
data_dir = "/var/lib/ippan"
db_path = "/var/lib/ippan/db"

[logging]
level = "info"
format = "json"

[metrics]
enabled = true
EOF
```

#### Node 2 (Validator)
```bash
# Replace NODE1_PRIVATE_IP with node1's private IP (or public if no private network)
cat > /etc/ippan/config/node.toml << EOF
[node]
id = "ippan-devnet-node2"

[network]
id = "ippan-devnet"

[rpc]
host = "0.0.0.0"
port = 8080

[p2p]
host = "0.0.0.0"
port = 9000
bootstrap_nodes = "http://NODE1_PRIVATE_IP:9000"

[storage]
data_dir = "/var/lib/ippan"
db_path = "/var/lib/ippan/db"

[logging]
level = "info"
format = "json"

[metrics]
enabled = true
EOF
```

#### Node 3 (Validator)
```bash
# Replace NODE1_PRIVATE_IP with node1's private IP (or public if no private network)
cat > /etc/ippan/config/node.toml << EOF
[node]
id = "ippan-devnet-node3"

[network]
id = "ippan-devnet"

[rpc]
host = "0.0.0.0"
port = 8080

[p2p]
host = "0.0.0.0"
port = 9000
bootstrap_nodes = "http://NODE1_PRIVATE_IP:9000"

[storage]
data_dir = "/var/lib/ippan"
db_path = "/var/lib/ippan/db"

[logging]
level = "info"
format = "json"

[metrics]
enabled = true
EOF
```

#### Node 4 (Observer/RPC)
```bash
# Replace NODE1_PRIVATE_IP with node1's private IP (or public if no private network)
cat > /etc/ippan/config/node.toml << EOF
[node]
id = "ippan-devnet-node4"

[network]
id = "ippan-devnet"

[rpc]
host = "0.0.0.0"
port = 8080

[p2p]
host = "0.0.0.0"
port = 9000
bootstrap_nodes = "http://NODE1_PRIVATE_IP:9000"

[storage]
data_dir = "/var/lib/ippan"
db_path = "/var/lib/ippan/db"

[logging]
level = "info"
format = "json"

[metrics]
enabled = true
EOF
```

---

### STEP C5 — Systemd Service

**Run on EACH host:**

```bash
sudo tee /etc/systemd/system/ippan-node.service > /dev/null << 'EOF'
[Unit]
Description=IPPAN Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/ippan
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1
ExecStart=/opt/ippan/target/release/ippan-node --config /etc/ippan/config/node.toml
Restart=always
RestartSec=2
LimitNOFILE=1048576
StandardOutput=append:/var/log/ippan/node.log
StandardError=append:/var/log/ippan/node.err

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd and enable service
sudo systemctl daemon-reload
sudo systemctl enable ippan-node
```

**Start the service:**
```bash
sudo systemctl start ippan-node
sudo systemctl status ippan-node --no-pager
```

**Useful commands:**
```bash
# Check status
sudo systemctl status ippan-node

# View logs
sudo journalctl -u ippan-node -f
tail -f /var/log/ippan/node.log

# Restart service
sudo systemctl restart ippan-node

# Stop service
sudo systemctl stop ippan-node
```

---

### STEP C6 — Networking / Firewall

**Run on EACH host:**

```bash
# Allow SSH
sudo ufw allow 22/tcp

# Allow RPC/HTTP
sudo ufw allow 8080/tcp

# Allow P2P
sudo ufw allow 9000/tcp
sudo ufw allow 9000/udp

# Enable firewall
sudo ufw --force enable

# Verify rules
sudo ufw status verbose
```

**If using private network:**
- Ensure private network is configured in Hetzner Cloud Console
- Nodes should communicate via private IPs for P2P
- Public IPs only needed for external RPC access (node4)

---

### STEP C7 — Bootstrapping Peers

**Critical:** Node1 is the bootstrap peer. Nodes 2, 3, and 4 must point to node1.

**On node1:** Ensure `bootstrap_nodes = ""` (empty, as it's the bootstrap)

**On nodes 2, 3, 4:** Ensure `bootstrap_nodes = "http://NODE1_IP:9000"` points to node1's IP.

**After updating configs, restart services:**
```bash
sudo systemctl restart ippan-node
```

**Verify connectivity:**
```bash
# On node2, node3, node4 - check logs for peer connection
sudo journalctl -u ippan-node -n 50 | grep -i peer
```

---

### STEP C8 — Validation Checklist

**From your laptop/workstation, validate the deployment:**

#### 1. Check node status (especially node4 observer/RPC):
```bash
# Replace NODE4_IP with node4's public IP
curl http://NODE4_IP:8080/status | jq .
```

**Expected response:**
```json
{
  "status": "ok",
  "network": "ippan-devnet",
  "node_id": "ippan-devnet-node4",
  "validators": 3,
  "metrics_available": true,
  ...
}
```

#### 2. Verify validators count >= 3:
```bash
curl http://NODE4_IP:8080/status | jq '.validators'
```

#### 3. Check peer connections:
```bash
curl http://NODE4_IP:8080/p2p/peers | jq .
```

#### 4. Monitor logs on each node:
```bash
# SSH to each node and check
sudo journalctl -u ippan-node -f
```

#### 5. Run 24h soak test:
Use your existing soak workflow or run locally:
```bash
# From your laptop
# (Use your soak test script or workflow)
```

---

### STEP C9 — Observability

#### Log Rotation

**Create logrotate config on EACH host:**

```bash
sudo tee /etc/logrotate.d/ippan-node > /dev/null << 'EOF'
/var/log/ippan/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0640 root root
    sharedscripts
    postrotate
        systemctl reload ippan-node > /dev/null 2>&1 || true
    endscript
}
EOF
```

#### Baseline Metrics

**Record baseline on EACH host:**
```bash
# CPU/RAM
top -bn1 | head -5

# Disk usage
df -h /var/lib/ippan

# Network
ss -tuln | grep -E ':(8080|9000)'
```

#### Health Check Script

**Create simple health check on node4 (observer):**

```bash
sudo tee /usr/local/bin/ippan-healthcheck.sh > /dev/null << 'EOF'
#!/bin/bash
curl -s http://localhost:8080/status | jq -e '.status == "ok" && .validators >= 3' > /dev/null
exit $?
EOF

sudo chmod +x /usr/local/bin/ippan-healthcheck.sh

# Add to cron (runs every 5 minutes)
(crontab -l 2>/dev/null; echo "*/5 * * * * /usr/local/bin/ippan-healthcheck.sh || systemctl restart ippan-node") | crontab -
```

---

## Troubleshooting

### Node won't start
```bash
# Check logs
sudo journalctl -u ippan-node -n 100

# Verify binary exists
ls -lh /opt/ippan/target/release/ippan-node

# Check config syntax
/opt/ippan/target/release/ippan-node --config /etc/ippan/config/node.toml --check
```

### Peers not connecting
```bash
# Verify firewall rules
sudo ufw status

# Check if ports are listening
sudo ss -tuln | grep -E ':(8080|9000)'

# Verify bootstrap node IP is correct in config
grep bootstrap_nodes /etc/ippan/config/node.toml

# Check network connectivity
curl -v http://NODE1_IP:9000/p2p/peer-info
```

### High resource usage
```bash
# Check system resources
htop

# Check node metrics
curl http://NODE_IP:8080/metrics

# Review logs for errors
sudo journalctl -u ippan-node --since "1 hour ago" | grep -i error
```

---

## Next Steps

After successful deployment:

1. **Monitor for 24-48 hours** to ensure stability
2. **Run soak tests** to validate consensus behavior
3. **Test upgrades** (build new binary, restart services)
4. **Document any issues** encountered
5. **Prepare for mainnet** deployment (when ready)

---

## Server Information Template

**Fill this out after provisioning:**

```
Node 1 (Bootstrap Validator):
- Public IP: _______________
- Private IP: _______________
- Hostname: _______________

Node 2 (Validator):
- Public IP: _______________
- Private IP: _______________
- Hostname: _______________

Node 3 (Validator):
- Public IP: _______________
- Private IP: _______________
- Hostname: _______________

Node 4 (Observer/RPC):
- Public IP: _______________
- Private IP: _______________
- Hostname: _______________
```

---

**Last Updated:** 2025-12-07  
**Deployment Status:** Ready for execution

