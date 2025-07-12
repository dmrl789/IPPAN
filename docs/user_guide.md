# 📖 IPPAN User Guide

Welcome to IPPAN (Immutable Proof & Availability Network)! This guide will help you get started with using IPPAN for decentralized storage and blockchain operations.

## Table of Contents

1. [What is IPPAN?](#what-is-ippan)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [Configuration](#configuration)
5. [Basic Operations](#basic-operations)
6. [Storage Operations](#storage-operations)
7. [Wallet Operations](#wallet-operations)
8. [Domain Management](#domain-management)
9. [API Usage](#api-usage)
10. [Troubleshooting](#troubleshooting)

## What is IPPAN?

IPPAN is a fully decentralized Layer-1 blockchain with built-in global DHT storage. It provides:

- **Immutable Proof**: Prove when any data existed with tenth-of-a-microsecond precision
- **Decentralized Storage**: Trustless, incentivized storage across the network
- **M2M Payments**: Direct payments between devices and AI agents
- **Human-Readable Domains**: Register handles like `@alice.ipn`
- **Global Fund**: Autonomous reward distribution to network participants

### Key Features

- **HashTimers**: Precise timestamping with 0.1 microsecond accuracy
- **BlockDAG**: Scalable consensus with deterministic ordering
- **AES-256 Encryption**: Secure, encrypted storage
- **Staking System**: Permissionless node participation
- **M2M Payments**: Micro-payments for IoT and AI services

## Installation

### Prerequisites

- **Rust**: Version 1.70 or higher
- **Operating System**: Linux, macOS, or Windows
- **Network**: Internet connection for node discovery
- **Storage**: At least 10GB available space

### Installing Rust

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Reload your shell
source ~/.bashrc

# Verify installation
rustc --version
cargo --version
```

### Installing IPPAN

```bash
# Clone the repository
git clone https://github.com/ippan/ippan.git
cd ippan

# Build IPPAN
cargo build --release

# Install IPPAN globally
cargo install --path .
```

### Verifying Installation

```bash
# Check IPPAN version
ippan --version

# Check available commands
ippan --help
```

## Quick Start

### 1. Generate Your Keys

```bash
# Generate a new wallet
ippan wallet generate

# This will create:
# - Private key file: ~/.ippan/keys/private.key
# - Public key file: ~/.ippan/keys/public.key
# - Node ID: ~/.ippan/keys/node_id.txt
```

### 2. Start Your Node

```bash
# Start IPPAN node
ippan node start

# The node will:
# - Connect to the IPPAN network
# - Begin participating in consensus
# - Start providing storage services
# - Begin earning rewards
```

### 3. Check Node Status

```bash
# View node status
ippan node status

# View network information
ippan network info

# View wallet balance
ippan wallet balance
```

## Configuration

### Configuration File

IPPAN uses a TOML configuration file located at `~/.ippan/config.toml`:

```toml
[network]
# Network listening port
port = 8080

# Bootstrap nodes for network discovery
bootstrap_nodes = [
    "node1.ippan.net:8080",
    "node2.ippan.net:8080"
]

[storage]
# Storage directory
data_dir = "~/.ippan/storage"

# Maximum storage space (in bytes)
max_storage = 107374182400  # 100GB

[consensus]
# Minimum stake required (in IPN)
min_stake = 10

# Maximum stake allowed (in IPN)
max_stake = 100

[api]
# API server port
port = 3000

# Enable CORS
enable_cors = true
```

### Environment Variables

You can also configure IPPAN using environment variables:

```bash
export IPPAN_NETWORK_PORT=8080
export IPPAN_STORAGE_DIR="/path/to/storage"
export IPPAN_API_PORT=3000
export IPPAN_LOG_LEVEL=info
```

## Basic Operations

### Node Management

```bash
# Start node
ippan node start

# Stop node
ippan node stop

# Restart node
ippan node restart

# View node logs
ippan node logs

# Update node
ippan node update
```

### Network Operations

```bash
# View connected peers
ippan network peers

# View network statistics
ippan network stats

# Ping a specific node
ippan network ping <node_id>

# View network topology
ippan network topology
```

### Consensus Operations

```bash
# View current consensus state
ippan consensus status

# View validators
ippan consensus validators

# View recent blocks
ippan consensus blocks

# View IPPAN Time
ippan consensus time
```

## Storage Operations

### Uploading Files

```bash
# Upload a single file
ippan storage upload /path/to/file.txt

# Upload with custom name
ippan storage upload /path/to/file.txt --name "my-document"

# Upload directory
ippan storage upload /path/to/directory --recursive

# Upload with encryption
ippan storage upload /path/to/file.txt --encrypt
```

### Downloading Files

```bash
# Download by hash
ippan storage download <file_hash>

# Download to specific location
ippan storage download <file_hash> --output /path/to/save

# Download with verification
ippan storage download <file_hash> --verify
```

### Storage Management

```bash
# View storage usage
ippan storage usage

# View stored files
ippan storage list

# Delete a file
ippan storage delete <file_hash>

# Generate storage proof
ippan storage proof <file_hash>
```

### File Information

```bash
# Get file info
ippan storage info <file_hash>

# Check file availability
ippan storage check <file_hash>

# View file history
ippan storage history <file_hash>
```

## Wallet Operations

### Balance and Transactions

```bash
# View wallet balance
ippan wallet balance

# View transaction history
ippan wallet history

# Send IPN to another address
ippan wallet send <address> <amount>

# View pending transactions
ippan wallet pending
```

### Staking Operations

```bash
# Stake IPN
ippan wallet stake <amount>

# Unstake IPN
ippan wallet unstake <amount>

# View staking status
ippan wallet staking

# View staking rewards
ippan wallet rewards
```

### M2M Payments

```bash
# Create payment channel
ippan wallet channel create <recipient> <amount> <duration>

# Send micro-payment
ippan wallet channel pay <channel_id> <amount>

# View payment channels
ippan wallet channel list

# Close payment channel
ippan wallet channel close <channel_id>
```

## Domain Management

### Registering Domains

```bash
# Register a domain
ippan domain register <name>.ipn

# Register with custom data
ippan domain register <name>.ipn --data "my-website.com"

# Register premium TLD
ippan domain register <name>.m --premium
```

### Managing Domains

```bash
# View domain info
ippan domain info <name>.ipn

# Update domain data
ippan domain update <name>.ipn --data "new-data"

# Transfer domain
ippan domain transfer <name>.ipn <new-owner>

# Renew domain
ippan domain renew <name>.ipn
```

### Domain Operations

```bash
# List your domains
ippan domain list

# Check domain availability
ippan domain check <name>.ipn

# View domain history
ippan domain history <name>.ipn

# Delete domain
ippan domain delete <name>.ipn
```

## API Usage

### REST API

IPPAN provides a REST API for programmatic access:

```bash
# Start API server
ippan api start

# API endpoints:
# GET /api/v1/status - Node status
# GET /api/v1/network - Network information
# GET /api/v1/storage - Storage information
# GET /api/v1/wallet - Wallet information
# POST /api/v1/storage/upload - Upload file
# GET /api/v1/storage/download/{hash} - Download file
# POST /api/v1/wallet/send - Send transaction
# GET /api/v1/domain/{name} - Domain information
```

### Example API Usage

```bash
# Get node status
curl http://localhost:3000/api/v1/status

# Upload a file
curl -X POST http://localhost:3000/api/v1/storage/upload \
  -F "file=@/path/to/file.txt"

# Send a transaction
curl -X POST http://localhost:3000/api/v1/wallet/send \
  -H "Content-Type: application/json" \
  -d '{"to": "address", "amount": 1000}'
```

### WebSocket API

For real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    console.log('Received:', data);
};

// Subscribe to network events
ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'network'
}));
```

## Troubleshooting

### Common Issues

#### Node Won't Start

```bash
# Check if port is in use
netstat -tulpn | grep 8080

# Check configuration
ippan config validate

# Check logs
ippan node logs --level debug
```

#### Network Connection Issues

```bash
# Check network connectivity
ippan network test

# Reset network connections
ippan network reset

# View network diagnostics
ippan network diagnose
```

#### Storage Issues

```bash
# Check storage space
df -h ~/.ippan/storage

# Verify storage integrity
ippan storage verify

# Rebuild storage index
ippan storage rebuild
```

#### Wallet Issues

```bash
# Check wallet status
ippan wallet status

# Verify wallet integrity
ippan wallet verify

# Backup wallet
ippan wallet backup
```

### Getting Help

```bash
# View help for any command
ippan <command> --help

# View detailed help
ippan <command> --help --verbose

# Check IPPAN version
ippan --version

# View system information
ippan system info
```

### Logs and Debugging

```bash
# View real-time logs
ippan logs --follow

# View logs with specific level
ippan logs --level debug

# View logs for specific component
ippan logs --component network

# Export logs to file
ippan logs --output logs.txt
```

### Performance Monitoring

```bash
# View performance metrics
ippan metrics

# Monitor resource usage
ippan monitor

# View system statistics
ippan stats
```

## Advanced Usage

### Running as a Service

```bash
# Create systemd service
sudo tee /etc/systemd/system/ippan.service << EOF
[Unit]
Description=IPPAN Node
After=network.target

[Service]
Type=simple
User=ippan
ExecStart=/usr/local/bin/ippan node start
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl enable ippan
sudo systemctl start ippan
```

### Backup and Recovery

```bash
# Backup wallet
ippan wallet backup --output wallet.backup

# Backup configuration
ippan config backup --output config.backup

# Backup storage data
ippan storage backup --output storage.backup

# Restore from backup
ippan wallet restore wallet.backup
```

### Security Best Practices

1. **Keep Private Keys Secure**: Never share your private keys
2. **Regular Updates**: Keep IPPAN updated to latest version
3. **Firewall Configuration**: Configure firewall to allow IPPAN ports
4. **Monitoring**: Set up monitoring for your node
5. **Backups**: Regular backups of wallet and configuration

## Support and Community

- **Documentation**: [docs.ippan.net](https://docs.ippan.net)
- **GitHub**: [github.com/ippan/ippan](https://github.com/ippan/ippan)
- **Discord**: [discord.gg/ippan](https://discord.gg/ippan)
- **Telegram**: [t.me/ippan](https://t.me/ippan)
- **Email**: support@ippan.net

---

**Happy IPPANing! 🚀** 