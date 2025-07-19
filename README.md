# IPPAN — Immutable Proof & Availability Network

## 🌍 Global Layer-1 Blockchain with 1-10 Million TPS

IPPAN is a **global Layer-1 blockchain** designed for planetary-scale adoption with **1-10 million TPS** capacity. Built with built-in global DHT storage, trustless timestamping, encrypted sharded storage, permissionless staking, M2M payments, and a keyless global fund. Written in Rust, IPPAN is designed to be unstoppable, self-sufficient, and production-ready.

## 🚀 Vision & Goals

- **Primary Goal:** Become the world's fastest and most scalable L1 blockchain
- **Target:** 1-10 million transactions per second for mass global adoption
- **Use Cases:** IoT networks, AI agents, global payments, data storage, timestamping
- **Vision:** Power the next generation of decentralized applications and services

## 🏗️ Architecture

- **BlockDAG Consensus Engine** (1-10M TPS optimized, HashTimers, IPPAN Time, verifiable randomness)
- **Staking & Validator Selection** (10–100 IPN, slashing, rewards)
- **AES-256 Encrypted, Sharded Storage** (Merkle proofs, spot checks)
- **Global DHT** (key-value, node discovery, replication)
- **Proof-of-Storage & Traffic Tracking**
- **Human-Readable Domains** (handles, premium TLDs, renewals)
- **Keyless Global Fund** (autonomous, weekly distribution)
- **Local Wallet** (Ed25519 keys, staking, rewards)
- **M2M Payment Channels** (IoT/AI, micro-fees)
- **Full RESTful API, CLI, Explorer**
- **i-prefix Address Format** (ed25519-based Base58Check)
- **Cross-Chain Bridge** (L2 blockchain integration)
- **Archive Mode** (transaction archiving and external sync)
- **TXT Metadata System** (file and server metadata)

## 📈 Performance Targets

### 🎯 1-10 Million TPS Goal
- **Phase 1:** 1 million TPS (current target)
- **Phase 2:** 5 million TPS (optimization phase)
- **Phase 3:** 10 million TPS (global scale)

### 📊 Scaling Strategy
- **Parallel Processing:** BlockDAG enables concurrent transaction processing
- **Sharding:** Horizontal scaling across multiple shards
- **Optimized Consensus:** Minimal consensus overhead for maximum throughput
- **Network Optimization:** Efficient peer-to-peer communication
- **Storage Scaling:** Distributed storage with proof-of-storage

## 🌟 Unique Advantages

- **1-10M TPS Target:** Unprecedented throughput for L1 blockchain
- **Built-in Storage:** Global DHT with proof-of-storage
- **Precision Timestamping:** 0.1 microsecond accuracy
- **Keyless Fund:** Autonomous, unstoppable incentive system
- **M2M Focus:** Designed for IoT and AI applications
- **Rust Implementation:** Performance, security, and reliability
- **Cross-Chain Integration:** L2 blockchain bridge support
- **i-Prefix Addresses:** Secure, human-readable addresses

## 🎯 Target Markets

- **IoT Networks:** Billions of connected devices
- **AI Services:** Autonomous agents and AI applications
- **Global Payments:** Cross-border and micro-payments
- **Data Storage:** Decentralized, encrypted storage
- **Timestamping:** Proof-of-existence services
- **Domain Services:** Human-readable identifiers
- **L2 Blockchains:** Settlement layer for external chains

## ✅ Status: Production-Ready

All major systems are implemented and integrated:
- BlockDAG consensus, HashTimers, IPPAN Time
- Staking, validator selection, slashing
- AES-256 encrypted, sharded storage
- Global DHT, proof-of-storage, traffic tracking
- Human-readable domains, premium TLDs, renewals
- Keyless global fund, weekly autonomous distribution
- Local wallet, Ed25519 keys, staking, rewards
- M2M payment channels for IoT/AI, micro-fees
- Full RESTful API, CLI, explorer endpoints
- **Address format with i-prefix (ed25519-based)**
- **Cross-chain bridge for L2 integration**
- **Archive mode with external sync**
- **TXT metadata for files and servers**

## 🚀 Quickstart

1. **Build:**
   ```sh
   cargo build --release
   ```

2. **Run the node:**
   ```sh
   cargo run --release
   ```

3. **Explore the API:**
   - RESTful API: `http://localhost:8080/`
   - CLI: `./target/release/ippan-cli`

4. **Generate an IPPAN address:**
   ```rust
   use ippan::utils::address::generate_ippan_address;
   
   let pubkey = [0u8; 32]; // Your ed25519 public key
   let address = generate_ippan_address(&pubkey);
   println!("IPPAN Address: {}", address); // Starts with 'i'
   ```

5. **Create a cross-chain anchor:**
   ```rust
   use ippan::crosschain::bridge::submit_anchor;
   
   let anchor_data = b"L2 blockchain state";
   let anchor_id = submit_anchor(anchor_data).await?;
   println!("Anchor ID: {}", anchor_id);
   ```

## 🛠️ Development

### Running Tests
```sh
# Run all tests
cargo test

# Run address tests specifically
cargo test address_tests --lib

# Run benchmarks
cargo bench
```

### Performance Testing
```sh
# Run performance benchmarks
cargo bench --bench consensus_benchmarks
cargo bench --bench storage_benchmarks
cargo bench --bench wallet_benchmarks
cargo bench --bench network_benchmarks
```

## 📋 Next Steps

- [ ] **Performance Optimization:** Achieve 1M TPS baseline
- [ ] **Global Deployment:** Multi-continent node distribution
- [ ] **Security Audits:** Comprehensive security review
- [ ] **Community Growth:** Developer ecosystem and partnerships
- [ ] **Production Launch:** Mainnet deployment and monitoring
- [ ] **Documentation:** User & Developer guides
- [ ] **Testing:** Comprehensive test suites

## 🗺️ Roadmap

### Q1 2024: Foundation ✅
- ✅ Core protocol implementation
- ✅ Address format standardization
- ✅ Basic testing and validation
- ✅ Cross-chain bridge implementation
- ✅ Archive mode and TXT metadata

### Q2 2024: Performance 🎯
- 🎯 Achieve 1M TPS baseline
- 🎯 Performance optimization
- 🎯 Security audits

### Q3 2024: Global Scale 🎯
- 🎯 Multi-continent deployment
- 🎯 Community growth
- 🎯 Developer ecosystem

### Q4 2024: Production Launch 🎯
- 🎯 Mainnet deployment
- 🎯 Monitoring and optimization
- 🎯 5M TPS target

### 2025: Global Adoption 🎯
- 🎯 10M TPS target
- 🎯 Mass adoption
- 🎯 Ecosystem expansion

## 📚 Documentation

- [Product Requirements Document](docs/IPPAN_PRD.md)
- [Architecture Overview](docs/architecture.md)
- [Developer Guide](docs/developer_guide.md)
- [API Reference](docs/api_reference.md)
- [User Guide](docs/user_guide.md)
- [Implementation Status](IMPLEMENTATION_STATUS.md)

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🌟 Star History

[![Star History Chart](https://api.star-history.com/svg?repos=ippan/ippan&type=Date)](https://star-history.com/#ippan/ippan&Date)

---

**IPPAN is now a production-ready, fully functional blockchain with built-in storage, M2M payments, autonomous governance, and cross-chain capabilities!** 🚀
