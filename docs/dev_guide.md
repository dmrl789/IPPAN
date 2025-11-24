# IPPAN Developer Guide
**Building, Testing, and Contributing to IPPAN**

**Version:** v1.0.0-rc1  
**Last Updated:** 2025-11-24

---

## Table of Contents

1. [Repository Layout](#repository-layout)
2. [Building the Node](#building-the-node)
3. [Running a Development Node](#running-a-development-node)
4. [Running Tests](#running-tests)
5. [Coding Conventions](#coding-conventions)
6. [Adding New Features](#adding-new-features)
7. [Debugging Tips](#debugging-tips)
8. [Contributing](#contributing)

---

## Repository Layout

```
IPPAN/
├── crates/          # Core Rust crates
│   ├── ai_core/            # D-GBDT inference engine (no floats)
│   ├── ai_registry/        # Model storage and activation
│   ├── consensus/          # Core consensus logic
│   ├── consensus_dlc/      # DLC + DAG consensus
│   ├── storage/            # Sled-based persistence
│   ├── rpc/                # Axum HTTP/WebSocket API
│   ├── p2p/                # libp2p networking
│   ├── time/               # HashTimer implementation
│   ├── economics/          # Emission and fee logic
│   └── ...                 # Other crates
├── node/            # Main node binary
├── docs/            # Documentation
├── testnet/         # Testnet configurations
├── grafana_dashboards/ # Observability dashboards
├── scripts/         # Helper scripts
├── tests/           # Integration tests
└── Cargo.toml       # Workspace manifest
```

### Key Crates

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| **ippan-consensus** | Emission tracking, validator mgmt | economics, storage |
| **ippan-consensus-dlc** | DLC verifier selection, DAG | ai_core, time |
| **ippan-storage** | Sled/memory storage, snapshots | sled, types |
| **ippan-rpc** | HTTP API (Axum) | consensus, storage |
| **ippan-p2p** | libp2p networking, DHT | libp2p, types |
| **ippan-time** | HashTimer (deterministic time) | blake3, chrono |
| **ippan-ai-core** | D-GBDT (no floats) | blake3, serde |

---

## Building the Node

### Prerequisites

- **Rust:** 1.70+ (stable channel)
- **System packages:** `build-essential`, `libssl-dev`, `pkg-config` (Linux)
- **macOS:** `brew install openssl pkg-config`

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup update stable
```

### Clone Repository

```bash
git clone https://github.com/dmrl789/IPPAN.git
cd IPPAN
```

### Build All Crates

```bash
cargo build --workspace --release
```

**Build Time:** ~10-30 minutes (first build)  
**Output:** `target/release/ippan-node`

### Build Specific Crate

```bash
cargo build -p ippan-consensus --release
cargo build -p ippan-rpc --release
```

### Build in Debug Mode (Faster, for Development)

```bash
cargo build --workspace
# Output: target/debug/ippan-node
```

---

## Running a Development Node

### Quick Start (In-Memory)

```bash
cargo run --bin ippan-node -- --config config/local-node.toml
```

**Default ports:**
- RPC: `http://localhost:8080`
- P2P: `0.0.0.0:9000`
- Metrics: `http://localhost:9615/metrics`

### With Persistent Storage (Sled)

```bash
mkdir -p data/dev-node
cargo run --bin ippan-node -- --config config/local-node.toml --data-dir data/dev-node
```

### Development Environment Variables

```bash
export RUST_LOG=debug
export IPPAN_DEV_MODE=1          # Enables /dev/* endpoints
export IPPAN_DGBDT_ALLOW_STUB=1  # Allows stub D-GBDT (for local testing)

cargo run --bin ippan-node
```

### Check Node Status

```bash
curl http://localhost:8080/health
curl http://localhost:8080/metrics
```

---

## Running Tests

### All Tests

```bash
cargo test --workspace
```

### Specific Crate

```bash
cargo test -p ippan-consensus -- --nocapture
cargo test -p ippan-consensus-dlc -- --nocapture
cargo test -p ippan-storage -- --nocapture
```

### Long-Run Simulations

```bash
# 256-round emission invariants test (~10s)
cargo test -p ippan-consensus-dlc long_run_emission_and_fairness_invariants -- --nocapture

# 240-round fairness test (~30s)
cargo test -p ippan-consensus-dlc long_run_fairness_roles_remain_balanced -- --nocapture

# 512-round chaos test (~60s)
cargo test -p ippan-consensus-dlc long_run_dlc_with_churn_splits_slashing_and_drift -- --nocapture
```

### Integration Tests

```bash
cargo test --test integration_test -- --nocapture
```

### Benchmarks

```bash
cargo bench -p ippan-ai-core
cargo bench -p ippan-storage
```

---

## Coding Conventions

### 1. No Floating-Point in Runtime Code

**Rule:** Use `i64`, `u64`, or fixed-point integers for all runtime calculations.

**Rationale:** Ensures deterministic consensus across architectures.

**Examples:**

✅ **Correct:**
```rust
const SCALE: i64 = 1_000_000;
let score: i64 = (uptime * SCALE) / 100;
```

❌ **Incorrect:**
```rust
let score: f64 = uptime as f64 / 100.0;  // NO FLOATS!
```

**Allowed:**  
- Floats in tests (`#[cfg(test)]`)
- Floats in examples (`examples/`)
- Floats in benches (`benches/`)
- Floats in training code (`ai_trainer/`)

### 2. Error Handling

Use `Result<T, E>` for fallible operations:

```rust
pub fn process_transaction(tx: Transaction) -> Result<Receipt, ConsensusError> {
    if tx.amount.is_zero() {
        return Err(ConsensusError::InvalidAmount);
    }
    // ... process
    Ok(receipt)
}
```

**Don't use:**
- `unwrap()` in production code (use `expect()` with clear message, or `?`)
- `panic!()` outside of invariant violations

### 3. Logging

Use `tracing` crate:

```rust
use tracing::{info, warn, error, debug};

info!("Processing round {}", round);
warn!(validator = %validator_id, "Low reputation score");
error!("Failed to finalize block: {:?}", err);
debug!(score = %score, "D-GBDT inference complete");
```

**Levels:**
- `error!`: Critical failures
- `warn!`: Recoverable issues, potential problems
- `info!`: Normal operation events
- `debug!`: Detailed diagnostic info
- `trace!`: Very verbose (hot path logging)

### 4. Integer Arithmetic Safety

Always check for overflow/underflow in critical paths:

```rust
// ✅ Good: saturating arithmetic
let new_balance = balance.saturating_add(amount);
let remaining = supply.saturating_sub(emission);

// ✅ Good: checked arithmetic with error handling
let sum = balance.checked_add(amount)
    .ok_or(EconomicsError::Overflow)?;

// ❌ Bad: unchecked (can panic)
let sum = balance + amount;  // Panics on overflow in debug, wraps in release!
```

### 5. Serialization

Use canonical JSON for deterministic hashing:

```rust
use serde_json;

// ✅ Canonical JSON (sorted keys)
let json = serde_json::to_string_pretty(&model)?;
let hash = blake3::hash(json.as_bytes());

// Document that order matters!
```

---

## Adding New Features

### Scenario: Add a New Transaction Type

1. **Define the type in `crates/types/src/transaction.rs`:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxKind {
    Payment { ... },
    Handle { ... },
    NewFeature { data: Vec<u8> },  // <-- Add here
}
```

2. **Add validation logic in `crates/consensus/src/lib.rs`:**

```rust
fn validate_transaction(tx: &Transaction) -> Result<(), ConsensusError> {
    match &tx.kind {
        TxKind::NewFeature { data } => {
            if data.len() > MAX_FEATURE_DATA_SIZE {
                return Err(ConsensusError::InvalidTransaction(...));
            }
            Ok(())
        }
        // ... other cases
    }
}
```

3. **Add processing logic in `crates/consensus/src/round_executor.rs`:**

```rust
fn apply_transaction(tx: &Transaction, state: &mut State) -> Result<(), ConsensusError> {
    match &tx.kind {
        TxKind::NewFeature { data } => {
            // Update state
            state.feature_registry.insert(tx.id, data.clone());
            Ok(())
        }
        // ... other cases
    }
}
```

4. **Add RPC endpoint in `crates/rpc/src/server.rs`:**

```rust
async fn submit_new_feature(
    Json(payload): Json<NewFeatureRequest>,
    State(state): State<AppState>,
) -> Result<Json<TxResponse>, RpcError> {
    let tx = Transaction::new_feature(payload.data);
    state.mempool.add(tx)?;
    Ok(Json(TxResponse { tx_id: tx.id }))
}

// Wire into router
app.route("/tx/new_feature", post(submit_new_feature))
```

5. **Add tests in `crates/consensus/tests/`:**

```rust
#[test]
fn test_new_feature_transaction() {
    let tx = Transaction::new_feature(vec![1, 2, 3]);
    assert!(validate_transaction(&tx).is_ok());
}
```

6. **Update documentation in `docs/`:**

Create `docs/NEW_FEATURE_SPEC.md` describing:
- Rationale
- Transaction format
- Validation rules
- RPC interface
- Test scenarios

---

## Debugging Tips

### 1. Enable Verbose Logging

```bash
RUST_LOG=ippan_consensus=trace,ippan_rpc=debug cargo run
```

### 2. Debug a Specific Test

```rust
#[test]
fn test_my_feature() {
    tracing_subscriber::fmt::init();  // Enable logging in test
    // ... test code
}
```

Run with:
```bash
cargo test test_my_feature -- --nocapture
```

### 3. Use `dbg!()` Macro

```rust
let score = model.score(&features);
dbg!(score);  // Prints: [src/model.rs:42] score = 8500000000
```

### 4. Inspect Storage (Sled)

```bash
cargo run --bin ippan-node -- snapshot export --output snapshot.json
cat snapshot.json | jq '.accounts[] | select(.balance > 0)'
```

### 5. Check Consensus State

```rust
// In code:
eprintln!("DAG stats: {:?}", consensus.dag.stats());
eprintln!("Validator count: {}", consensus.validators.count());
```

---

## Contributing

### Workflow

1. **Fork the repository** on GitHub
2. **Clone your fork:**
   ```bash
   git clone https://github.com/YOUR_USERNAME/IPPAN.git
   cd IPPAN
   ```

3. **Create a feature branch:**
   ```bash
   git checkout -b feature/my-new-feature
   ```

4. **Make changes and test:**
   ```bash
   cargo fmt
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```

5. **Commit directly to `master` (per project workflow):**
   ```bash
   git checkout master
   git merge feature/my-new-feature
   git push origin master
   ```

   **Note:** Per AGENTS.md, this project uses a `master`-only workflow. No PRs unless explicitly requested by maintainers.

### Code Review Checklist

- [ ] All tests pass (`cargo test --workspace`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy --workspace`)
- [ ] No new floats in runtime code
- [ ] Logging added for key events
- [ ] Documentation updated (if API/behavior changes)
- [ ] Integration tests added (if new subsystem)

---

## Common Tasks

### Add a New Crate

```bash
cd crates
cargo new my_new_crate --lib
```

Edit `Cargo.toml` (workspace root):
```toml
[workspace.members]
members = [
    "crates/my_new_crate",
    # ... other crates
]
```

### Update Dependencies

```bash
cargo update
cargo build --workspace
cargo test --workspace
```

### Clean Build Artifacts

```bash
cargo clean
```

### Check for Outdated Dependencies

```bash
cargo install cargo-outdated
cargo outdated
```

---

## Summary

✅ **Repository:** Workspace with ~20 crates  
✅ **Build:** `cargo build --workspace --release`  
✅ **Run:** `cargo run --bin ippan-node`  
✅ **Test:** `cargo test --workspace`  
✅ **Conventions:** No floats, use `Result<T, E>`, tracing for logs  
✅ **Workflow:** Direct commits to `master` (no PRs unless requested)  

**Next Steps:**
- Read `docs/architecture_overview.md` for system design
- Check `docs/FEES_AND_EMISSION.md` for economics
- Review `CHECKLIST_AUDIT_MAIN.md` for feature status

---

**For Questions:** IPPAN Community Discord / GitHub Issues  
**Maintainers:** Ugo Giuliani, Desirée Verga, Kambei Sapote
