# IPPAN Blockchain

A high-performance, quantum-resistant blockchain with advanced features including BlockDAG consensus, ZK-STARK proofs, and comprehensive security measures.

## Features

- **BlockDAG Consensus**: High-throughput consensus mechanism
- **ZK-STARK Proofs**: Zero-knowledge proofs for privacy and scalability
- **Quantum-Resistant Cryptography**: Future-proof security
- **BFT Consensus**: Byzantine Fault Tolerant consensus
- **Real-time P2P Network**: Efficient peer-to-peer communication
- **Encrypted Storage**: Secure data storage with sharding and replication
- **Comprehensive API**: REST API for all operations
- **Advanced Monitoring**: Real-time monitoring and alerting
- **Production Ready**: Docker, Kubernetes, and CI/CD support
- **Unified UI**: Single, comprehensive web interface for all operations

## Quick Start

### Prerequisites

- Rust 1.70+
- Docker 20.10+
- Protocol Buffers 3.20+
- Node.js 18+ (for frontend)

### Backend Setup

```bash
# Clone repository
git clone https://github.com/dmrl789/IPPAN.git
cd ippan

# Build project
cargo build --release

# Run single node
./target/release/ippan-node --config configs/testnet-node.toml
```

### Frontend Setup

```bash
# Install dependencies
cd apps/unified-ui
npm install

# Start development server
npm run dev

# Build for production
npm run build
```

### Docker Deployment

```bash
# Deploy with Docker Compose
docker-compose -f docker-compose.production.yml up -d

# Check status
docker-compose -f docker-compose.production.yml ps
```

## Unified UI

The repository includes a single, comprehensive web interface at `apps/unified-ui` that provides:

- **Wallet Management**: View balances, send transactions, manage keys
- **Block Explorer**: Browse blocks, transactions, and accounts
- **Network Monitoring**: Real-time network statistics and node status
- **Storage Management**: Upload, manage, and share files
- **Domain Management**: Register and manage DNS domains
- **Staking Interface**: Participate in network consensus
- **AI/ML Marketplace**: Deploy and manage neural models

### Environment Variables

Create a `.env` file in `apps/unified-ui/`:

```bash
# API Configuration
VITE_API_URL=http://localhost:3000

# Optional: Custom API endpoints
VITE_API_BASE_URL=http://your-node-ip:3000
```

### Running the UI

```bash
# Development mode
cd apps/unified-ui
npm run dev

# Production build
npm run build
npm run preview
```

### Kubernetes Deployment

```bash
# Deploy with Helm
helm install ippan-mainnet ippan/ippan \
  --set network.networkId=ippan-mainnet \
  --set node.replicas=5

# Deploy with YAML
kubectl apply -f deployments/kubernetes/ippan-deployment.yaml
```

## Documentation

- [Deployment Guide](docs/DEPLOYMENT_GUIDE.md) - Complete deployment instructions
- [Operational Guide](docs/OPERATIONAL_GUIDE.md) - Day-to-day operations
- [API Reference](docs/API_REFERENCE.md) - REST API documentation
- [Configuration Reference](docs/CONFIGURATION.md) - Configuration options

## Architecture

### Core Components

- **Consensus**: BFT consensus with BlockDAG
- **Network**: P2P network with libp2p
- **Storage**: Encrypted storage with sharding
- **Wallet**: Multi-account wallet with transaction management
- **API**: REST API for all operations
- **Monitoring**: Comprehensive monitoring and alerting

### Network Topology

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Bootstrap     │    │   Validator     │    │   Validator     │
│     Node        │◄──►│     Node        │◄──►│     Node        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Load          │    │   Monitoring    │    │   API Gateway   │
│   Balancer      │    │   System        │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Configuration

### Node Configuration

```toml
[network]
network_id = "ippan-mainnet"
chain_id = "ippan-1"
node_id = "your-unique-node-id"
listen_address = "0.0.0.0:30333"
api_address = "0.0.0.0:8080"

[consensus]
consensus_type = "bft"
block_time = 5
max_block_size = 1048576
finality_threshold = 0.67

[security]
enable_tls = true
enable_encryption = true
rate_limit = 1000
```

## API Usage

### Node Status

```bash
curl http://localhost:8080/api/v1/node/status
```

### Submit Transaction

```bash
curl -X POST http://localhost:8080/api/v1/transactions \
  -H "Content-Type: application/json" \
  -d '{
    "from": "0x1234...",
    "to": "0x5678...",
    "amount": 1000,
    "fee": 10
  }'
```

### Get Block

```bash
curl http://localhost:8080/api/v1/blocks/12345
```

## Monitoring

### Health Checks

```bash
# Overall health
curl http://localhost:8080/health

# Component health
curl http://localhost:8080/health/consensus
curl http://localhost:8080/health/network
curl http://localhost:8080/health/storage
```

### Metrics

```bash
# Prometheus metrics
curl http://localhost:9090/metrics

# Application metrics
curl http://localhost:8080/api/v1/metrics
```

## Development

### Building

```bash
# Build all components
cargo build

# Build with optimizations
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Frontend Development

```bash
# Unified UI
cd apps/unified-ui
npm install
npm run dev

# Wallet UI
cd apps/wallet
npm install
npm run dev
```

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
cargo test --test integration
```

### Performance Tests

```bash
cargo test --test performance
```

### Load Testing

```bash
# Deploy testnet
cd deployments/testnet
docker-compose -f docker-compose.testnet.yml up -d

# Run load test
./scripts/load-test.sh
```

## Security

### Security Audit

```bash
# Run security audit
cargo audit

# Run security scan
cargo deny check

# Run vulnerability scan
trivy fs .
```

### Security Features

- **TLS/SSL Encryption**: All network communications encrypted
- **Authentication**: JWT and API key authentication
- **Rate Limiting**: Configurable rate limits
- **Input Validation**: Comprehensive input validation
- **Access Control**: Role-based access control
- **Audit Logging**: Comprehensive audit trails

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- **Documentation**: https://docs.ippan.net
- **GitHub Issues**: https://github.com/dmrl789/IPPAN/issues
- **Discord**: https://discord.gg/ippan
- **Email**: support@ippan.net

## Roadmap

- [x] Core blockchain implementation
- [x] BFT consensus engine
- [x] P2P network layer
- [x] Encrypted storage system
- [x] Wallet implementation
- [x] REST API
- [x] Monitoring and alerting
- [x] Security audit
- [x] Performance optimization
- [x] Documentation
- [ ] Mainnet deployment
- [ ] Frontend integration
- [ ] Smart contracts
- [ ] Cross-chain bridges
- [ ] Mobile applications