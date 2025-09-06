# L2 Quick Start Guide

## 🚀 Get Started with IPPAN L2 in 5 Minutes

This guide will help you quickly set up and start using the L2-on-top architecture in IPPAN.

## 📋 Prerequisites

- Rust 1.70+ installed
- IPPAN repository cloned
- Basic understanding of blockchain concepts

## ⚡ Quick Setup

### 1. Build IPPAN with L2 Support

```bash
# Navigate to IPPAN directory
cd ippan

# Build with L2 support (crosschain feature)
cargo build --features crosschain

# For ZK proof support (optional)
cargo build --features "crosschain zk-groth16"
```

### 2. Verify Installation

```bash
# Run L2 tests to ensure everything works
cargo test --test l2_commit_exit

# Expected output: 8 tests passing
```

## 🎯 Your First L2 Network

### 1. Register an L2 Network

```bash
# Register a ZK rollup
ippan-cli l2 register \
  --id my-zk-rollup \
  --proof-type zk-groth16 \
  --da external \
  --challenge-window-ms 60000 \
  --max-commit-size 16384 \
  --min-epoch-gap-ms 250
```

### 2. Submit Your First Commit

```bash
# Submit L2 state update
ippan-cli l2 commit \
  --id my-zk-rollup \
  --epoch 1 \
  --state-root 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --da-hash 0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321 \
  --proof 0xabcd1234efgh5678ijkl9012mnop3456
```

### 3. Check Status

```bash
# Get L2 network status
ippan-cli l2 status --id my-zk-rollup

# List all L2 networks
ippan-cli l2 list
```

## 🔧 Configuration

### Basic L2 Configuration

Edit `config/default.toml`:

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

## 📡 API Usage

### REST API Endpoints

```bash
# Register L2 network
curl -X POST http://localhost:3000/v1/l2/register \
  -H "Content-Type: application/json" \
  -d '{
    "l2_id": "my-rollup",
    "proof_type": "zk-groth16",
    "da_mode": "external"
  }'

# Submit commit
curl -X POST http://localhost:3000/v1/l2/commit \
  -H "Content-Type: application/json" \
  -d '{
    "l2_id": "my-rollup",
    "epoch": 1,
    "state_root": "0x1234...",
    "da_hash": "0x5678...",
    "proof_type": "zk-groth16",
    "proof": "0xabcd..."
  }'

# Get status
curl http://localhost:3000/v1/l2/my-rollup/status
```

## 🧪 Testing

### Run All L2 Tests

```bash
# Run comprehensive L2 test suite
cargo test --test l2_commit_exit -- --nocapture

# Run specific test
cargo test --test l2_commit_exit test_l2_commit_validation
```

### Test Coverage

The test suite covers:
- ✅ L2 commit validation
- ✅ L2 exit validation
- ✅ Epoch monotonicity
- ✅ Rate limiting
- ✅ Challenge windows
- ✅ Anchor events
- ✅ L2 verifier
- ✅ Registry configuration

## 🔍 Monitoring

### Check L2 Metrics

```bash
# View Prometheus metrics (if configured)
curl http://localhost:3000/metrics | grep l2

# Expected metrics:
# ippan_l2_commits_total{l2_id="my-rollup"}
# ippan_l2_exits_total{l2_id="my-rollup"}
# ippan_l2_commit_bytes_sum{l2_id="my-rollup"}
```

### Logs

L2 operations are logged with structured JSON format:

```json
{
  "level": "info",
  "message": "L2 commit accepted",
  "l2_id": "my-rollup",
  "epoch": 1,
  "timestamp": "2024-01-01T00:00:00Z"
}
```

## 🚨 Common Issues

### Build Errors

```bash
# Error: "crosschain" feature not found
# Solution: Ensure crosschain is in default features
cargo build --features crosschain

# Error: ZK dependencies missing
# Solution: Install ZK feature or use basic L2
cargo build --features "crosschain zk-groth16"
```

### Runtime Errors

```bash
# Error: "L2 not registered"
# Solution: Register L2 network first
ippan-cli l2 register --id my-rollup --proof-type optimistic

# Error: "Epoch regression"
# Solution: Ensure epoch numbers increase monotonically
```

## 📚 Next Steps

### Learn More

- **Complete Architecture**: Read `docs/L2_ARCHITECTURE.md`
- **Implementation Details**: See `docs/L2_IMPLEMENTATION_SUMMARY.md`
- **API Reference**: Check `src/api/v1.rs` for endpoint details

### Advanced Usage

- **Custom Proof Systems**: Implement custom `L2Verifier` trait
- **Data Availability**: Configure inline vs external DA modes
- **Rate Limiting**: Adjust L2-specific rate limits
- **Monitoring**: Set up Prometheus metrics and alerting

### Examples

- **ZK Rollup**: High-security, instant-finality applications
- **Optimistic Rollup**: High-throughput, cost-effective applications
- **App-Chain**: Specialized blockchain applications
- **Hybrid Systems**: Combine multiple proof types

## 🆘 Need Help?

- **Documentation**: Check the docs folder
- **Tests**: Run test suite for examples
- **Code**: Review `src/crosschain/` and `src/bridge/` modules
- **Issues**: Open GitHub issue with detailed error information

---

*This quick start guide gets you up and running with IPPAN L2. For comprehensive information, see the full documentation.*
