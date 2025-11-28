# IPPAN D-GBDT Model Registry

This document tracks all D-GBDT fairness models trained and promoted to production use in the IPPAN DLC consensus system.

## Model Format

All models use the deterministic GBDT format with:
- Integer-only inference (no floats in runtime)
- Canonical JSON serialization (sorted keys for deterministic hashing)
- BLAKE3 hash verification via `cargo run -p ippan-ai-core --bin verify_model_hash`

## Model Entries

### ippan_d_gbdt_localnet_v4.json

**Model File**: `crates/ai_registry/models/ippan_d_gbdt_localnet_v4.json`

**BLAKE3 Hash**: `f684f53e9efbd04f12d5170ef885106b9ea9696987ff83662bbcaeda1bb0470d`

**Dataset**:
- Path: `ai_assets/datasets/localnet/localnet_training.csv`
- Row count: 2000 (training) + 1 (header) = 2001 total
- Source: DLC localnet with metrics drift enabled
- Export date: 2025-11-28

**Training**:
- Script: `ai_training/train_ippan_d_gbdt.py`
- Command:
  ```bash
  python ai_training/train_ippan_d_gbdt.py \
    --csv ai_assets/datasets/localnet/localnet_training.csv \
    --out ai_assets/models/staging/ippan_d_gbdt_localnet_v4.json
  ```
- Hyperparameters:
  - n_estimators: 300
  - learning_rate: 0.05
  - max_depth: 3
  - random_state: 42 (deterministic)
- Validation MSE: 0.000024

**Features** (7 total, all scaled to 1,000,000):
- `uptime_ratio_7d`: Uptime percentage (0-1,000,000)
- `validated_blocks_7d`: Blocks verified in last 7 days
- `missed_blocks_7d`: Blocks missed in last 7 days
- `avg_latency_ms`: Average latency in milliseconds
- `slashing_events_90d`: Number of slashing events
- `stake_normalized`: Stake amount (normalized 0-1, scaled)
- `peer_reports_quality`: Peer quality reports (normalized 0-1, scaled)

**Target**: `fairness_score` (0.0-1.0, computed deterministically)

**Scale Factors**:
- Feature scale: 1,000,000 (micro precision)
- Post scale: 1,000,000
- Bias: 0

**Promotion**:
- Script: `ai_training/promote_fairness_model.py`
- Command:
  ```bash
  python ai_training/promote_fairness_model.py \
    --model ai_assets/models/staging/ippan_d_gbdt_localnet_v4.json \
    --runtime-dest crates/ai_registry/models/ippan_d_gbdt_localnet_v4.json \
    --config config/dlc.toml
  ```
- Config updated: `config/dlc.toml` [dgbdt.model] section

**Verification**:
- Hash verification: `cargo run -p ippan-ai-core --bin verify_model_hash -- config/dlc.toml` ✅
- Tests: `cargo test -p ippan-ai-registry --doc` ✅
- Fairness invariants: `cargo test -p ippan-consensus-dlc --test fairness_invariants` ✅

**Environment**:
- Machine: Windows 11
- Localnet: Docker Compose (3 nodes)
- Consensus: DLC with metrics drift enabled
- Model format: LightGBM v4 (converted to deterministic integer format)

**Notes**:
- Model trained from localnet dataset with metrics drift simulation
- Hash computed by Rust verifier from canonical JSON model structure (not raw file bytes)
- All inference is integer-only (no f32/f64 in runtime)

