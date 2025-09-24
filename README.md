# IPPAN Blockchain

A real blockchain implementation with **IPPAN Time** and **HashTimer** systems for temporal ordering and validation.

## 🚀 Features

- **IPPAN Time**: Monotonic microsecond precision time service with peer synchronization
- **HashTimer**: 256-bit temporal identifiers (14 hex prefix + 50 hex suffix) embedded in all blockchain operations
- **Real Blockchain**: Complete implementation with transactions, blocks, consensus, and P2P networking
- **Unified UI**: Modern React-based frontend for blockchain interaction
- **L2 Bridge**: Anchor rollup commitments and manage exits through the RPC and UI
- **Production Ready**: Docker, systemd, and CI/CD configurations

## 🏗️ Architecture

### Core Components

- **`crates/types`**: HashTimer, IPPAN Time, Transaction, and Block types
- **`crates/crypto`**: Cryptographic primitives and key management
- **`crates/storage`**: Blockchain data persistence
- **`crates/p2p`**: Peer-to-peer networking
- **`crates/mempool`**: Transaction pool management
- **`crates/consensus`**: Block validation and consensus
- **`crates/rpc`**: REST API for blockchain interaction
- **`node`**: Main blockchain node runtime

### HashTimer System

Every blockchain operation includes a **HashTimer**:

```
Format: <14-hex time prefix><50-hex blake3 hash>
Example: 063f4c29f0a5fa30f78d856f1e88975e73c2504559224adc259ccbb3fb90df8a
         ^^^^^^^^^^^^^^ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
         14-char time   50-char cryptographic hash
```

- **Time Prefix**: Microsecond IPPAN Time (56 bits)
- **Hash Suffix**: Blake3 hash of context + time + domain + payload + nonce + node_id (200 bits)

## 🛠️ Development

### Prerequisites

- Rust 1.75+
- Node.js 18+ (for UI)
- Docker (optional)

### Building

```bash
# Build the entire workspace
cargo build --workspace

# Run the node
cargo run --bin ippan-node

# Run tests
cargo test --workspace
```

### Running the Node

```bash
# Start IPPAN node
cargo run --bin ippan-node
```

The node will:
1. Initialize IPPAN Time service
2. Create HashTimers for different contexts (tx, block, round)
3. Demonstrate time monotonicity
4. Start blockchain services

## 🌐 API Endpoints

- `POST /tx` - Submit transaction
- `GET /block/{hash|height}` - Get block
- `GET /account/{address}` - Get account info
- `GET /time` - Get current IPPAN Time
- `POST /api/v1/l2/commit` - Anchor an L2 state commitment
- `GET /api/v1/l2/commits` - List anchored L2 state commitments
- `POST /api/v1/l2/verify_exit` - Submit an exit proof for verification
- `GET /api/v1/l2/exits` - List tracked L2 exit requests
- `GET /api/v1/l2/exits/<id>` - Inspect a single exit by identifier

## 🪐 L2 Anchoring & Exits

IPPAN now exposes a full bridge surface for Layer-2 networks. The RPC keeps the bridge
configuration in `node/src/main.rs` and persists rollup activity through the storage
crate, so every node has access to the latest commitments and exit lifecycle states.

### Anchoring a commitment

```bash
curl -X POST http://localhost:8080/api/v1/l2/commit \
  -H 'Content-Type: application/json' \
  -d '{
        "l2_id": "rollup-1",
        "epoch": 42,
        "state_root": "0xabc123",
        "da_hash": null,
        "proof_type": "zk-groth16",
        "proof": "0xdeadbeef",
        "inline_data": null
      }'
```

The server returns a deterministic `commit_id` and HashTimer. Commitments are stored in
sled and can be queried with `GET /api/v1/l2/commits?l2_id=rollup-1`.

### Submitting an exit

```bash
curl -X POST http://localhost:8080/api/v1/l2/verify_exit \
  -H 'Content-Type: application/json' \
  -d '{
        "l2_id": "rollup-1",
        "epoch": 42,
        "proof_of_inclusion": "base64-proof",
        "account": "0x1234...5678",
        "amount": 12.5,
        "nonce": 7
      }'
```

Responses include the generated exit identifier, current status, and challenge window
deadline. Exits can be listed with `GET /api/v1/l2/exits` or fetched individually via the
`/api/v1/l2/exits/<id>` endpoint.

### Operator UI

The React interoperability dashboard under `/interoperability` lets bridge operators
submit commits, stage exits, and monitor rollup metrics. Start the UI with `npm run dev`
inside `apps/unified-ui` and navigate to the Interoperability page to access the L2
controls.

## 🐳 Deployment

### Docker

```bash
# Build production image
docker build -f Dockerfile.production -t ippan-node .

# Run container
docker run -p 3000:3000 ippan-node
```

### Systemd

```bash
# Install service
sudo cp deploy/ippan-node.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable ippan-node
sudo systemctl start ippan-node
```

## 🎯 Unified UI

The `apps/unified-ui/` directory contains a modern React frontend:

```bash
cd apps/unified-ui
npm install
npm run dev
```

Features:
- Wallet management
- Transaction submission
- Block explorer
- Network monitoring
- Validator registration

## 📊 HashTimer Examples

### Transaction HashTimer
```rust
let tx_hashtimer = HashTimer::now_tx("transfer", payload, nonce, node_id);
// Result: 063f4c29f0c8c7e61eb3d2914435c3ab1894dd6c51eec42c152a2c566922ce4e
```

### Block HashTimer
```rust
let block_hashtimer = HashTimer::now_block("block_creation", payload, nonce, node_id);
// Result: 063f4c29f0c9077cb85a40787b8df4f664299fede0ffd93dd37fc4b576c432a0
```

### Round HashTimer
```rust
let round_hashtimer = HashTimer::now_round("consensus", payload, nonce, node_id);
// Result: 063f4c29f0c90e853ee578cc36d1824f0d9e2241a6ef97e7429366a145bd08e3
```

## 🔧 Configuration

Environment variables:
- `RUST_LOG`: Logging level (default: info)
- `IPPAN_NETWORK`: Network type (mainnet/testnet)
- `IPPAN_DATA_DIR`: Data directory path

## 📈 Performance

- **Time Precision**: Microsecond accuracy
- **HashTimer Generation**: ~1μs per operation
- **Block Processing**: Optimized for high throughput
- **Memory Usage**: Efficient in-memory structures

## 🧪 Testing

```bash
# Run all tests
cargo test --workspace

# Run specific test suite
cargo test -p ippan-types

# Run with logging
RUST_LOG=debug cargo test --workspace
```

## 📝 License

Apache-2.0

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 🚨 Security

- All cryptographic operations use industry-standard libraries
- HashTimer provides temporal ordering guarantees
- IPPAN Time prevents time-based attacks
- Production deployments include security hardening

---

**IPPAN Blockchain**: Real blockchain with authoritative time and temporal validation.