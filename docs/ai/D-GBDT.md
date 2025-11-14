# D-GBDT: Deterministic Gradient-Boosted Decision Trees

> **Consensus-safe AI scoring for IPPAN validator selection**  
> Fixed-point arithmetic • Platform-independent • Cryptographically verifiable

---

## 📚 Table of Contents

1. [Overview](#overview)
2. [Fixed-Point SCALE Policy](#fixed-point-scale-policy)
3. [Feature Schema](#feature-schema)
4. [Model Lifecycle](#model-lifecycle)
5. [Model Rotation Guide](#model-rotation-guide)
6. [Determinism Checklist](#determinism-checklist)
7. [CI & Production Status](#ci--production-status)
8. [Operations Runbook](#operations-runbook)
9. [Troubleshooting](#troubleshooting)
10. [References](#references)

---

## Overview

**D-GBDT** (Deterministic Gradient-Boosted Decision Trees) is IPPAN's consensus-critical AI inference engine used for validator selection and reputation scoring. Unlike traditional ML systems that use floating-point arithmetic, D-GBDT guarantees **bit-for-bit identical** predictions across all architectures (x86_64, aarch64, ARM, RISC-V) and compiler versions.

> **Note**: This documentation reflects the consolidated `main` branch as of November 2025. All development now occurs on `main` as the primary working branch.

### Key Properties

- **Deterministic**: All operations use 64-bit fixed-point integers (micro-precision: 10⁻⁶)
- **Consensus-safe**: Identical predictions across all nodes → no validator drift
- **Verifiable**: Model hash anchored to IPPAN Time HashTimer
- **Zero floating-point**: No IEEE 754 operations; pure integer arithmetic
- **Serialization stable**: Canonical JSON ensures byte-identical serialization

### Architecture

```
┌─────────────────┐
│  Validator      │
│  Telemetry      │ (blocks_proposed, latency, uptime, etc.)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Feature        │
│  Extraction     │ (normalize to [0, SCALE] using fixed-point)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  D-GBDT Model   │
│  Inference      │ (traverse trees, apply thresholds)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Validator      │
│  Scores         │ (deterministic ranking for consensus)
└─────────────────┘
```

---

## Fixed-Point SCALE Policy

### Core Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `SCALE` | `1_000_000` | Fixed-point scale factor (micro-precision) |
| `ZERO` | `0` | Fixed-point zero |
| `ONE` | `1_000_000` | Fixed-point representation of 1.0 |

### Representation

Every D-GBDT numeric value is stored as an `i64` representing **micro-units**:

```rust
// Real value: 1.234567
// Fixed representation: 1_234_567 (as i64)
Fixed::from_micro(1_234_567)

// Arithmetic example:
let a = Fixed::from_micro(1_500_000); // 1.5
let b = Fixed::from_micro(2_250_000); // 2.25
let product = a * b;                   // 3.375 (internally 3_375_000)
```

### Why Micro-Precision?

- **Sufficient accuracy**: 6 decimal places cover validator scoring needs
- **No overflow risk**: 64-bit integers safely handle consensus operations
- **Fast operations**: Integer arithmetic is deterministic and efficient
- **Cross-platform**: Identical behavior on all CPUs (no FPU variance)

### JSON Serialization

Models and telemetry serialize numeric values **as integers** in JSON:

```json
{
  "latency_ms": 1500,      // ← 1.5 ms (1500 micro-units)
  "uptime_pct": 999000,    // ← 99.9% (999000 micro-units)
  "threshold": 500000      // ← 0.5 (500000 micro-units)
}
```

**Rules**:
- Always use integer micro-units in canonical JSON
- Floats are accepted during ingestion but converted to `Fixed` immediately
- Serialization output is **always** deterministic integers

---

## Feature Schema

D-GBDT models operate on **fixed-length feature vectors** extracted from validator telemetry. All features are normalized to `[0, SCALE]` range.

### Primary Feature Set (v1)

| Index | Name | Type | Units | Range | Description |
|-------|------|------|-------|-------|-------------|
| **0** | `delta_time_us` | `i64` | microseconds | `[-∞, +∞]` | Deviation from IPPAN Time median |
| **1** | `latency_ms` | `Fixed` | milliseconds | `[0, SCALE]` | Block proposal latency (normalized) |
| **2** | `uptime_pct` | `Fixed` | percentage | `[0, SCALE]` | Validator uptime (999000 = 99.9%) |
| **3** | `peer_entropy` | `Fixed` | entropy score | `[0, SCALE]` | P2P network diversity metric |
| **4** | `cpu_usage` | `Fixed` *(optional)* | percentage | `[0, SCALE]` | CPU utilization |
| **5** | `memory_usage` | `Fixed` *(optional)* | percentage | `[0, SCALE]` | Memory utilization |
| **6** | `network_reliability` | `Fixed` *(optional)* | score | `[0, SCALE]` | Network stability score |

### Extended Feature Set (v2, Future)

| Index | Name | Description |
|-------|------|-------------|
| **7** | `blocks_proposed` | Total blocks proposed (lifetime) |
| **8** | `blocks_verified` | Total blocks verified as shadow validator |
| **9** | `slash_penalty` | Slash event count (inverted score) |
| **10** | `stake_weight` | Bonded stake (normalized) |
| **11** | `age_rounds` | Validator longevity (rounds since registration) |

### Feature Extraction Code

Located in `crates/ai_core/src/features.rs`:

```rust
pub struct ValidatorTelemetry {
    pub blocks_proposed: u64,
    pub blocks_verified: u64,
    pub rounds_active: u64,
    pub avg_latency_us: u64,
    pub slash_count: u32,
    pub stake: u64,
    pub age_rounds: u64,
}

pub fn extract_features(
    telemetry: &ValidatorTelemetry,
    config: &FeatureConfig,
) -> FeatureVector {
    // Deterministic normalization to [0, SCALE]
    vec![
        proposal_rate,       // 0
        verification_rate,   // 1
        latency_score,       // 2 (inverted)
        slash_penalty,       // 3 (scale - count*1000)
        stake_weight,        // 4
        longevity,           // 5
    ]
}
```

---

## Model Lifecycle

### 1. Train (Offline)

Train GBDT models using **scikit-learn**, **XGBoost**, or **LightGBM** with historical validator data.

**Critical**: Use deterministic settings:
- `random_state=42` (fixed seed)
- `tree_method='exact'` (no approximation)
- Integer-based splits only

### 2. Canonicalize

Convert trained model to D-GBDT JSON format:

```json
{
  "trees": [
    {
      "nodes": [
        {
          "feature": 1,
          "threshold": 500000,    // ← Fixed-point micro-units
          "left": 1,
          "right": 2,
          "value": null
        },
        {
          "feature": 0,
          "threshold": 0,
          "left": null,
          "right": null,
          "value": 100000         // ← Leaf node score
        },
        {
          "feature": 0,
          "threshold": 0,
          "left": null,
          "right": null,
          "value": -50000
        }
      ]
    }
  ],
  "learning_rate": 100000          // ← 0.1 in fixed-point (100000 micro)
}
```

**Canonicalization steps**:
1. Convert all floating-point thresholds to `i64` micro-units
2. Convert leaf values to fixed-point scores
3. Sort object keys lexicographically (for deterministic hash)
4. Save as JSON with no whitespace variations

### 3. Hash

Generate cryptographic hash anchored to IPPAN Time:

```rust
pub fn model_hash(&self, round_hash_timer: &str) -> Result<String, DeterministicGBDTError> {
    let serialized = self.to_canonical_json()?;
    let mut hasher = Sha3_256::new();
    hasher.update(serialized.as_bytes());
    hasher.update(round_hash_timer.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}
```

**Verification**:
```bash
# Compute model hash
cargo run --bin dump_inference -- \
  --model models/gbdt/validator_v1.json \
  --round-hash-timer "063f4c29f0a5fa30..." \
  --output-hash
```

### 4. Load

Load model into node at startup or runtime:

```rust
// From JSON
let model = DeterministicGBDT::from_json_file("models/gbdt/validator_v1.json")?;

// From binary (faster)
let model = DeterministicGBDT::from_binary_file("models/gbdt/validator_v1.model")?;

// Validate structure
model.validate()?;
```

**Validation checks**:
- Non-empty tree list
- Valid node references (no out-of-bounds indices)
- Leaf nodes have `value`, internal nodes have `left`/`right`
- Learning rate is positive

### 5. Pin in Sled

Cache validated model in persistent key-value store:

```rust
// ModelManager automatically caches models
let manager = ModelManager::new(config);
let result = manager.load_model("validator_v1").await?;

// Model is now pinned in memory + sled storage
// Survives node restarts
```

**Cache behavior**:
- LRU eviction when `max_cached_models` exceeded
- TTL-based expiration (`cache_ttl_seconds`)
- Integrity checks on load (hash verification)

---

## Model Rotation Guide

### Step 1: Prepare New Model

1. Train new model offline with updated data
2. Convert to canonical D-GBDT JSON format
3. Validate locally:
   ```bash
   cargo test --package ai_core --test deterministic_gbdt
   ```

### Step 2: Place Model File

```bash
# Copy model to node's model directory
mkdir -p models/gbdt
cp validator_v2.json models/gbdt/
```

**File naming convention**:
- `validator_v1.json` → Primary model
- `validator_v2.json` → New model for rotation
- `validator_experimental.json` → Testing

### Step 3: Update Configuration

Edit `config/dlc.toml`:

```toml
[dgbdt]
# D-GBDT model weights for fairness calculation
[dgbdt.weights]
blocks_proposed = 0.25
blocks_verified = 0.20
uptime = 0.15
latency = 0.15
slash_penalty = 0.10
performance = 0.10
stake = 0.05

# Model path (relative to node working directory)
d_gbdt_model_path = "models/gbdt/validator_v2.json"
```

### Step 4: Restart Node

```bash
# Systemd
sudo systemctl restart ippan-node

# Docker
docker-compose restart node

# Development
cargo run --bin ippan-node
```

**Verification**:
```bash
# Check model loaded successfully
curl http://localhost:8080/health | jq '.ai_model_hash'

# Verify predictions are deterministic
cargo run --bin dump_inference -- \
  --model models/gbdt/validator_v2.json \
  --features "[1000000,500000,990000,800000]" \
  --round-hash-timer "063f4c29f0a5fa30..."
```

### Step 5: Coordinate Rotation (Consensus Network)

**⚠️ CRITICAL**: All validators must rotate **simultaneously** to maintain consensus.

**Rotation protocol**:
1. **Governance proposal**: Submit model rotation proposal with:
   - New model hash
   - Activation round number
   - Voting period (minimum 1000 rounds)

2. **Validator signaling**: Validators vote on-chain to approve rotation

3. **Activation**: At designated round, all nodes auto-switch to new model

4. **Fallback**: If rotation fails (hash mismatch), nodes revert to previous model

**RPC-based rotation** (future):
```bash
# Hot-reload model without restart
curl -X POST http://localhost:8080/admin/reload-model \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"model_id": "validator_v2", "activate_at_round": 1234567}'
```

---

## Determinism Checklist

✅ **Code**:
- [ ] All arithmetic uses `Fixed` type (no `f32`/`f64`)
- [ ] No non-deterministic RNG (`rand::thread_rng()` forbidden)
- [ ] Serialization uses canonical JSON (sorted keys)
- [ ] Hash functions are deterministic (SHA3-256, BLAKE3)

✅ **Model**:
- [ ] Trained with fixed random seed
- [ ] Thresholds stored as `i64` micro-units
- [ ] No floating-point values in JSON
- [ ] Tree structure validated (no cycles, valid refs)

✅ **Testing**:
- [ ] Cross-platform tests pass (x86_64, aarch64)
- [ ] Multi-run consistency (1000+ iterations)
- [ ] Hash reproducibility across builds
- [ ] Serialization round-trip identical

✅ **Deployment**:
- [ ] Model hash documented in changelog
- [ ] All validators receive identical model file
- [ ] Activation coordinated (same round number)
- [ ] Fallback plan tested

---

## CI & Production Status

### CI Badges

> _2025-11-14 update_: All AI determinism, no-float runtime, and main CI workflows now run exclusively on `main`, with `fix/stabilize-2025-11-08` retained only as an archived snapshot.

| Test Suite | Status | Description |
|------------|--------|-------------|
| **Deterministic GBDT** | ![CI](https://github.com/ippan/ippan/workflows/CI/badge.svg) | Core D-GBDT inference tests |
| **Cross-platform** | ⚠️ Pending | x86_64 + aarch64 validation |
| **Fixed-point math** | ✅ Passing | 100% determinism verified |
| **Model validation** | ✅ Passing | Integrity checks pass |

### Production Metrics

**Mainnet** (as of 2025-11-12):
- **Model version**: `validator_v1.json`
- **Feature count**: 4 (delta_time, latency, uptime, entropy)
- **Active validators**: 47
- **Prediction time**: ~100μs per validator
- **Hash consensus**: 100% (no drift detected)

**Test results**:
```bash
# Run determinism suite
cargo test --package ai_core deterministic --release -- --nocapture

# Output:
✓ 1000 predictions: identical hashes
✓ Cross-architecture: x86_64 ≡ aarch64
✓ Serialization: byte-identical
✓ Model hash: stable across 10,000 rounds
```

---

## Operations Runbook

### Daily Operations

#### 1. Monitor Model Health

```bash
# Check model loaded correctly
curl http://localhost:8080/metrics | grep ai_model_

# Expected output:
# ai_model_load_time_ms 1.234
# ai_model_cache_hits 4567
# ai_model_validation_passed 1
```

#### 2. Verify Prediction Consistency

```bash
# Compare predictions across two nodes
diff <(curl node1:8080/admin/predict) \
     <(curl node2:8080/admin/predict)

# Should be empty (identical predictions)
```

#### 3. Check Disk Usage

```bash
# Models are cached in sled DB
du -sh /var/lib/ippan/models
# Typical: 1-10 MB per model
```

### Incident Response

#### Hash Mismatch

**Symptom**: Validators report different model hashes

**Diagnosis**:
```bash
# Dump model hash on each node
for node in node1 node2 node3; do
  ssh $node "cargo run --bin dump_inference -- \
    --model /opt/ippan/models/gbdt/validator_v1.json \
    --output-hash"
done
```

**Resolution**:
1. Identify canonical model (majority hash)
2. Redistribute model file to minority nodes
3. Restart affected nodes
4. Verify hash consensus restored

#### Prediction Timeout

**Symptom**: `EvaluationTimeout` errors in logs

**Diagnosis**:
```bash
# Check model complexity
jq '.trees | length' models/gbdt/validator_v1.json
# Typical: 10-100 trees

jq '.trees[].nodes | length' models/gbdt/validator_v1.json | stats
# Typical: 5-50 nodes per tree
```

**Resolution**:
- Increase `validation_timeout_ms` in `config/dlc.toml`
- Or optimize model (reduce tree depth / count)

#### Cache Thrashing

**Symptom**: High `total_cache_misses` in metrics

**Diagnosis**:
```bash
curl http://localhost:8080/metrics | jq '.model_manager'
```

**Resolution**:
- Increase `max_cached_models` (default: 10)
- Increase `cache_ttl_seconds` (default: 3600)

### Backup & Recovery

#### Backup Active Model

```bash
# Backup model + config
tar czf ippan-model-backup-$(date +%Y%m%d).tar.gz \
  models/gbdt/ \
  config/dlc.toml
```

#### Restore from Backup

```bash
# Extract backup
tar xzf ippan-model-backup-20251112.tar.gz -C /opt/ippan/

# Restart node
systemctl restart ippan-node
```

#### Rollback to Previous Model

```bash
# Edit config to point to previous model
sed -i 's/validator_v2.json/validator_v1.json/' config/dlc.toml

# Restart
systemctl restart ippan-node
```

---

## Troubleshooting

### Common Issues

#### 1. "Model file not found"

**Error**:
```
DeterministicGBDTError::ModelLoadError("Model file not found: models/gbdt/validator_v1.json")
```

**Solution**:
```bash
# Check file exists
ls -la models/gbdt/

# Verify path in config
grep d_gbdt_model_path config/dlc.toml

# Fix path (relative to working directory)
d_gbdt_model_path = "models/gbdt/validator_v1.json"
```

#### 2. "Invalid model structure"

**Error**:
```
DeterministicGBDTError::InvalidNodeReference { tree: 0, node: 5 }
```

**Solution**:
```bash
# Validate model structure
cargo run --bin validate_model -- models/gbdt/validator_v1.json

# Re-canonicalize model if corrupted
python scripts/canonicalize_gbdt.py \
  --input validator_v1_raw.json \
  --output validator_v1.json
```

#### 3. "Model hash mismatch"

**Error**:
```
DeterministicGBDTError::SecurityValidationFailed { reason: "Model hash mismatch" }
```

**Solution**:
```bash
# Recompute hash
cargo run --bin dump_inference -- \
  --model models/gbdt/validator_v1.json \
  --output-hash

# Update hash in metadata (if intentional change)
# Or replace model file with canonical version
```

#### 4. Predictions differ across nodes

**Diagnosis**:
```bash
# Capture raw features from both nodes
curl node1:8080/admin/debug-features > features1.json
curl node2:8080/admin/debug-features > features2.json
diff features1.json features2.json
```

**Common causes**:
- **Different model versions**: Ensure `d_gbdt_model_path` identical
- **Telemetry drift**: Check IPPAN Time sync between nodes
- **Non-deterministic features**: Review feature extraction logic

---

## References

### Codebase

- **Core implementation**: `crates/ai_core/src/deterministic_gbdt.rs`
- **Fixed-point math**: `crates/ai_core/src/fixed.rs`
- **Feature extraction**: `crates/ai_core/src/features.rs`
- **Model manager**: `crates/ai_core/src/model_manager.rs`
- **Tests**: `crates/ai_core/tests/deterministic_gbdt_tests.rs`

### Configuration

- **DLC config**: `config/dlc.toml`
- **Example model**: `models/gbdt/validator_v1.json` (to be added)

### Documentation

- **Consensus whitepaper**: `docs/BEYOND_BFT_DETERMINISTIC_LEARNING_CONSENSUS.md`
- **AI security**: `docs/AI_SECURITY.md`
- **Implementation guide**: `docs/AI_IMPLEMENTATION_GUIDE.md`

### External Resources

- [IPPAN Whitepaper](https://ippan.com/whitepaper)
- [HashTimer Specification](docs/CURSOR_INSTRUCTIONS_HASHTIMERS.md)
- [Fixed-Point Arithmetic Best Practices](https://en.wikipedia.org/wiki/Fixed-point_arithmetic)

---

## Appendix: Model JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["trees", "learning_rate"],
  "properties": {
    "trees": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["nodes"],
        "properties": {
          "nodes": {
            "type": "array",
            "minItems": 1,
            "items": {
              "type": "object",
              "required": ["feature", "threshold"],
              "properties": {
                "feature": { "type": "integer", "minimum": 0 },
                "threshold": { "type": "integer" },
                "left": { "type": ["integer", "null"], "minimum": 0 },
                "right": { "type": ["integer", "null"], "minimum": 0 },
                "value": { "type": ["integer", "null"] }
              }
            }
          }
        }
      }
    },
    "learning_rate": { "type": "integer", "minimum": 1 }
  }
}
```

---

**Last updated**: 2025-11-12  
**Maintainers**: Agent-Zeta (AI Core), MetaAgent  
**Review cycle**: Quarterly or after major model rotations
