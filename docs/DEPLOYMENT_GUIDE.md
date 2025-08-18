# IPPAN Deployment Guide

## Overview

This guide provides comprehensive instructions for deploying IPPAN (InterPlanetary Network) nodes in various environments, from development to production.

## Prerequisites

### System Requirements

- **CPU**: 4+ cores (8+ recommended for production)
- **RAM**: 8GB minimum (16GB+ recommended for production)
- **Storage**: 100GB+ SSD (1TB+ recommended for production)
- **Network**: 100Mbps+ connection (1Gbps+ recommended)
- **OS**: Linux (Ubuntu 20.04+ recommended), macOS, or Windows

### Software Dependencies

- **Rust**: 1.70+ (latest stable recommended)
- **Docker**: 20.10+ (for containerized deployment)
- **Git**: Latest version
- **OpenSSL**: 1.1.1+ (for cryptography)

## Installation Methods

### 1. From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/ippan/ippan.git
cd ippan

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build the project
cargo build --release

# Install the binary
cargo install --path .
```

### 2. Using Docker

```bash
# Pull the official image
docker pull ippan/ippan:latest

# Run a node
docker run -d \
  --name ippan-node \
  -p 8080:8080 \
  -p 8081:8081 \
  -v /data/ippan:/data \
  ippan/ippan:latest
```

### 3. Using Package Managers

#### Ubuntu/Debian
```bash
# Add repository
curl -fsSL https://packages.ippan.network/gpg | sudo gpg --dearmor -o /usr/share/keyrings/ippan-archive-keyring.gpg
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/ippan-archive-keyring.gpg] https://packages.ippan.network/ubuntu focal main" | sudo tee /etc/apt/sources.list.d/ippan.list

# Install
sudo apt update
sudo apt install ippan
```

## Configuration

### 1. Basic Configuration

Create a configuration file at `~/.ippan/config.toml`:

```toml
[network]
# Network mode: mainnet, testnet, or devnet
mode = "mainnet"

# Node identity
node_id = "your-node-id-here"

# Listen addresses
listen_addr = "0.0.0.0:8080"
p2p_listen_addr = "0.0.0.0:8081"

# Bootstrap nodes (for initial peer discovery)
bootstrap_nodes = [
    "node1.ippan.network:8081",
    "node2.ippan.network:8081",
    "node3.ippan.network:8081"
]

[consensus]
# Consensus algorithm: roundchain, blockdag
algorithm = "roundchain"

# Block time in seconds
block_time = 3

# Maximum transactions per block
max_transactions_per_block = 10000

[storage]
# Storage mode: local, distributed, or hybrid
mode = "distributed"

# Local storage path
data_dir = "/data/ippan"

# Replication factor for distributed storage
replication_factor = 3

# Encryption settings
encryption_enabled = true
encryption_algorithm = "aes256-gcm"

[quantum]
# Quantum-resistant cryptography settings
enabled = true
default_algorithm = "kyber"
security_level = "level3"
hybrid_encryption = true

[api]
# API server settings
enabled = true
bind_addr = "0.0.0.0:8080"
max_connections = 1000
rate_limit_requests_per_minute = 1000

[security]
# Security settings
tls_enabled = true
tls_cert_path = "/path/to/cert.pem"
tls_key_path = "/path/to/key.pem"

# Firewall settings
allowed_ips = ["0.0.0.0/0"]
blocked_ips = []

[monitoring]
# Monitoring and logging
log_level = "info"
log_file = "/var/log/ippan/node.log"
metrics_enabled = true
metrics_port = 9090

# Health check settings
health_check_interval = 30
health_check_timeout = 5
```

### 2. Environment-Specific Configurations

#### Development Environment
```toml
[network]
mode = "devnet"
bootstrap_nodes = ["localhost:8081"]

[consensus]
block_time = 1
max_transactions_per_block = 1000

[api]
rate_limit_requests_per_minute = 10000
```

#### Production Environment
```toml
[network]
mode = "mainnet"
bootstrap_nodes = [
    "mainnet-node1.ippan.network:8081",
    "mainnet-node2.ippan.network:8081",
    "mainnet-node3.ippan.network:8081"
]

[consensus]
block_time = 3
max_transactions_per_block = 10000

[security]
tls_enabled = true
allowed_ips = ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"]

[monitoring]
log_level = "warn"
metrics_enabled = true
```

## Deployment Scenarios

### 1. Single Node Deployment

```bash
# Create data directory
sudo mkdir -p /data/ippan
sudo chown $USER:$USER /data/ippan

# Start the node
ippan --config ~/.ippan/config.toml
```

### 2. Multi-Node Cluster

#### Using Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  ippan-node-1:
    image: ippan/ippan:latest
    container_name: ippan-node-1
    ports:
      - "8080:8080"
      - "8081:8081"
    volumes:
      - ./config/node1.toml:/etc/ippan/config.toml
      - node1-data:/data
    environment:
      - RUST_LOG=info
    restart: unless-stopped

  ippan-node-2:
    image: ippan/ippan:latest
    container_name: ippan-node-2
    ports:
      - "8082:8080"
      - "8083:8081"
    volumes:
      - ./config/node2.toml:/etc/ippan/config.toml
      - node2-data:/data
    environment:
      - RUST_LOG=info
    restart: unless-stopped

  ippan-node-3:
    image: ippan/ippan:latest
    container_name: ippan-node-3
    ports:
      - "8084:8080"
      - "8085:8081"
    volumes:
      - ./config/node3.toml:/etc/ippan/config.toml
      - node3-data:/data
    environment:
      - RUST_LOG=info
    restart: unless-stopped

volumes:
  node1-data:
  node2-data:
  node3-data:
```

#### Using Kubernetes

Create `ippan-deployment.yaml`:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ippan-node
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ippan-node
  template:
    metadata:
      labels:
        app: ippan-node
    spec:
      containers:
      - name: ippan
        image: ippan/ippan:latest
        ports:
        - containerPort: 8080
        - containerPort: 8081
        volumeMounts:
        - name: config
          mountPath: /etc/ippan
        - name: data
          mountPath: /data
        env:
        - name: RUST_LOG
          value: "info"
      volumes:
      - name: config
        configMap:
          name: ippan-config
      - name: data
        persistentVolumeClaim:
          claimName: ippan-data-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: ippan-service
spec:
  selector:
    app: ippan-node
  ports:
  - name: api
    port: 8080
    targetPort: 8080
  - name: p2p
    port: 8081
    targetPort: 8081
  type: LoadBalancer
```

### 3. Cloud Deployment

#### AWS Deployment

```bash
# Create EC2 instance
aws ec2 run-instances \
  --image-id ami-0c02fb55956c7d316 \
  --instance-type t3.large \
  --key-name your-key-pair \
  --security-group-ids sg-12345678 \
  --subnet-id subnet-12345678 \
  --user-data file://user-data.sh
```

User data script (`user-data.sh`):
```bash
#!/bin/bash
yum update -y
yum install -y docker git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Clone and build IPPAN
git clone https://github.com/ippan/ippan.git
cd ippan
cargo build --release

# Create configuration
mkdir -p ~/.ippan
cat > ~/.ippan/config.toml << EOF
[network]
mode = "mainnet"
node_id = "$(uuidgen)"
listen_addr = "0.0.0.0:8080"
p2p_listen_addr = "0.0.0.0:8081"
EOF

# Start IPPAN
nohup ./target/release/ippan --config ~/.ippan/config.toml > /var/log/ippan.log 2>&1 &
```

#### Google Cloud Deployment

```bash
# Create instance
gcloud compute instances create ippan-node \
  --zone=us-central1-a \
  --machine-type=e2-standard-2 \
  --image-family=ubuntu-2004-lts \
  --image-project=ubuntu-os-cloud \
  --metadata-from-file startup-script=startup-script.sh
```

Startup script (`startup-script.sh`):
```bash
#!/bin/bash
apt-get update
apt-get install -y docker.io git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Build and run IPPAN
git clone https://github.com/ippan/ippan.git
cd ippan
cargo build --release

# Create systemd service
cat > /etc/systemd/system/ippan.service << EOF
[Unit]
Description=IPPAN Node
After=network.target

[Service]
Type=simple
User=ippan
WorkingDirectory=/opt/ippan
ExecStart=/opt/ippan/target/release/ippan --config /opt/ippan/config.toml
Restart=always

[Install]
WantedBy=multi-user.target
EOF

systemctl enable ippan
systemctl start ippan
```

## Security Hardening

### 1. Firewall Configuration

```bash
# UFW (Ubuntu)
sudo ufw allow 8080/tcp  # API
sudo ufw allow 8081/tcp  # P2P
sudo ufw allow 9090/tcp  # Metrics
sudo ufw enable

# iptables
sudo iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 8081 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 9090 -j ACCEPT
```

### 2. TLS/SSL Configuration

```bash
# Generate self-signed certificate
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Or use Let's Encrypt
sudo certbot certonly --standalone -d your-domain.com
```

### 3. User and Permissions

```bash
# Create dedicated user
sudo useradd -r -s /bin/false ippan
sudo mkdir -p /data/ippan
sudo chown ippan:ippan /data/ippan
```

## Monitoring and Logging

### 1. Log Management

```bash
# Configure log rotation
sudo cat > /etc/logrotate.d/ippan << EOF
/var/log/ippan/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 ippan ippan
}
EOF
```

### 2. Metrics Collection

```yaml
# Prometheus configuration
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'ippan'
    static_configs:
      - targets: ['localhost:9090']
```

### 3. Health Checks

```bash
# Health check script
#!/bin/bash
curl -f http://localhost:8080/health || exit 1
```

## Backup and Recovery

### 1. Data Backup

```bash
# Create backup script
#!/bin/bash
BACKUP_DIR="/backup/ippan"
DATE=$(date +%Y%m%d_%H%M%S)

# Stop node
systemctl stop ippan

# Create backup
tar -czf "$BACKUP_DIR/ippan_$DATE.tar.gz" /data/ippan

# Start node
systemctl start ippan

# Clean old backups (keep last 7 days)
find $BACKUP_DIR -name "ippan_*.tar.gz" -mtime +7 -delete
```

### 2. Disaster Recovery

```bash
# Recovery script
#!/bin/bash
BACKUP_FILE="$1"
DATA_DIR="/data/ippan"

# Stop node
systemctl stop ippan

# Restore data
rm -rf $DATA_DIR/*
tar -xzf $BACKUP_FILE -C /

# Start node
systemctl start ippan
```

## Troubleshooting

### Common Issues

1. **Node won't start**
   ```bash
   # Check logs
   journalctl -u ippan -f
   
   # Check configuration
   ippan --config ~/.ippan/config.toml --check-config
   ```

2. **Network connectivity issues**
   ```bash
   # Test P2P connectivity
   telnet node1.ippan.network 8081
   
   # Check firewall
   sudo ufw status
   ```

3. **High memory usage**
   ```bash
   # Monitor memory usage
   htop
   
   # Check for memory leaks
   valgrind --tool=memcheck ippan
   ```

### Performance Tuning

```toml
[performance]
# Increase worker threads
worker_threads = 8

# Optimize memory usage
max_memory_mb = 8192

# Enable connection pooling
connection_pool_size = 100

# Optimize storage
storage_buffer_size = 1048576
```

## Support

For deployment support:

- **Documentation**: https://docs.ippan.network/deployment
- **GitHub Issues**: https://github.com/ippan/ippan/issues
- **Discord**: https://discord.gg/ippan
- **Email**: deployment-support@ippan.network
