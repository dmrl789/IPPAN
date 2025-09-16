# IPPAN Real Mode Implementation

This document describes the real-mode implementation of IPPAN that replaces all mock/demo code with live blockchain functionality.

## 🚫 Zero Mock Policy

The project now enforces a strict zero-mock policy:

- **CI Guard**: `ci/no_mocks.sh` fails the build if any mock/demo code is detected
- **API Guards**: API panics if started without a live node reference
- **Environment Flags**: `REAL_MODE_REQUIRED=true` and `DEMO_MODE=false` are enforced

## 🏗️ Architecture

### Core Components

1. **Real-Mode API** (`src/api/real_mode.rs`)
   - Implements minimal RPC contract
   - Wired to live node state
   - No mock responses

2. **Ed25519 Signer** (`src/crypto/ed25519_signer.rs`)
   - Canonical transaction signing
   - Real cryptographic operations
   - Transaction verification

3. **Genesis Configuration** (`config/genesis.json`)
   - Deterministic initial state
   - Funded test addresses
   - Real blockchain parameters

## 📡 API Contract

The real-mode API implements the following minimal contract:

### Endpoints

- `GET /api/v1/status` → `{ height, peers, role, latest_block_hash }`
- `GET /api/v1/address/validate?address=...` → `{ valid: bool }`
- `POST /api/v1/tx/submit` → `{ tx_id }` (HTTP 202)
- `GET /api/v1/tx/{tx_id}` → `{ status: "in_mempool"|"included"|"rejected", block_hash? }`
- `GET /api/v1/block/latest` → `{ height, hash, tx_ids: [...] }`

### Error Handling

- **400/401** for: bad JSON, invalid signature, wrong nonce, insufficient balance, double spend
- **404** for: unknown endpoints
- **500** for: internal server errors

## 🔧 Configuration

### Node Configurations

**Node A (188.245.97.41)** - `config/node-a.json`:
```json
{
  "node_id": "node-a",
  "p2p": {"listen": "0.0.0.0:8080", "seeds": []},
  "rpc": {"http": "0.0.0.0:3000"},
  "db_path": "/var/lib/ippan/node-a",
  "genesis": "/etc/ippan/genesis.json",
  "real_mode": true
}
```

**Node B (135.181.145.174)** - `config/node-b.json`:
```json
{
  "node_id": "node-b",
  "p2p": {"listen": "0.0.0.0:8080", "seeds": ["188.245.97.41:8080"]},
  "rpc": {"http": "0.0.0.0:3000"},
  "db_path": "/var/lib/ippan/node-b",
  "genesis": "/etc/ippan/genesis.json",
  "real_mode": true
}
```

### Genesis Configuration

```json
{
  "chain_id": "ippan-devnet-001",
  "timestamp": "2025-01-15T12:00:00Z",
  "alloc": [
    {
      "address": "iSender1111111111111111111111111111111111111",
      "balance": "1000000000"
    },
    {
      "address": "iRbDqSo0H4NxPGC0q55ohG36JrvlcYGvM3DpS4Q",
      "balance": "0"
    }
  ],
  "params": {
    "block_ms": 50,
    "round_ms": 200,
    "max_block_bytes": 32768
  }
}
```

## 🚀 Deployment

### Prerequisites

1. Set environment variables:
   ```bash
   export REAL_MODE_REQUIRED=true
   export DEMO_MODE=false
   ```

2. Ensure SSH access to both servers:
   - 188.245.97.41 (Node A)
   - 135.181.145.174 (Node B)

### Deploy

```bash
# Run the deployment script
./deploy_real_mode.sh
```

### Manual Deployment

```bash
# Build
cargo build --release

# Deploy to Node A
scp target/release/ippan root@188.245.97.41:/usr/local/bin/ippan-node
scp config/genesis.json root@188.245.97.41:/etc/ippan/genesis.json
scp config/node-a.json root@188.245.97.41:/etc/ippan/node.json
scp config/ippan.service root@188.245.97.41:/etc/systemd/system/ippan.service

# Deploy to Node B
scp target/release/ippan root@135.181.145.174:/usr/local/bin/ippan-node
scp config/genesis.json root@135.181.145.174:/etc/ippan/genesis.json
scp config/node-b.json root@135.181.145.174:/etc/ippan/node.json
scp config/ippan.service root@135.181.145.174:/etc/systemd/system/ippan.service

# Start services
ssh root@188.245.97.41 "systemctl daemon-reload && systemctl enable ippan && systemctl restart ippan"
ssh root@135.181.145.174 "systemctl daemon-reload && systemctl enable ippan && systemctl restart ippan"
```

## 🧪 Testing

### TestSprite Configuration

The project includes a comprehensive TestSprite configuration (`testsprite.prd.yaml`) that tests:

1. **Health Checks**: Both nodes are running and accessible
2. **Address Validation**: Receiver address format validation
3. **Transaction Submission**: Signed transaction processing
4. **Cross-Node Visibility**: Transactions visible on both nodes
5. **Persistence**: State survives node restarts
6. **Error Handling**: Invalid signatures and double-spend rejection

### Manual Testing

```bash
# Check node status
curl -sS http://188.245.97.41:3000/api/v1/status | jq
curl -sS http://135.181.145.174:3000/api/v1/status | jq

# Validate address
curl -sS "http://188.245.97.41:3000/api/v1/address/validate?address=iRbDqSo0H4NxPGC0q55ohG36JrvlcYGvM3DpS4Q" | jq

# Generate and submit transaction
python3 tools/gen_tx.py iSender1111111111111111111111111111111111111 iRbDqSo0H4NxPGC0q55ohG36JrvlcYGvM3DpS4Q 25000 10 1 /tmp/tx.json
curl -sS -X POST http://188.245.97.41:3000/api/v1/tx/submit -H "Content-Type: application/json" --data @/tmp/tx.json | jq

# Check transaction status
curl -sS http://188.245.97.41:3000/api/v1/tx/<TX_ID> | jq
curl -sS http://135.181.145.174:3000/api/v1/tx/<TX_ID> | jq
```

## 🔐 Transaction Signing

### Ed25519 Implementation

The project includes a complete Ed25519 signing implementation:

```rust
use ippan::crypto::{Ed25519Keypair, CanonicalTransaction, TransactionSigner};

// Generate keypair
let keypair = Ed25519Keypair::generate();

// Create transaction
let tx = CanonicalTransaction::new(
    "ippan-devnet-001".to_string(),
    "iSender1111111111111111111111111111111111111".to_string(),
    "iRbDqSo0H4NxPGC0q55ohG36JrvlcYGvM3DpS4Q".to_string(),
    "25000".to_string(),
    "10".to_string(),
    1,
    timestamp.to_string(),
);

// Sign transaction
let signer = TransactionSigner::new(keypair);
let signed_tx = signer.sign_transaction(tx)?;
```

### Transaction Format

```json
{
  "chain_id": "ippan-devnet-001",
  "from": "iSender1111111111111111111111111111111111111",
  "to": "iRbDqSo0H4NxPGC0q55ohG36JrvlcYGvM3DpS4Q",
  "amount": "25000",
  "fee": "10",
  "nonce": 1,
  "timestamp": "1640995200",
  "signature": "ed25519_signature_hex",
  "pubkey": "ed25519_public_key_hex"
}
```

## 📊 Monitoring

### Systemd Service

Both nodes run as systemd services:

```bash
# Check status
systemctl status ippan

# View logs
journalctl -u ippan -f

# Restart
systemctl restart ippan
```

### Health Checks

```bash
# Node health
curl -sS http://188.245.97.41:3000/api/v1/status

# Network connectivity
curl -sS http://135.181.145.174:3000/api/v1/status
```

## 🚨 Troubleshooting

### Common Issues

1. **API Panic on Startup**
   - Ensure `REAL_MODE_REQUIRED=true` is set
   - Verify node reference is properly initialized

2. **Transaction Rejection**
   - Check signature validity
   - Verify nonce sequence
   - Ensure sufficient balance

3. **Node Connection Issues**
   - Verify firewall settings (ports 3000, 8080)
   - Check network connectivity between nodes
   - Review systemd service logs

### Debug Commands

```bash
# Check CI guard
bash ci/no_mocks.sh

# Verify environment
echo $REAL_MODE_REQUIRED
echo $DEMO_MODE

# Test API endpoints
curl -v http://188.245.97.41:3000/api/v1/status
```

## 📈 Next Steps

After core functionality is verified:

1. **Mempool & Fees**: Implement transaction fee validation
2. **Consensus**: Add block production and validation
3. **State Queries**: Implement balance and nonce queries
4. **Rate Limiting**: Add transaction rate limiting
5. **Explorer**: Build blockchain explorer interface
6. **Wallet**: Create user wallet application

## 🔗 Links

- [TestSprite Configuration](testsprite.prd.yaml)
- [Deployment Script](deploy_real_mode.sh)
- [Genesis Configuration](config/genesis.json)
- [Node Configurations](config/)
- [API Implementation](src/api/real_mode.rs)
- [Ed25519 Signer](src/crypto/ed25519_signer.rs)

