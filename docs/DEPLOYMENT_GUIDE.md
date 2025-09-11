# IPPAN Blockchain Deployment Guide

## Table of Contents
1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [System Requirements](#system-requirements)
4. [Installation](#installation)
5. [Configuration](#configuration)
6. [Deployment Options](#deployment-options)
7. [Network Setup](#network-setup)
8. [Security Configuration](#security-configuration)
9. [Monitoring Setup](#monitoring-setup)
10. [Troubleshooting](#troubleshooting)
11. [Maintenance](#maintenance)

## Overview

This guide provides comprehensive instructions for deploying the IPPAN blockchain network to production environments. IPPAN is a high-performance, quantum-resistant blockchain with advanced features including BlockDAG consensus, ZK-STARK proofs, and comprehensive security measures.

## Prerequisites

### Software Requirements
- **Rust**: Version 1.70+ (latest stable recommended)
- **Docker**: Version 20.10+ with Docker Compose
- **Kubernetes**: Version 1.25+ (for K8s deployment)
- **Helm**: Version 3.8+ (for K8s deployment)
- **Protocol Buffers**: Version 3.20+ (protoc compiler)
- **Node.js**: Version 18+ (for frontend applications)
- **Nginx**: Version 1.20+ (for load balancing)

### System Requirements

#### Minimum Requirements (Testnet)
- **CPU**: 4 cores, 2.4 GHz
- **RAM**: 8 GB
- **Storage**: 100 GB SSD
- **Network**: 100 Mbps

#### Recommended Requirements (Mainnet)
- **CPU**: 8+ cores, 3.0+ GHz
- **RAM**: 32+ GB
- **Storage**: 1+ TB NVMe SSD
- **Network**: 1+ Gbps

#### Production Requirements (High-Performance)
- **CPU**: 16+ cores, 3.5+ GHz
- **RAM**: 64+ GB
- **Storage**: 2+ TB NVMe SSD with RAID
- **Network**: 10+ Gbps

## Installation

### 1. Clone Repository
```bash
git clone https://github.com/your-org/ippan.git
cd ippan
```

### 2. Install Dependencies

#### On Ubuntu/Debian:
```bash
# Install system dependencies
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev libudev-dev

# Install Protocol Buffers
sudo apt install -y protobuf-compiler

# Install Node.js
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt install -y nodejs

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER
```

#### On Windows:
```powershell
# Install Protocol Buffers
winget install Google.Protobuf

# Install Node.js
winget install OpenJS.NodeJS

# Install Docker Desktop
winget install Docker.DockerDesktop
```

#### On macOS:
```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install protobuf node docker
```

### 3. Build IPPAN
```bash
# Build the project
cargo build --release

# Build frontend applications
cd apps/unified-ui
npm install
npm run build

cd ../wallet
npm install
npm run build
```

## Configuration

### 1. Node Configuration

Create a configuration file for your node:

```toml
# configs/production-node.toml
[network]
network_id = "ippan-mainnet"
chain_id = "ippan-1"
node_id = "your-unique-node-id"
is_bootstrap_node = false
listen_address = "0.0.0.0:30333"
api_address = "0.0.0.0:8080"
p2p_address = "0.0.0.0:30333"

[bootstrap_nodes]
nodes = [
    "12D3KooW...@bootstrap1.ippan.net:30333",
    "12D3KooW...@bootstrap2.ippan.net:30333"
]

[consensus]
consensus_type = "bft"
block_time = 5
max_block_size = 1048576
max_transactions_per_block = 1000
finality_threshold = 0.67

[security]
enable_tls = true
cert_path = "/etc/ippan/certs/server.crt"
key_path = "/etc/ippan/certs/server.key"
enable_encryption = true
encryption_key = "your-encryption-key"

[storage]
data_dir = "/var/lib/ippan"
max_storage_size = "1TB"
enable_compression = true
enable_deduplication = true

[monitoring]
enable_metrics = true
metrics_port = 9090
enable_health_checks = true
health_check_interval = 30
```

### 2. Network Configuration

#### Bootstrap Nodes
Bootstrap nodes are critical for network initialization. Configure at least 3-5 bootstrap nodes:

```toml
# configs/bootstrap-node.toml
[network]
network_id = "ippan-mainnet"
chain_id = "ippan-1"
node_id = "bootstrap-node-1"
is_bootstrap_node = true
listen_address = "0.0.0.0:30333"
api_address = "0.0.0.0:8080"

[consensus]
consensus_type = "bft"
block_time = 5
max_block_size = 1048576

[security]
enable_tls = true
cert_path = "/etc/ippan/certs/bootstrap.crt"
key_path = "/etc/ippan/certs/bootstrap.key"
```

#### Validator Nodes
```toml
# configs/validator-node.toml
[network]
network_id = "ippan-mainnet"
chain_id = "ippan-1"
node_id = "validator-node-1"
is_bootstrap_node = false
listen_address = "0.0.0.0:30333"
api_address = "0.0.0.0:8080"

[bootstrap_nodes]
nodes = [
    "12D3KooW...@bootstrap1.ippan.net:30333",
    "12D3KooW...@bootstrap2.ippan.net:30333"
]

[staking]
validator_address = "your-validator-address"
stake_amount = 1000000
commission_rate = 0.05
```

## Deployment Options

### 1. Docker Deployment

#### Single Node
```bash
# Build Docker image
docker build -f Dockerfile.production -t ippan:latest .

# Run single node
docker run -d \
  --name ippan-node \
  -p 30333:30333 \
  -p 8080:8080 \
  -v $(pwd)/configs:/etc/ippan/configs \
  -v $(pwd)/data:/var/lib/ippan \
  ippan:latest \
  --config /etc/ippan/configs/production-node.toml
```

#### Multi-Node Network
```bash
# Deploy testnet
cd deployments/testnet
docker-compose -f docker-compose.testnet.yml up -d

# Check status
docker-compose -f docker-compose.testnet.yml ps
```

### 2. Kubernetes Deployment

#### Using Helm
```bash
# Add IPPAN Helm repository
helm repo add ippan https://charts.ippan.net
helm repo update

# Deploy IPPAN network
helm install ippan-mainnet ippan/ippan \
  --set network.networkId=ippan-mainnet \
  --set network.chainId=ippan-1 \
  --set node.replicas=5 \
  --set node.resources.requests.cpu=2 \
  --set node.resources.requests.memory=8Gi \
  --set storage.size=1Ti
```

#### Using YAML Manifests
```bash
# Deploy IPPAN nodes
kubectl apply -f deployments/kubernetes/ippan-deployment.yaml

# Deploy monitoring
kubectl apply -f deployments/kubernetes/ippan-monitoring.yaml

# Check deployment status
kubectl get pods -l app=ippan
```

### 3. Bare Metal Deployment

#### Systemd Service
```bash
# Create systemd service file
sudo tee /etc/systemd/system/ippan.service > /dev/null <<EOF
[Unit]
Description=IPPAN Blockchain Node
After=network.target

[Service]
Type=simple
User=ippan
Group=ippan
WorkingDirectory=/opt/ippan
ExecStart=/opt/ippan/target/release/ippan-node --config /etc/ippan/configs/production-node.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable ippan
sudo systemctl start ippan
```

## Network Setup

### 1. Firewall Configuration

#### UFW (Ubuntu)
```bash
# Allow P2P port
sudo ufw allow 30333/tcp

# Allow API port
sudo ufw allow 8080/tcp

# Allow metrics port
sudo ufw allow 9090/tcp

# Enable firewall
sudo ufw enable
```

#### Firewalld (CentOS/RHEL)
```bash
# Allow P2P port
sudo firewall-cmd --permanent --add-port=30333/tcp

# Allow API port
sudo firewall-cmd --permanent --add-port=8080/tcp

# Allow metrics port
sudo firewall-cmd --permanent --add-port=9090/tcp

# Reload firewall
sudo firewall-cmd --reload
```

### 2. Load Balancer Configuration

#### Nginx Configuration
```nginx
# /etc/nginx/sites-available/ippan
upstream ippan_api {
    server 10.0.1.10:8080;
    server 10.0.1.11:8080;
    server 10.0.1.12:8080;
    server 10.0.1.13:8080;
    server 10.0.1.14:8080;
}

server {
    listen 80;
    server_name api.ippan.net;

    location / {
        proxy_pass http://ippan_api;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### 3. DNS Configuration

Configure DNS records for your network:

```
# A records
bootstrap1.ippan.net    A    10.0.1.10
bootstrap2.ippan.net    A    10.0.1.11
bootstrap3.ippan.net    A    10.0.1.12
validator1.ippan.net    A    10.0.1.13
validator2.ippan.net    A    10.0.1.14
api.ippan.net           A    10.0.1.100

# CNAME records
www.ippan.net           CNAME api.ippan.net
```

## Security Configuration

### 1. TLS/SSL Setup

#### Generate Certificates
```bash
# Create certificate directory
sudo mkdir -p /etc/ippan/certs

# Generate private key
sudo openssl genrsa -out /etc/ippan/certs/server.key 4096

# Generate certificate signing request
sudo openssl req -new -key /etc/ippan/certs/server.key -out /etc/ippan/certs/server.csr

# Generate self-signed certificate (for testing)
sudo openssl x509 -req -days 365 -in /etc/ippan/certs/server.csr -signkey /etc/ippan/certs/server.key -out /etc/ippan/certs/server.crt
```

#### Configure TLS in Node
```toml
[security]
enable_tls = true
cert_path = "/etc/ippan/certs/server.crt"
key_path = "/etc/ippan/certs/server.key"
ca_cert_path = "/etc/ippan/certs/ca.crt"
```

### 2. Network Security

#### VPN Setup
```bash
# Install WireGuard
sudo apt install wireguard

# Generate keys
wg genkey | tee privatekey | wg pubkey > publickey

# Configure WireGuard
sudo tee /etc/wireguard/wg0.conf > /dev/null <<EOF
[Interface]
PrivateKey = $(cat privatekey)
Address = 10.0.0.1/24
ListenPort = 51820

[Peer]
PublicKey = $(cat peer_publickey)
AllowedIPs = 10.0.0.2/32
Endpoint = peer.ippan.net:51820
EOF

# Start WireGuard
sudo systemctl enable wg-quick@wg0
sudo systemctl start wg-quick@wg0
```

### 3. Access Control

#### API Authentication
```toml
[api]
enable_auth = true
auth_type = "jwt"
jwt_secret = "your-jwt-secret"
rate_limit = 1000
rate_limit_window = 3600
```

#### Admin Access
```toml
[admin]
enable_admin_api = true
admin_users = [
    "admin1:hashed_password",
    "admin2:hashed_password"
]
admin_ips = [
    "10.0.0.0/8",
    "192.168.0.0/16"
]
```

## Monitoring Setup

### 1. Prometheus Configuration

```yaml
# monitoring/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "ippan_rules.yml"

scrape_configs:
  - job_name: 'ippan-nodes'
    static_configs:
      - targets: ['node1:9090', 'node2:9090', 'node3:9090']
    scrape_interval: 5s
    metrics_path: /metrics

  - job_name: 'ippan-api'
    static_configs:
      - targets: ['api1:8080', 'api2:8080']
    scrape_interval: 10s
    metrics_path: /api/v1/metrics
```

### 2. Grafana Dashboards

Import the IPPAN dashboard:

```bash
# Download dashboard
curl -o ippan-dashboard.json https://grafana.com/api/dashboards/ippan/revisions/1/download

# Import to Grafana
curl -X POST \
  -H "Content-Type: application/json" \
  -d @ippan-dashboard.json \
  http://admin:admin@grafana:3000/api/dashboards/db
```

### 3. Alerting Rules

```yaml
# monitoring/ippan_rules.yml
groups:
  - name: ippan
    rules:
      - alert: NodeDown
        expr: up{job="ippan-nodes"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "IPPAN node is down"
          description: "Node {{ $labels.instance }} has been down for more than 1 minute"

      - alert: HighCPUUsage
        expr: cpu_usage_percent > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High CPU usage on IPPAN node"
          description: "CPU usage is {{ $value }}% on node {{ $labels.instance }}"

      - alert: HighMemoryUsage
        expr: memory_usage_percent > 85
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage on IPPAN node"
          description: "Memory usage is {{ $value }}% on node {{ $labels.instance }}"
```

## Troubleshooting

### 1. Common Issues

#### Node Won't Start
```bash
# Check logs
journalctl -u ippan -f

# Check configuration
ippan-node --config /etc/ippan/configs/production-node.toml --validate

# Check ports
netstat -tlnp | grep :30333
```

#### Network Connectivity Issues
```bash
# Test P2P connectivity
telnet bootstrap1.ippan.net 30333

# Check firewall
sudo ufw status
sudo iptables -L

# Test DNS resolution
nslookup bootstrap1.ippan.net
```

#### Performance Issues
```bash
# Check system resources
htop
iostat -x 1
df -h

# Check node metrics
curl http://localhost:9090/metrics

# Check network latency
ping bootstrap1.ippan.net
```

### 2. Log Analysis

#### View Logs
```bash
# System logs
journalctl -u ippan -f

# Application logs
tail -f /var/lib/ippan/logs/ippan.log

# Error logs
grep -i error /var/lib/ippan/logs/ippan.log
```

#### Log Rotation
```bash
# Configure logrotate
sudo tee /etc/logrotate.d/ippan > /dev/null <<EOF
/var/lib/ippan/logs/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 ippan ippan
    postrotate
        systemctl reload ippan
    endscript
}
EOF
```

## Maintenance

### 1. Regular Maintenance Tasks

#### Daily
- Check node status and health
- Review error logs
- Monitor resource usage
- Verify network connectivity

#### Weekly
- Update system packages
- Review security logs
- Check disk space
- Validate backups

#### Monthly
- Update IPPAN software
- Review and rotate certificates
- Analyze performance metrics
- Update documentation

### 2. Backup Procedures

#### Database Backup
```bash
# Create backup script
sudo tee /usr/local/bin/ippan-backup.sh > /dev/null <<EOF
#!/bin/bash
BACKUP_DIR="/var/backups/ippan"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p $BACKUP_DIR

# Backup database
cp -r /var/lib/ippan/data $BACKUP_DIR/data_$DATE

# Backup configuration
cp -r /etc/ippan/configs $BACKUP_DIR/configs_$DATE

# Compress backup
tar -czf $BACKUP_DIR/ippan_backup_$DATE.tar.gz -C $BACKUP_DIR data_$DATE configs_$DATE

# Remove uncompressed files
rm -rf $BACKUP_DIR/data_$DATE $BACKUP_DIR/configs_$DATE

# Keep only last 30 days of backups
find $BACKUP_DIR -name "ippan_backup_*.tar.gz" -mtime +30 -delete
EOF

# Make executable
sudo chmod +x /usr/local/bin/ippan-backup.sh

# Schedule daily backup
echo "0 2 * * * /usr/local/bin/ippan-backup.sh" | sudo crontab -
```

### 3. Update Procedures

#### Rolling Update
```bash
# Stop node
sudo systemctl stop ippan

# Backup current version
sudo cp /opt/ippan/target/release/ippan-node /opt/ippan/target/release/ippan-node.backup

# Update software
cd /opt/ippan
git pull
cargo build --release

# Start node
sudo systemctl start ippan

# Verify update
sudo systemctl status ippan
```

#### Zero-Downtime Update
```bash
# Deploy new version to staging
kubectl set image deployment/ippan-node ippan-node=ippan:new-version

# Wait for rollout
kubectl rollout status deployment/ippan-node

# Verify health
kubectl get pods -l app=ippan
```

### 4. Disaster Recovery

#### Recovery Procedures
```bash
# Restore from backup
sudo systemctl stop ippan
sudo rm -rf /var/lib/ippan/data
sudo tar -xzf /var/backups/ippan/ippan_backup_20240101_020000.tar.gz
sudo mv data_20240101_020000 /var/lib/ippan/data
sudo systemctl start ippan

# Verify recovery
sudo systemctl status ippan
curl http://localhost:8080/health
```

## Support

For additional support and documentation:

- **Documentation**: https://docs.ippan.net
- **GitHub**: https://github.com/your-org/ippan
- **Discord**: https://discord.gg/ippan
- **Email**: support@ippan.net

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.