# IPPAN Executable Binaries

This document provides an overview of all executable binaries available in the IPPAN blockchain project.

## Core Binaries

### ippan-node
**Location:** `node/src/main.rs`  
**Package:** `ippan-node`

The main IPPAN blockchain node binary with full functionality:
- PoA (Proof of Authority) consensus
- DLC (Deterministic Learning Consensus) integration
- P2P networking with libp2p
- Integrated RPC server
- Mempool management
- L2 interoperability support
- Security features

**Build:**
```bash
cargo build --release --bin ippan-node
```

**Run:**
```bash
./target/release/ippan-node
```

**Environment Variables:**
- `IPPAN_VALIDATOR_ID` - Validator ID (hex)
- `IPPAN_RPC_HOST` - RPC host (default: 0.0.0.0)
- `IPPAN_RPC_PORT` - RPC port (default: 8080)
- `IPPAN_P2P_HOST` - P2P host (default: 0.0.0.0)
- `IPPAN_P2P_PORT` - P2P port (default: 9000)
- And many more configuration options...

---

### ippan-check-nodes
**Location:** `node/src/bin/check_nodes.rs`  
**Package:** `ippan-node`

Node health checker and validator utility for monitoring IPPAN nodes.

**Build:**
```bash
cargo build --release --bin ippan-check-nodes
```

**Run:**
```bash
./target/release/ippan-check-nodes --node-url http://localhost:8080
```

---

## Command-Line Tools

### ippan-cli
**Location:** `crates/cli/src/main.rs`  
**Package:** `ippan-cli`

Comprehensive command-line interface for interacting with IPPAN nodes.

**Features:**
- Node operations (status, peers, version, info)
- Wallet operations (balance, send)
- Transaction queries and submission
- Blockchain queries (blocks, stats)
- Validator management

**Build:**
```bash
cargo build --release --bin ippan-cli
```

**Usage Examples:**
```bash
# Get node status
./target/release/ippan-cli node status

# Get latest block
./target/release/ippan-cli query latest-block

# List validators
./target/release/ippan-cli validator list

# Get wallet balance
./target/release/ippan-cli wallet balance <address>

# Send transaction
./target/release/ippan-cli wallet send <from> <to> <amount>

# Get transaction by hash
./target/release/ippan-cli transaction get <hash>
```

**Options:**
- `--rpc-url` - RPC endpoint URL (default: http://localhost:8080)

---

### ippan-keygen
**Location:** `crates/keygen/src/main.rs`  
**Package:** `ippan-keygen`

Secure Ed25519 keypair generation and management tool for validators and wallets.

**Build:**
```bash
cargo build --release --bin ippan-keygen
```

**Usage Examples:**
```bash
# Generate new keypair
./target/release/ippan-keygen generate --output ./keys --name my_validator

# Derive public key from private key
./target/release/ippan-keygen pub-key ./keys/my_validator_private.key

# Verify keypair
./target/release/ippan-keygen verify ./keys/my_validator_private.key ./keys/my_validator_public.key

# Get validator ID from public key
./target/release/ippan-keygen validator-id ./keys/my_validator_public.key
```

**Commands:**
- `generate` - Generate a new Ed25519 keypair
- `pub-key` - Derive public key from private key
- `verify` - Verify keypair validity
- `validator-id` - Compute validator ID from public key

---

### ippan-benchmark
**Location:** `crates/benchmark/src/main.rs`  
**Package:** `ippan-benchmark`

Performance benchmarking tool for measuring node performance, TPS, and latency.

**Build:**
```bash
cargo build --release --bin ippan-benchmark
```

**Usage Examples:**
```bash
# Run TPS benchmark (default)
./target/release/ippan-benchmark --count 1000 --workers 10

# Run latency benchmark
./target/release/ippan-benchmark --test latency --count 500

# Run network benchmark
./target/release/ippan-benchmark --test network

# Run storage benchmark
./target/release/ippan-benchmark --test storage --count 1000

# Run all benchmarks
./target/release/ippan-benchmark --test all
```

**Options:**
- `--rpc-url` - RPC endpoint URL (default: http://localhost:8080)
- `--count` - Number of operations (default: 1000)
- `--workers` - Number of concurrent workers (default: 10)
- `--test` - Test type: tps, latency, network, storage, all (default: tps)
- `--warmup` - Warmup iterations (default: 10)

**Output:**
- TPS (Transactions Per Second)
- Latency percentiles (P50, P95, P99)
- Network statistics
- Storage performance metrics

---

### ippan-explorer
**Location:** `crates/explorer/src/main.rs`  
**Package:** `ippan-explorer`

Block explorer and API gateway providing a RESTful API for blockchain exploration.

**Build:**
```bash
cargo build --release --bin ippan-explorer
```

**Run:**
```bash
./target/release/ippan-explorer --host 0.0.0.0 --port 3000 --node-rpc http://localhost:8080
```

**Options:**
- `--host` - Bind address (default: 0.0.0.0)
- `--port` - Bind port (default: 3000)
- `--node-rpc` - IPPAN node RPC URL (default: http://localhost:8080)

**API Endpoints:**

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | API documentation (HTML) |
| `/api/health` | GET | Health check |
| `/api/blocks?page=1&limit=20` | GET | List blocks |
| `/api/block/:id` | GET | Get block by height or hash |
| `/api/transactions?page=1&limit=20` | GET | List transactions |
| `/api/transaction/:hash` | GET | Get transaction by hash |
| `/api/validators` | GET | List validators |
| `/api/validator/:id` | GET | Get validator info |
| `/api/stats` | GET | Blockchain statistics |
| `/api/node/status` | GET | Node status |
| `/api/node/peers` | GET | Node peers |

**Example Usage:**
```bash
# Query blocks
curl http://localhost:3000/api/blocks

# Get latest block
curl http://localhost:3000/api/block/latest

# Get validator list
curl http://localhost:3000/api/validators

# Check health
curl http://localhost:3000/api/health
```

---

## Wallet Tools

### ippan-wallet
**Location:** `crates/wallet/src/bin/ippan-wallet.rs`  
**Package:** `ippan_wallet`

Wallet CLI interface for managing wallets and transactions.

**Build:**
```bash
cargo build --release --bin ippan-wallet
```

**Run:**
```bash
./target/release/ippan-wallet
```

---

## AI & Testing Tools

### ippan-ai-service
**Location:** `crates/ai_service/src/main.rs`  
**Package:** `ippan-ai-service`

AI service daemon with GBDT inference, metrics collection, and monitoring.

**Build:**
```bash
cargo build --release --bin ippan-ai-service
```

**Run:**
```bash
./target/release/ippan-ai-service
```

**Features:**
- GBDT deterministic inference
- Prometheus metrics export
- JSON metrics export
- Production-ready monitoring

---

### test_deterministic_gbdt
**Location:** `test_gbdt/src/main.rs`  
**Package:** `test_deterministic_gbdt`

GBDT determinism testing tool for validating AI inference reproducibility.

**Build:**
```bash
cargo build --release --bin test_deterministic_gbdt
```

---

## Installation

Install all binaries to your system:

```bash
# Install all binaries
cargo install --path node --bin ippan-node
cargo install --path crates/cli --bin ippan-cli
cargo install --path crates/keygen --bin ippan-keygen
cargo install --path crates/benchmark --bin ippan-benchmark
cargo install --path crates/explorer --bin ippan-explorer
cargo install --path crates/wallet --bin ippan-wallet
cargo install --path crates/ai_service --bin ippan-ai-service
```

Or build all at once:

```bash
cargo build --release --bins
```

Binaries will be located in `./target/release/`

---

## Quick Start Guide

### 1. Generate Keys
```bash
ippan-keygen generate --output ~/.ippan/keys --name validator
```

### 2. Start Node
```bash
ippan-node
```

### 3. Check Node Status
```bash
ippan-cli node status
```

### 4. Start Block Explorer
```bash
ippan-explorer --port 3000
```

### 5. Run Benchmarks
```bash
ippan-benchmark --test all
```

---

## Summary

| Binary | Purpose | Production Ready |
|--------|---------|------------------|
| `ippan-node` | Main blockchain node | ✅ Yes |
| `ippan-check-nodes` | Node health checker | ✅ Yes |
| `ippan-cli` | Command-line interface | ✅ Yes |
| `ippan-keygen` | Key generation | ✅ Yes |
| `ippan-benchmark` | Performance testing | ✅ Yes |
| `ippan-explorer` | Block explorer API | ✅ Yes |
| `ippan-wallet` | Wallet management | ✅ Yes |
| `ippan-ai-service` | AI service daemon | ✅ Yes |
| `test_deterministic_gbdt` | Testing tool | 🧪 Development |

All binaries are implemented and ready for use. The new CLI tools (`ippan-cli`, `ippan-keygen`, `ippan-benchmark`, `ippan-explorer`) significantly enhance the developer and operator experience.
