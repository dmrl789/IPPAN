# 👥 IPPAN User Guide

## 🌍 Global Layer-1 Blockchain for Everyone

Welcome to IPPAN, the **global Layer-1 blockchain** designed for planetary-scale adoption with **1-10 million TPS** capacity. This guide helps you understand and use IPPAN for your needs.

## 🚀 What is IPPAN?

IPPAN (Immutable Proof & Availability Network) is a **global Layer-1 blockchain** that:

- **Processes 1-10 million transactions per second** for mass adoption
- **Provides global storage** with built-in DHT and proof-of-storage
- **Offers precision timestamping** with 0.1 microsecond accuracy
- **Enables M2M payments** for IoT devices and AI agents
- **Features human-readable domains** like `@alice.ipn`
- **Runs autonomously** with a keyless global fund

## 🎯 Key Features

### 🌟 High Performance
- **1-10M TPS:** Unprecedented throughput for global scale
- **Low Latency:** Fast transaction confirmation
- **Global Distribution:** Nodes across all continents
- **Parallel Processing:** Concurrent transaction handling

### 🔐 Security & Reliability
- **Ed25519 Cryptography:** Secure digital signatures
- **AES-256 Encryption:** Encrypted file storage
- **ZK-STARK Proofs:** Sub-second finality with cryptographic guarantees
- **Proof-of-Storage:** Verifiable data availability
- **Byzantine Fault Tolerance:** Survives malicious nodes

### 💰 Economic Model
- **IPN Token:** Native cryptocurrency (21M max supply)
- **Staking Rewards:** Earn by running nodes
- **1% Transaction Fees:** Automatic fee collection
- **Keyless Global Fund:** Autonomous reward distribution

## 🏗️ Getting Started

### 1. **Install IPPAN**

```bash
# Clone the repository
git clone https://github.com/ippan/ippan.git
cd ippan

# Build the project
cargo build --release

# Run a node
cargo run --release
```

### 2. **Generate Your First Address**

```rust
use ippan::utils::address::generate_ippan_address;
use ed25519_dalek::SigningKey;

// Generate a new keypair
let mut rng = rand::thread_rng();
let signing_key = SigningKey::generate(&mut rng);
let verifying_key = signing_key.verifying_key();

// Generate your IPPAN address
let address = generate_ippan_address(&verifying_key.to_bytes());
println!("Your IPPAN address: {}", address);
// Output: i1hV6Ro8Adgj7fw1MPWAhUHyZBcZevfyz
```

### 3. **Connect to the Network**

```bash
# Start your node
./target/release/ippan

# Check node status
curl http://localhost:8080/api/v1/status

# View network peers
curl http://localhost:8080/api/v1/peers
```

## 💳 Using IPPAN

### **Making Transactions**

```rust
use ippan::wallet::payments::create_payment_transaction;

// Create a payment transaction
let transaction = create_payment_transaction(
    &from_address,
    &to_address,
    amount_in_satoshi,
    &signing_key
)?;

// Submit to network
network.broadcast_transaction(&transaction).await?;
```

### **Storing Files**

```rust
use ippan::storage::orchestrator::upload_file;

// Upload a file to IPPAN storage
let file_hash = upload_file("document.txt", &file_data).await?;
println!("File stored with hash: {}", file_hash);

// Download the file
let downloaded_data = download_file(&file_hash).await?;
```

### **Registering Domains**

```rust
use ippan::domain::registry::register_domain;

// Register a human-readable domain
let domain = register_domain("@alice.ipn", &address).await?;
println!("Domain registered: {}", domain);
```

## Using TXT Metadata in IPPAN

### **Overview**
- **TXT Metadata:** Allows users to publish signed text entries for files and servers.
- **Use Cases:**
  - **Files:** Add descriptions to content like PDFs and media.
  - **Servers:** Announce services such as API endpoints.

### **How to Use**
- **Publishing TXT Records:**
  - Use the CLI command `ipn txt publish` to create a new TXT record.
- **Viewing TXT Records:**
  - Use the CLI command `ipn txt list @handle` to view records for a handle.
- **GUI Integration:**
  - View file descriptions and server info directly in the IPPAN interface.

### **Technical Details**
- **Signature and Timestamp:**
  - Each TXT record is signed by the handle's owner and timestamped using HashTimer.
- **Discovery:**
  - TXT records are discoverable in IPNDHT and can be optionally anchored on-chain.

## Using Archive Mode in IPPAN

### **Overview**
- **Archive Mode:** Allows nodes to retain validated transactions and sync them to external endpoints, enhancing transparency and robustness.

### **How to Use**
- **Enabling Archive Mode:**
  - Configure archive mode in `node_config.rs` with desired sync targets and intervals.
- **Managing Archive Mode:**
  - Use CLI commands `ipn archive status` and `ipn archive push-now` to manage archive operations.

### **Technical Details**
- **Local Archive Store:**
  - Stores validated transactions, file manifests, TXT records, and zk-STARK proofs using RocksDB.
- **Sync Uploader:**
  - Periodically syncs transactions to configured external APIs.
- **API Specification:**
  - Transactions are received and validated at external endpoints as specified in `api_spec.md`.

## 🌐 Network Participation

### **Running a Node**

#### Minimum Requirements
- **CPU:** 4+ cores (8+ recommended)
- **RAM:** 8GB (16GB recommended)
- **Storage:** 100GB SSD (1TB recommended)
- **Network:** 100 Mbps (1 Gbps recommended)
- **Stake:** 10-100 IPN after first month

#### Setup Steps

1. **Install Dependencies**
   ```bash
   # Ubuntu/Debian
   sudo apt update
   sudo apt install build-essential curl git
   
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Build and Run**
   ```bash
   git clone https://github.com/ippan/ippan.git
   cd ippan
   cargo build --release
   ./target/release/ippan
   ```

3. **Configure Staking**
   ```bash
   # After first month, stake IPN tokens
   ippan-cli stake --amount 50 --address your_address
   ```

### **Earning Rewards**

Nodes earn rewards from the Global Fund for:
- **Uptime:** Maintaining 99%+ availability
- **Validation:** Correctly validating blocks and transactions
- **Time Precision:** Providing accurate IPPAN Time
- **Storage:** Proving file availability
- **Traffic:** Serving real file requests

## 🔧 Advanced Usage

### **M2M Payments**

```rust
// IoT device making micro-payment
let micro_payment = create_micro_payment(
    &device_address,
    &service_address,
    amount_in_smallest_units
).await?;

// AI agent payment
let ai_payment = create_ai_payment(
    &ai_agent_address,
    &compute_provider_address,
    compute_cost
).await?;
```

### **Precision Timestamping**

```rust
use ippan::consensus::hashtimer::create_hashtimer;

// Create a HashTimer for proof-of-existence
let hashtimer = create_hashtimer(
    &data_hash,
    &node_id
)?;

println!("Timestamp: {} ns", hashtimer.ippan_time_ns);
```

### **ZK-STARK Round Verification**

```rust
use ippan::consensus::roundchain::zk_prover::verify_round_proof;

// Verify ZK-STARK proof for a round
let is_valid = verify_round_proof(&round_header, &zk_proof).await?;

if is_valid {
    println!("Round verified with ZK-STARK proof");
    println!("Finality achieved in {} ms", verification_time);
} else {
    println!("Round verification failed");
}
```

### **L2 Blockchain Integration**

```rust
use ippan::crosschain::l2_integration::L2Integration;

// Initialize L2 integration
let l2_integration = L2Integration::new(l2_chain_id)?;

// Submit L2 settlement to IPPAN
let settlement_tx = l2_integration.submit_settlement(
    l2_block_hash,
    l2_state_root,
    settlement_amount
).await?;

println!("L2 settlement submitted: {}", settlement_tx);

// Store L2 data on IPPAN
let data_tx = l2_integration.store_data(
    l2_data,
    L2DataType::StateUpdate
).await?;

println!("L2 data stored: {}", data_tx);
```

### **Storage Proofs**

```rust
use ippan::storage::proofs::generate_storage_proof;

// Generate proof that file is stored
let proof = generate_storage_proof(&file_hash).await?;

// Verify storage proof
let is_valid = verify_storage_proof(&file_hash, &proof).await?;
```

## 📊 Technical Specifications

### **Block & Transaction Details**

#### **Block Structure**
- **Max Block Size:** 10 MB
- **Max Transactions per Block:** 100,000
- **Block Header:** 256 bytes
- **ZK-STARK Proof:** 50-100 KB per round
- **Round Duration:** 1-5 seconds

#### **Transaction Structure**
- **Base Size:** 145 bytes
- **Ed25519 Signature:** 64 bytes
- **Public Key:** 32 bytes
- **Transaction Type:** 1 byte
- **Amount:** 8 bytes (IPN in satoshis)
- **Timestamp:** 8 bytes
- **HashTimer:** 32 bytes
- **Variable Data:** 0-500 bytes

#### **Transaction Types**
- **Payment (0x01):** Standard IPN transfer
- **Storage (0x02):** File upload/download operations
- **Domain (0x03):** Domain registration/renewal
- **Staking (0x04):** Stake/unstake operations
- **Anchor (0x05):** Cross-chain anchor transactions
- **M2M (0x06):** Machine-to-machine payments
- **L2 Settlement (0x07):** L2 blockchain settlement transactions
- **L2 Data (0x08):** L2 data availability and storage

#### **ZK-STARK Performance**
- **Proof Generation:** 1-5 seconds per round
- **Proof Verification:** 10-50ms
- **Proof Size:** 50-100 KB
- **Security Level:** 128-256 bit
- **Finality:** Sub-second deterministic

### **Network Performance**
- **Target TPS:** 1-10 million transactions/second
- **Block Time:** 1-5 seconds
- **Finality:** Sub-second with ZK-STARK
- **Latency:** <100ms intercontinental
- **Throughput:** 10MB blocks with 100K transactions

## 📊 Monitoring & Analytics

### **Node Dashboard**

Access your node's dashboard at `http://localhost:8080/dashboard`:

- **Performance Metrics:** TPS, latency, memory usage
- **Network Status:** Peer connections, sync status
- **Economic Data:** Staking, rewards, fees
- **Storage Stats:** File availability, proof generation

### **Network Explorer**

Visit the IPPAN Explorer at `https://explorer.ippan.io`:

- **Global Network:** View all nodes and their status
- **Transaction History:** Search and verify transactions
- **Address Lookup:** Check balances and transaction history
- **Domain Registry:** Browse registered domains

### **API Access**

```bash
# Get network statistics
curl https://api.ippan.io/v1/network/stats

# Check address balance
curl https://api.ippan.io/v1/address/i1hV6Ro8Adgj7fw1MPWAhUHyZBcZevfyz

# Get transaction details
curl https://api.ippan.io/v1/tx/transaction_hash
```

## 🛠️ Troubleshooting

### **Common Issues**

#### Node Won't Start
```bash
# Check system requirements
free -h  # Check RAM
df -h    # Check disk space
nproc    # Check CPU cores

# Check network connectivity
ping 8.8.8.8
curl -I https://api.ippan.io
```

#### Low Performance
```bash
# Check system resources
htop
iotop
nethogs

# Optimize settings
# Edit config/default.toml
# Increase memory limits
# Optimize network settings
```

#### Sync Issues
```bash
# Check peer connections
curl http://localhost:8080/api/v1/peers

# Restart sync
ippan-cli sync --restart

# Check logs
tail -f logs/ippan.log
```

### **Performance Optimization**

#### For High TPS
- **Use SSD storage** for faster I/O
- **Increase RAM** for better caching
- **Optimize network** with low-latency connections
- **Use multiple CPU cores** for parallel processing

#### For Global Distribution
- **Deploy across continents** for low latency
- **Use CDN** for static content delivery
- **Implement load balancing** for traffic distribution
- **Monitor geographic performance** metrics

## 🔮 Future Features

### **Phase 1: 1M TPS (Q2 2024)**
- ✅ Core protocol implementation
- ✅ Basic optimization
- 🎯 Achieve 1M TPS baseline

### **Phase 2: 5M TPS (Q4 2024)**
- 🎯 Advanced sharding
- 🎯 Network optimization
- 🎯 Global deployment

### **Phase 3: 10M TPS (2025)**
- 🎯 Global scale optimization
- 🎯 Mass adoption features
- 🎯 Ecosystem expansion

## 📚 Additional Resources

### **Documentation**
- [Product Requirements Document](IPPAN_PRD.md)
- [Architecture Overview](architecture.md)
- [Developer Guide](developer_guide.md)
- [API Reference](api_reference.md)

### **Community**
- **Discord:** [Join our community](https://discord.gg/ippan)
- **GitHub:** [Contribute to IPPAN](https://github.com/ippan/ippan)
- **Twitter:** [Follow updates](https://twitter.com/ippan_network)
- **Blog:** [Read latest news](https://blog.ippan.io)

### **Support**
- **Documentation:** [docs.ippan.io](https://docs.ippan.io)
- **FAQ:** [faq.ippan.io](https://faq.ippan.io)
- **Support:** [support@ippan.io](mailto:support@ippan.io)

## 🎉 Getting Help

If you need assistance:

1. **Check the documentation** first
2. **Search existing issues** on GitHub
3. **Ask in Discord** for community help
4. **Create an issue** for bugs or feature requests
5. **Contact support** for urgent issues

Welcome to the future of global blockchain technology! 🌍🚀 