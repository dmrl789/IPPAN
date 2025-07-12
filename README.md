# IPPAN — Immutable Proof & Availability Network

## Overview
IPPAN is a fully decentralized Layer-1 blockchain with built-in global DHT storage, trustless timestamping, encrypted sharded storage, permissionless staking, M2M payments, and a keyless global fund. Written in Rust, IPPAN is designed to be unstoppable, self-sufficient, and production-ready.

## Architecture
- **BlockDAG Consensus Engine** (HashTimers, IPPAN Time, verifiable randomness)
- **Staking & Validator Selection** (10–100 IPN, slashing, rewards)
- **AES-256 Encrypted, Sharded Storage** (Merkle proofs, spot checks)
- **Global DHT** (key-value, node discovery, replication)
- **Proof-of-Storage & Traffic Tracking**
- **Human-Readable Domains** (handles, premium TLDs, renewals)
- **Keyless Global Fund** (autonomous, weekly distribution)
- **Local Wallet** (Ed25519 keys, staking, rewards)
- **M2M Payment Channels** (IoT/AI, micro-fees)
- **Full RESTful API, CLI, Explorer**

## Status: Production-Ready
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

## Quickstart
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

## Next Steps
- [ ] Comprehensive Testing & Test Suites
- [ ] Performance Optimization & Benchmarking
- [ ] Security Audit & Review
- [ ] Documentation (User & Developer)
- [ ] Production Deployment Preparation
- [ ] Community Contributions & Feedback

## License
MIT
