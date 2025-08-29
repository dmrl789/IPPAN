# IPPAN - High Performance Blockchain MVP
 
A high-throughput blockchain implementation designed to validate up to 10M TPS with a focus on performance and scalability.

## 🚀 Features

### Core Components
- **IPPAN Time**: Network median time synchronization with 2ms drift guard
- **HashTimers**: Deterministic transaction ordering mechanism
- **Sharded Mempool**: Multi-shard priority queue for high-throughput transaction processing
- **Micro-blocks**: 16KB blocks built every 10-50ms
- **Round-based Finality**: 100-250ms rounds with VRF-based verifier selection
- **State Management**: In-memory KV store with atomic transaction application
- **P2P Networking**: Simplified network layer for transaction propagation

### Tools
- **Wallet CLI**: Ed25519 keypair management and transaction sending
- **Load Generator**: High-TPS transaction generation for performance testing
- **Benchmark Suite**: Criterion-based performance benchmarks

## 📁 Project Structure

```
ippan/
├── Cargo.toml                 # Workspace manifest
├── rust-toolchain.toml        # Stable Rust toolchain
├── docs/
│   └── IPPAN_Minimal_PRD.md   # Product Requirements Document
└── crates/
    ├── common/                # Shared types and utilities
    ├── node/                  # Validator node implementation
    ├── wallet-cli/            # Wallet management CLI
    ├── loadgen-cli/           # Load generation tool
    └── bench/                 # Performance benchmarks
```

## 🛠️ Quick Start

### Prerequisites
- Rust 1.70+ (stable)
- Windows/Linux/macOS

### Build
```bash
# Clone and build
git clone <repository>
cd ippan
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Start Node
```bash
# Start IPPAN node
cargo run -p ippan-node -- --http-port 8080 --p2p-port 8081 --shards 4

# Check health
curl http://localhost:8080/health

# View metrics
curl http://localhost:8080/metrics
```

### Wallet Operations
```bash
# Create wallet
cargo run -p ippan-wallet-cli -- new --name mywallet --password secret

# Show address
cargo run -p ippan-wallet-cli -- addr --name mywallet

# Send transaction
cargo run -p ippan-wallet-cli -- send --name mywallet --to i... --amount 1000 --node http://localhost:8080
```

### Load Testing
```bash
# Generate load (1000 TPS for 60 seconds)
cargo run -p ippan-loadgen-cli -- --tps 1000 --accounts 100 --duration 60 --nodes http://localhost:8080

# High-performance test
cargo run -p ippan-loadgen-cli -- --tps 10000 --accounts 1000 --duration 300 --output results.csv
```

## 📊 Performance Targets

- **Throughput**: 10M TPS sustained
- **Latency**: P50 ≤ 350ms, P95 ≤ 500ms, P99 ≤ 1s
- **Block Size**: 16KB target (micro-blocks)
- **Block Interval**: 10-50ms
- **Round Duration**: 100-250ms
- **Transaction Size**: ≤ 185 bytes

## 🔧 Configuration

### Node Configuration
- `--http-port`: HTTP API port (default: 8080)
- `--p2p-port`: P2P network port (default: 8081)
- `--shards`: Number of mempool shards (default: 4)
- `--peers`: Bootstrap peer addresses

### Load Generator Configuration
- `--tps`: Target transactions per second
- `--accounts`: Number of test accounts
- `--duration`: Test duration in seconds
- `--nodes`: Comma-separated node URLs
- `--output`: CSV output file for results

## 📈 Monitoring

### Metrics Endpoints
- `GET /health`: Node health status
- `GET /metrics`: Prometheus metrics

### Key Metrics
- `ippan_transactions_received_total`: Total transactions received
- `ippan_transactions_finalized_total`: Total transactions finalized
- `ippan_blocks_built_total`: Total blocks built
- `ippan_rounds_completed_total`: Total rounds completed
- `ippan_mempool_size`: Current mempool size
- `ippan_active_peers`: Number of active peers

## 🧪 Testing

### Unit Tests
```bash
cargo test -p ippan-common
cargo test -p ippan-node
```

### Integration Tests
```bash
# E2E test with single node
cargo test --test e2e_test

# Performance benchmarks
cargo bench -p ippan-bench
```

### Load Testing
```bash
# Validate 10M TPS target
cargo run -p ippan-loadgen-cli -- --tps 10000000 --accounts 10000 --duration 300
```

## 🔒 Security Features

- **Ed25519 Signatures**: Fast, secure digital signatures
- **Batch Verification**: Efficient signature verification
- **HashTimer Ordering**: Deterministic transaction ordering
- **VRF-based Consensus**: Verifiable random function for verifier selection
- **Drift Guard**: 2ms time synchronization guard

## 🚧 MVP Limitations

- **P2P**: Simplified implementation (full libp2p integration planned)
- **Encryption**: Basic wallet encryption (proper KDF planned)
- **Seed Recovery**: Placeholder implementation
- **Account Pre-funding**: Manual process in load generator
- **Round Finality**: Basic 2f+1 signature threshold

## 📝 Development

### Adding Features
1. Follow Rust coding standards
2. Add comprehensive tests
3. Update documentation
4. Run benchmarks to ensure no performance regression

### Performance Optimization
- Use `cargo bench` to measure performance
- Profile with flamegraphs
- Monitor memory usage and GC pressure
- Optimize hot paths identified in benchmarks

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Implement changes with tests
4. Run full test suite
5. Submit pull request

## 📄 License

[License information to be added]

---

**Status**: MVP Complete ✅  
**Target**: 10M TPS validation  
**Next**: Production hardening and full P2P integration
