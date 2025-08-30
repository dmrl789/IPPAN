# IPPAN - High Performance Blockchain

A Rust implementation of the IPPAN blockchain designed to achieve **10M TPS** throughput.

## Quick Start

### Prerequisites
- Rust 1.70+ (stable)
- 32+ GB RAM (for high-throughput testing)
- Linux/Windows (jemalloc support)

### Build
```bash
cd ippan
cargo build --release
```

### Run Single Node
```bash
# Start node with 4 shards
cargo run --release -p ippan-node -- --http-port 8080 --p2p-port 8081 --shards 4
```

### Wallet Operations
```bash
# Create wallet
cargo run --release -p ippan-wallet-cli -- new mywallet

# Show address
cargo run --release -p ippan-wallet-cli -- addr

# Send transaction
cargo run --release -p ippan-wallet-cli -- send --to i1abc... --amount 1000 --node http://127.0.0.1:8080
```

### Load Testing
```bash
# Generate 1M TPS for 60 seconds
cargo run --release -p ippan-loadgen-cli -- --tps 1000000 --accounts 200000 --duration 60 --nodes http://127.0.0.1:8080
```

### Benchmarks
```bash
# Run performance benchmarks
cargo bench -p ippan-bench
```

### Cluster Management
```bash
# Run a cluster of nodes (Windows PowerShell)
.\run_cluster.ps1 -NodeCount 4 -LogLevel "warn"

# Run tests
.\run_tests.ps1

# Monitor performance
.\monitor_performance.ps1

# Simple test
.\simple_test.ps1

# 10M TPS test
.\test_10m_tps.ps1
```

### API Endpoints
- `GET /health` - Node health status
- `GET /metrics` - Prometheus metrics
- `POST /tx` - Submit transaction (binary/hex)

## Architecture

### Core Components
- **IPPAN Time**: Network median time with 2ms drift guard
- **HashTimers**: Deterministic transaction ordering
- **Mempool**: Sharded priority queues (HashTimer, nonce)
- **Block Builder**: 16KB micro-blocks every 10-50ms
- **Rounds**: 100-250ms cadence with 2f+1 finality
- **P2P**: libp2p with gossipsub, Kademlia, mDNS

### Performance Targets
- **Throughput**: 10M TPS sustained for 300s
- **Latency**: p50 ≤ 350ms, p95 ≤ 600ms
- **Block Size**: 16KB target (max 32KB)
- **Transaction Size**: ≤ 185 bytes

## Development

### Project Structure
```
ippan/
├── Cargo.toml              # Workspace configuration
├── rust-toolchain.toml     # Stable Rust
├── docs/                   # Documentation
├── run_cluster.ps1         # Cluster management script
├── run_tests.ps1           # Test runner script
├── monitor_performance.ps1 # Performance monitoring
├── simple_test.ps1         # Simple test script
├── test_10m_tps.ps1        # 10M TPS test script
└── crates/
    ├── common/             # Shared types & crypto
    ├── node/               # Validator node
    ├── wallet-cli/         # Wallet management
    ├── loadgen-cli/        # Load generator
    └── bench/              # Performance benchmarks
```

### Key Features
- **Lock-free**: Sharded mempool with minimal contention
- **Deterministic**: HashTimer-based ordering
- **Observable**: Prometheus metrics + CSV export
- **Scalable**: CPU-core sharding, batch verification

### Testing
```bash
# Unit tests
cargo test

# Integration tests
cargo test -p ippan-node

# Performance tests
cargo bench
```

## Configuration

### Node Settings
- `--http-port`: HTTP API port (default: 8080)
- `--p2p-port`: P2P port (default: 8081)
- `--shards`: Number of shards (default: 1)
- `--bootstrap-peers`: Initial peer list

### Performance Tuning
- Use `--release` for production builds
- Adjust shard count based on CPU cores
- Monitor memory usage (8-16GB recommended)
- Enable jemalloc for better memory management

## Monitoring

### Metrics
- Transaction ingress/egress rates
- Mempool size and latency
- Block/round statistics
- Network peer count

### Health Checks
```bash
curl http://localhost:8080/health
curl http://localhost:8080/metrics
```

### Docker Support
```bash
# Build and run with Docker
docker-compose up -d

# View logs
docker-compose logs -f
```

## License

 MIT License - see LICENSE file for details.

## Contributing

1. Fork the repository
2. Create feature branch
3. Add tests for new functionality
4. Ensure benchmarks pass
5. Submit pull request

## Support

- Issues: GitHub Issues
- Documentation: `docs/` directory
- Performance: Run `cargo bench` for metrics

## ⚠️ Important Notes

- **Workspace Structure**: This is a Cargo workspace with multiple crates
- **Package Commands**: Use `-p <package-name>` when running cargo commands
- **Scripts**: PowerShell scripts are available for cluster management and testing
- **Docker**: Docker Compose configuration is provided for containerized deployment
