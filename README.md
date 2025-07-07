# IPPAN (Immutable Proof & Availability Network)

A fully decentralized Layer-1 blockchain with built-in global DHT storage, written in Rust.

## Overview

IPPAN is an unstoppable protocol for:
- Proving when any data existed, with **tenth-of-a-microsecond precision**
- Keeping data available through trustless, incentivized storage
- Enabling direct, unstoppable M2M payments and AI services
- Running independently, even in catastrophic scenarios

## Core Features

### ✅ HashTimers & IPPAN Time
- Every block, transaction, and file anchor includes a **HashTimer**
- HashTimers embed **IPPAN Time**, calculated as the median time of node clocks
- Precision: **0.1 microsecond**

### ✅ BlockDAG & Simple Round Consensus
- Blocks are connected in a Directed Acyclic Graph (DAG)
- Rounds have a simple linear structure for consensus coordination
- Deterministic ordering via HashTimers
- Validators selected by **verifiable randomness**

### ✅ Staking & Node Rules
- Nodes are permissionless for the first month
- After 1 month, each node must stake **10–100 IPN**
- Stake can be slashed for downtime, fake proofs, or malicious behavior

### ✅ Native Token (IPN)
- **Ticker:** IPN
- **Max Supply:** 21,000,000 IPN (Bitcoin-style, 0 at genesis, halving schedule)
- **Subdivision:** 1 IPN = 100,000,000 satoshi-like units

### ✅ Fees & Global Fund
- **1% fee** on every transaction → goes to the Global Fund
- `@handle.ipn` domain names have annual fees → also to the Global Fund
- Keyless Global Fund: **no private keys**, cannot be seized or misused
- Weekly auto-distribution to nodes based on performance

### ✅ Encrypted, Sharded Storage
- Files are AES-256 encrypted, sharded, auto-balanced across nodes
- Built-in global DHT maps which nodes hold shards
- Proof-of-Storage via Merkle trees & spot checks

### ✅ Human-Readable Domains
- Users, devices, and AI agents can register handles like `@alice.ipn`
- Premium TLDs possible (`.m`, `.cyborg`, `.humanoid`)
- Annual fees fund the Global Fund

### ✅ Machine-to-Machine (M2M) Payments
- Micro-payments possible in smallest IPN units
- Perfect for IoT devices and autonomous AI agents
- Every M2M payment pays the 1% micro-fee to the Global Fund

## Project Structure

```
src/
├── api/                    # API interfaces (HTTP, CLI, Explorer)
├── consensus/              # BlockDAG, HashTimer, IPPAN Time, Randomness, Rounds
├── dht/                    # Distributed Hash Table (Routing, Discovery, Lookup, Replication)
├── domain/                 # Domain management (Registry, Renewals, Premium, Fees)
├── network/                # P2P networking (Discovery, NAT, Relay, P2P)
├── storage/                # Encrypted storage (Encryption, Shards, Proofs, Traffic, Orchestrator)
├── staking/                # Staking system (Manager, Rewards, Stake Pool)
├── utils/                  # Utilities (Crypto, Time, Logging, Config)
├── wallet/                 # Wallet functionality (Ed25519, Payments, Stake)
├── tests/                  # Comprehensive test suites
├── config.rs               # Configuration management
├── error.rs                # Error handling
├── lib.rs                  # Library exports
├── main.rs                 # Application entry point
└── node.rs                 # Main IPPAN node implementation
```

## Implemented Modules

### Core Node Components
- **IppanNode**: Main node orchestrator that coordinates all components
- **StorageManager**: Unified interface for encrypted, sharded storage
- **WalletManager**: Complete wallet functionality with payments and staking
- **StakingManager**: Stake management, rewards distribution, and pool operations

### Consensus System
- **BlockDAG**: Directed Acyclic Graph for block storage and validation
- **HashTimer**: Precise timing with 0.1 microsecond precision
- **IPPAN Time**: Synchronized time across the network
- **RandomnessEngine**: Verifiable randomness for validator selection
- **RoundManager**: Simple linear round consensus management

### Storage System
- **StorageEncryption**: AES-256-GCM encryption/decryption
- **ShardManager**: File sharding and reconstruction
- **ProofManager**: Storage proofs via Merkle trees
- **TrafficManager**: Bandwidth and traffic tracking
- **StorageOrchestrator**: Coordinated storage operations

### DHT System
- **DHTRouter**: Distributed hash table routing
- **DiscoveryManager**: Node discovery and peer management
- **LookupManager**: Key-value lookups
- **ReplicationManager**: Data replication across nodes

### Network Layer
- **NetworkManager**: P2P network coordination
- **P2PManager**: Peer-to-peer communication
- **DiscoveryManager**: Network node discovery
- **NATManager**: Network address translation handling
- **RelayManager**: Network relay functionality

### Domain System
- **DomainManager**: Domain registration and management
- **RenewalManager**: Domain renewal processing
- **PremiumManager**: Premium domain features
- **FeesManager**: Domain fee calculation and collection

### Wallet System
- **Ed25519Manager**: Cryptographic key management
- **PaymentManager**: Payment processing and validation
- **StakeManager**: Stake operations within wallet

### API Layer
- **ApiServer**: HTTP API server
- **CLI**: Command-line interface
- **Explorer**: Blockchain explorer functionality

### Utilities
- **CryptoUtils**: Cryptographic operations (hashing, signing, encryption)
- **TimeUtils**: Time management and formatting
- **LoggingUtils**: Structured logging
- **ConfigUtils**: Configuration management

## Testing

Comprehensive test suites are implemented for all major components:

- **Consensus Tests**: BlockDAG, HashTimer, IPPAN Time, randomness, rounds
- **Storage Tests**: Encryption, sharding, proofs, traffic, orchestration
- **DHT Tests**: Routing, discovery, lookup, replication
- **Rewards Tests**: Reward calculation, performance metrics, distribution

## Configuration

The system uses a comprehensive configuration system with support for:

- Node configuration
- Network settings
- Storage parameters
- Consensus parameters
- API settings
- Logging configuration
- Database settings

## Error Handling

Robust error handling with specific error types for:

- Configuration errors
- Network errors
- Consensus errors
- Storage errors
- DHT errors
- Wallet errors
- Staking errors
- Domain errors
- API errors
- Cryptographic errors
- Validation errors
- Timeout errors

## Dependencies

### Core Dependencies
- **tokio**: Async runtime
- **serde**: Serialization
- **ed25519-dalek**: Cryptography
- **libp2p**: P2P networking
- **sled/rocksdb**: Database storage

### Networking
- **libp2p-core**: Core networking
- **libp2p-swarm**: Network swarm management
- **libp2p-kad**: Kademlia DHT
- **libp2p-noise**: Secure communication
- **libp2p-yamux**: Multiplexing

### Cryptography
- **sha2**: Hashing
- **aes**: Encryption
- **aes-gcm**: Authenticated encryption
- **rand**: Random number generation

### Utilities
- **chrono**: Time handling
- **uuid**: Unique identifiers
- **hex**: Hexadecimal encoding
- **tracing**: Logging
- **config**: Configuration management

### API
- **axum**: HTTP framework
- **tower**: HTTP middleware
- **clap**: Command-line argument parsing

## Building and Running

### Prerequisites
- Rust 1.70+ with Cargo
- Protobuf compiler (protoc)
- LLVM/Clang (for bindgen)

### Build
```bash
cargo build --release
```

### Run
```bash
cargo run
```

### Test
```bash
cargo test
```

## Architecture Highlights

### Deterministic Ordering
- All events are ordered using HashTimers with IPPAN Time
- 0.1 microsecond precision ensures global consistency
- Median time calculation prevents clock manipulation

### Trustless Storage
- AES-256-GCM encryption for all stored data
- Automatic sharding with configurable redundancy
- Merkle tree proofs for storage verification
- Spot checks to ensure data availability

### Incentivized Participation
- Staking requirements (10-100 IPN after first month)
- Performance-based rewards
- Automatic slashing for malicious behavior
- Global Fund distribution based on uptime and contribution

### Scalable Consensus
- BlockDAG allows parallel block creation
- Simple linear rounds for consensus coordination
- Verifiable randomness for validator selection
- Efficient finalization through DAG structure

## Security Features

- **Cryptographic Security**: Ed25519 signatures, AES-256-GCM encryption
- **Economic Security**: Staking with slashing conditions
- **Network Security**: P2P networking with NAT traversal
- **Storage Security**: Encrypted sharding with proofs
- **Time Security**: HashTimers with IPPAN Time synchronization

## Performance Characteristics

- **High Throughput**: BlockDAG allows parallel processing
- **Low Latency**: Optimized networking and consensus
- **Scalable Storage**: Distributed sharding with automatic rebalancing
- **Efficient Lookups**: Kademlia DHT for O(log n) lookups
- **Fast Finalization**: DAG-based consensus with simple round coordination

## Future Enhancements

- **Layer 2 Solutions**: Payment channels and state channels
- **Cross-Chain Bridges**: Interoperability with other blockchains
- **Advanced AI Integration**: Autonomous agent support
- **IoT Optimization**: Lightweight client implementations
- **Governance System**: On-chain governance mechanisms

## Contributing

This is a comprehensive implementation of the IPPAN protocol. The codebase follows Rust best practices with:

- Comprehensive error handling
- Extensive test coverage
- Clear documentation
- Modular architecture
- Async/await patterns
- Memory safety guarantees

## License

MIT License - see LICENSE file for details.

## Disclaimer

This is a reference implementation of the IPPAN protocol. For production use, additional security audits and optimizations may be required.
