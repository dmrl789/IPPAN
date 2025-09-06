# L2-on-Top Architecture Documentation

## Overview

IPPAN implements a **minimal L1, smart contracts on L2** architecture that keeps the base layer ultra-fast while enabling arbitrary programmability. This document describes the complete L2 system implementation, including architecture, components, API, and usage.

## 🎯 Design Philosophy

### Core Principles
- **L1 = Deterministic Core:** Focus on payments, handles, DHT, validator operations
- **L2 = Smart Contracts:** Enable arbitrary programmability via ZK rollups, optimistic rollups, and app-chains
- **L1 Only Verifies:** Verify succinct proofs/commitments and enable exits
- **Performance First:** Maintain L1's 1-10M TPS target while adding L2 capabilities

### Benefits
- **Scalability:** L2s can handle unlimited smart contract complexity
- **Performance:** L1 remains ultra-fast for core operations
- **Flexibility:** Multiple proof systems and DA modes supported
- **Security:** L1 provides settlement guarantees and exit mechanisms

## 🏗️ Architecture Components

### 1. L2 Transaction Types

#### L2CommitTx
```rust
pub struct L2CommitTx {
    pub l2_id: String,           // L2 network identifier
    pub epoch: u64,              // L2 epoch/batch number
    pub state_root: [u8; 32],    // Merkle root after applying batch
    pub da_hash: [u8; 32],       // Data availability hash
    pub proof_type: ProofType,    // ZK, optimistic, or external
    pub proof: Vec<u8>,          // Proof bytes
    pub inline_data: Option<Vec<u8>>, // Optional inline DA data
}
```

#### L2ExitTx
```rust
pub struct L2ExitTx {
    pub l2_id: String,                    // L2 network identifier
    pub epoch: u64,                       // L2 epoch number
    pub proof_of_inclusion: Vec<u8>,      // Merkle/zk membership proof
    pub account: [u8; 32],                // Recipient on L1
    pub amount: u128,                     // Amount to exit
    pub nonce: u64,                       // Nonce to prevent replay
}
```

### 2. Proof Types

#### ZkGroth16
- **Type:** Zero-knowledge proofs
- **Finality:** Instant (no challenge window)
- **Use Case:** High-value, privacy-sensitive applications
- **Requirements:** `zk-groth16` feature flag enabled

#### Optimistic
- **Type:** Optimistic rollups
- **Finality:** After challenge window expires
- **Use Case:** High-throughput, cost-effective applications
- **Challenge Window:** Configurable (default: 60 seconds)

#### External
- **Type:** External attestations
- **Finality:** Based on external verification
- **Use Case:** Hybrid systems, off-chain computation
- **Verification:** External validators/attestations

### 3. Data Availability Modes

#### Inline DA
- **Description:** Store data directly in L1 transaction
- **Pros:** Immediate availability, no external dependencies
- **Cons:** Higher L1 costs, limited by block size
- **Max Size:** Configurable (default: 16 KB)

#### External DA
- **Description:** Store only hash reference on L1
- **Pros:** Lower L1 costs, unlimited data size
- **Cons:** Requires external data availability
- **Use Case:** Large data sets, cost-sensitive applications

## 🔧 Implementation Details

### 1. L2 Registry

The L2 registry manages all registered L2 networks and their parameters:

```rust
pub struct L2Registry {
    config: L2RegistryConfig,
    table: Arc<RwLock<HashMap<String, L2RegistryEntry>>>,
}

pub struct L2RegistryConfig {
    pub max_l2_count: usize,              // Maximum L2s (default: 100)
    pub default_max_commit_size: usize,   // Default commit size (default: 16 KB)
    pub default_min_epoch_gap_ms: u64,    // Min time between epochs (default: 250ms)
    pub default_challenge_window_ms: u64, // Challenge window (default: 60s)
}
```

#### Registry Operations
- **Register L2:** Create new L2 network with parameters
- **Update Parameters:** Modify L2 configuration
- **Deregister:** Remove L2 network
- **Get Statistics:** Track commits, exits, and performance

### 2. Verification System

#### L2Verifier Trait
```rust
#[async_trait]
pub trait L2Verifier {
    async fn verify_commit(&self, tx: &L2CommitTx, registry: &L2Registry) -> Result<(), L2ValidationError>;
    async fn verify_exit(&self, tx: &L2ExitTx, registry: &L2Registry) -> Result<(), L2ValidationError>;
}
```

#### Default Implementation
- **ZK Verification:** Parse and verify Groth16 proofs (when feature enabled)
- **Optimistic Verification:** Check challenge window and basic validity
- **External Verification:** Assume off-chain attestation verified

### 3. Consensus Integration

#### Transaction Validation
- **Size Checks:** Enforce maximum commit size limits
- **Rate Limiting:** Prevent epoch regression and rapid commits
- **Proof Verification:** Call appropriate verifier based on proof type
- **Block Limits:** Per-block caps for L2 operations

#### State Management
- **State Roots:** Track L2 state roots in consensus state
- **Epoch Tracking:** Maintain epoch monotonicity per L2
- **Event Emission:** Generate anchor events for successful commits

## 📡 API Reference

### REST Endpoints

#### Register L2 Network
```http
POST /v1/l2/register
Content-Type: application/json

{
  "l2_id": "rollup-1",
  "proof_type": "zk-groth16",
  "da_mode": "external",
  "challenge_window_ms": 60000,
  "max_commit_size": 16384,
  "min_epoch_gap_ms": 250
}
```

#### Submit L2 Commit
```http
POST /v1/l2/commit
Content-Type: application/json

{
  "l2_id": "rollup-1",
  "epoch": 1,
  "state_root": "0x1234...",
  "da_hash": "0x5678...",
  "proof_type": "zk-groth16",
  "proof": "0xabcd...",
  "inline_data": null
}
```

#### Submit L2 Exit
```http
POST /v1/l2/exit
Content-Type: application/json

{
  "l2_id": "rollup-1",
  "epoch": 1,
  "proof_of_inclusion": "0xefgh...",
  "account": "0xijkl...",
  "amount": 1000,
  "nonce": 1
}
```

#### Get L2 Status
```http
GET /v1/l2/rollup-1/status
```

#### List All L2s
```http
GET /v1/l2
```

### Response Formats

#### L2 Status Response
```json
{
  "success": true,
  "data": {
    "l2_id": "rollup-1",
    "params": {
      "proof_type": "zk-groth16",
      "da_mode": "external",
      "challenge_window_ms": 60000,
      "max_commit_size": 16384,
      "min_epoch_gap_ms": 250
    },
    "status": {
      "last_epoch": 5,
      "last_commit_at": 1640995200000,
      "total_commits": 5,
      "total_exits": 2
    }
  }
}
```

## 🖥️ CLI Usage

### Command Structure
```bash
ippan-cli l2 <command> [options]
```

### Available Commands

#### Register L2 Network
```bash
ippan-cli l2 register \
  --id rollup-1 \
  --proof-type zk-groth16 \
  --da external \
  --challenge-window-ms 60000 \
  --max-commit-size 16384 \
  --min-epoch-gap-ms 250
```

#### Submit Commit
```bash
ippan-cli l2 commit \
  --id rollup-1 \
  --epoch 1 \
  --state-root 0x1234567890abcdef... \
  --da-hash 0xfedcba0987654321... \
  --proof 0xabcd1234efgh5678...
```

#### Submit Exit
```bash
ippan-cli l2 exit \
  --id rollup-1 \
  --epoch 1 \
  --account 0x1234567890abcdef... \
  --amount 1000 \
  --nonce 1 \
  --proof 0xabcd1234efgh5678...
```

#### Check Status
```bash
ippan-cli l2 status --id rollup-1
```

#### List All L2s
```bash
ippan-cli l2 list
```

## ⚙️ Configuration

### Default Configuration
```toml
[l2]
max_commit_size = 16384             # 16 KB
min_epoch_gap_ms = 250              # per l2_id
challenge_window_ms = 60000         # optimistic default
da_mode = "external"                # or "inline"
max_l2_count = 100                  # maximum L2s that can be registered
```

### Feature Flags
```toml
[features]
default = ["crosschain"]
zk-groth16 = ["ark-std", "ark-ec", "ark-groth16", "ark-bn254"]
zk-plonk = []
optimistic = []
```

## 🔒 Security & Validation

### Validation Rules

#### Commit Validation
- **Epoch Monotonicity:** New epoch must be > last epoch
- **Rate Limiting:** Minimum time between commits per L2
- **Size Limits:** Proof size within configured limits
- **Proof Verification:** Valid proof for specified type

#### Exit Validation
- **Inclusion Proof:** Valid proof of inclusion in L2 state
- **Epoch Consistency:** Exit epoch must match committed state
- **Nonce Uniqueness:** Prevent replay attacks
- **Amount Validation:** Sufficient balance in L2 state

### Rate Limiting
- **Per-L2 Limits:** Configurable minimum time between commits
- **Global Limits:** Maximum L2s and operations per block
- **Spam Prevention:** Micro-fees and size restrictions

## 📊 Monitoring & Metrics

### Prometheus Metrics
```
# L2 Operations
ippan_l2_commits_total{l2_id="rollup-1"}
ippan_l2_exits_total{l2_id="rollup-1"}
ippan_l2_commit_bytes_sum{l2_id="rollup-1"}
ippan_l2_rejected_total{reason="epoch_regression"}

# L2 Registry
ippan_l2_registered_total
ippan_l2_active_total
```

### Logging
- **Structured Logs:** JSON format with consistent fields
- **Event Tracking:** All L2 operations logged with context
- **Error Reporting:** Detailed error information for debugging

## 🧪 Testing

### Test Coverage
The L2 system includes comprehensive tests covering:

- **L2 Commit Validation:** Valid and invalid commits
- **L2 Exit Validation:** Exit flow and proof verification
- **Epoch Monotonicity:** Prevent epoch regression
- **Rate Limiting:** Enforce time-based limits
- **Challenge Windows:** Optimistic rollup challenge periods
- **Anchor Events:** Event emission and tracking
- **L2 Verifier:** Proof verification logic
- **Registry Configuration:** L2 limits and parameters

### Running Tests
```bash
# Run all L2 tests
cargo test --test l2_commit_exit

# Run specific test
cargo test --test l2_commit_exit test_l2_commit_validation
```

## 🚀 Deployment & Operations

### Prerequisites
- Rust 1.70+ with Cargo
- IPPAN node with `crosschain` feature enabled
- Optional: ZK proof backend dependencies

### Feature Flags
```bash
# Basic L2 support
cargo build --features crosschain

# With ZK-Groth16 support
cargo build --features "crosschain zk-groth16"

# With all proof backends
cargo build --features "crosschain zk-groth16 zk-plonk optimistic"
```

### Configuration
1. **Enable L2:** Ensure `crosschain` feature is enabled
2. **Configure Limits:** Set appropriate L2 limits in `config/default.toml`
3. **Set Parameters:** Configure default L2 parameters
4. **Monitor:** Set up metrics and logging

## 🔮 Future Enhancements

### Planned Features
- **Advanced Proof Systems:** PLONK, STARK, recursive proofs
- **Cross-L2 Communication:** Inter-L2 messaging and asset transfers
- **Dynamic Parameters:** Runtime L2 parameter updates
- **Enhanced DA:** Multiple DA providers and fallback mechanisms
- **L2 Governance:** On-chain L2 parameter governance

### Integration Opportunities
- **Ethereum L2s:** Integrate with existing Ethereum L2 solutions
- **Cosmos Zones:** Connect with Cosmos ecosystem app-chains
- **Polkadot Parachains:** Bridge with Polkadot network
- **Custom App-Chains:** Support for specialized application chains

## 📚 References

- **Original Specification:** User's L2-on-top architecture requirements
- **Implementation:** `src/crosschain/`, `src/bridge/` modules
- **Tests:** `tests/l2_commit_exit.rs`
- **Configuration:** `config/default.toml`
- **API:** `src/api/v1.rs` L2 endpoints

---

*This documentation covers the complete L2-on-top architecture implementation in IPPAN. For questions or contributions, please refer to the main project repository.*
