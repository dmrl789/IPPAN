# IPPAN D-GBDT Training Bootstrap (Offline Only)

This guide describes how to generate a synthetic dataset and train IPPAN D-GBDT fairness models on a **separate, trusted machine** (e.g., Hetzner or local laptop). IPPAN nodes **never train models**; they only load the frozen JSON artifact and perform deterministic integer inference.

## Prerequisites

- Python 3.8+ with `pip`
- Rust toolchain (for runtime verification only)
- Git LFS installed (`git lfs install --local`)

## Environment setup

### Local Laptop (Windows PowerShell)

```powershell
# Create and activate virtual environment
python -m venv .venv
.\.venv\Scripts\Activate.ps1

# Install dependencies
pip install --upgrade pip
pip install "numpy==1.26.4" "pandas==2.2.2" "scikit-learn==1.5.2" "lightgbm==4.3.0" "blake3==0.4.1"
```

### Hetzner Server (Linux)

```bash
# Install system dependencies
sudo apt-get update
sudo apt-get install -y python3 python3-venv python3-pip build-essential

# Create and activate virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install Python dependencies
pip install --upgrade pip
pip install "numpy==1.26.4" "pandas==2.2.2" "scikit-learn==1.5.2" "lightgbm==4.3.0" "blake3==0.4.1"
```

## Generate the training dataset

### Option 1: Synthetic dataset (bootstrap)

This uses a fixed RNG seed (42) and writes `ai_training/data/ippan_training.csv` (gitignored).

```bash
# Windows PowerShell
python ai_training/generate_synthetic_dataset.py

# Linux/macOS
python3 ai_training/generate_synthetic_dataset.py
```

**Output**: `ai_training/data/ippan_training.csv` (100,000 rows, deterministic)

### Option 2: Localnet dataset (real-world proxy)

Export validator metrics from a running localnet to generate training data:

**Prerequisites**: The `/status` endpoint must expose `consensus.validators` map (not just `validator_ids` array). Update node to latest version if metrics are missing.

1. **Start localnet** (see [Localnet Quickstart](../docs/LOCALNET_QUICKSTART.md)):
   ```bash
   # Windows PowerShell
   .\localnet\run.ps1
   
   # Linux/macOS
   scripts/run-local-full-stack.sh
   ```

2. **Export dataset**:
   ```bash
   # Windows PowerShell
   .\localnet\export-dataset.ps1
   
   # Linux/macOS (or direct Python)
   python ai_training/export_localnet_dataset.py --mode rpc --rpc http://localhost:8080 --samples 120 --interval 5 --out ai_training/localnet_training.csv
   ```

   This fetches validator metrics from the RPC endpoint (`/status`) and exports to `ai_training/localnet_training.csv` (gitignored).
   
   **Note**: The exporter requires `consensus.validators` to be a map/object. If the endpoint only returns `validator_ids` array, the exporter will error with a clear message. The `stake.micro_ipn` field may be serialized as a string (JSON u128 limitation).

   **Metrics drift**: You can enable deterministic metrics drift to generate richer datasets with varied validator behavior:
   
   ```powershell
   .\localnet\run.ps1 -DriftMode tiers -DriftSeed 1
   ```
   
   This produces different metrics per validator and evolves deterministically by round, enabling non-identical model hashes across training runs.

3. **Train as usual** (see below).

**Note**: Localnet exports produce "proxy 7d" features (windowed deltas from current metrics) suitable for bootstrap/testing. For production training, use longer collection periods or aggregate historical data from testnet/mainnet.

## Promote model to runtime (strict hash-pinned)

After training and hashing, vendor the model into the runtime path:

### Step 1: Copy model to runtime location

```bash
# Windows PowerShell
Copy-Item ai_training\ippan_d_gbdt_v2.json crates\ai_registry\models\ippan_d_gbdt_v2.json

# Linux/macOS
cp ai_training/ippan_d_gbdt_v2.json crates/ai_registry/models/ippan_d_gbdt_v2.json
```

### Step 2: Update config with pinned hash

Edit `config/dlc.toml`:

```toml
[dgbdt.model]
path = "crates/ai_registry/models/ippan_d_gbdt_v2.json"
expected_hash = "<computed_hash_from_step_above>"
```

### Step 3: Verify strict loading

The runtime uses `ippan_ai_registry::d_gbdt::load_fairness_model_strict()` which:
- Reads model file bytes
- Computes BLAKE3 hash
- Compares to `expected_hash` in config
- **Fails fast** (returns error) if hash mismatch
- Only then deserializes JSON

**Node startup behavior**: If hash doesn't match, consensus initialization fails and the node refuses to start. This ensures all nodes use the exact same model bytes.

### Step 4: Test runtime loading

```bash
cargo test -p ippan-ai-registry --lib d_gbdt::tests::test_load_fairness_model_strict
cargo test -p ippan-consensus-dlc --lib
```

### Step 5: Promote with guard (recommended)

Use the promotion tool which automatically:
- Computes BLAKE3 hash
- Compares to current pinned hash in config
- **Refuses promotion if hash unchanged** (prevents fake "new" versions)
- Updates config and copies model file

```powershell
# Windows PowerShell
.\localnet\promote-model.ps1 -Model ai_training\ippan_d_gbdt_v3.json -Version v3

# Or direct Python
python ai_training/promote_fairness_model.py `
  --model ai_training/ippan_d_gbdt_v3.json `
  --runtime-dest crates/ai_registry/models/ippan_d_gbdt_v3.json
```

**Hash guard behavior**:
- If the new model hash matches the currently pinned hash, promotion is **refused** (exit code 2)
- This prevents accidentally creating "fake" new versions when the model hasn't changed
- To override (use with caution): add `--allow-same-hash` flag

**What happens when hash is unchanged**:
```
======================================================================
REFUSING promotion: hash unchanged.

You did not produce a new model. The hash matches the currently
pinned model in config. This prevents creating fake 'new' versions.

Options:
  1. Train with different data/parameters to get a new hash
  2. Do not bump the version number
  3. Use --allow-same-hash to override (use with caution)
======================================================================
```

### Step 6: Commit and push

```bash
git add ai_training/ippan_d_gbdt_v3.json
git add ai_training/model_card_ippan_d_gbdt_v3.toml
git add crates/ai_registry/models/ippan_d_gbdt_v3.json
git add config/dlc.toml
git commit -m "ai: promote GBDT v3 with strict hash verification"
git push origin master
```

**Important**: Do NOT commit CSV datasets (`ai_training/data/` is gitignored).

## Train the model

Train a deterministic GBDT model from the CSV dataset. The training script uses fixed seeds and deterministic LightGBM parameters to ensure reproducibility.

```bash
# Train v2 model (example)
python ai_training/train_ippan_d_gbdt.py --csv ai_training/data/ippan_training.csv --out ai_training/ippan_d_gbdt_v2.json

# Or use default paths
python ai_training/train_ippan_d_gbdt.py
```

**Output**: `ai_training/ippan_d_gbdt_v2.json` (deterministic integer-only JSON)

**Training parameters** (fixed for reproducibility):
- Random seed: 42
- LightGBM: `deterministic=True`, `num_threads=1`, `force_col_wise=True`
- Feature/bagging/data seeds: all set to 42
- Scale factor: 1,000,000 (for integer quantization)

## Compute BLAKE3 hash

Compute the canonical BLAKE3 hash of the model file for pinning:

```bash
# Python (recommended)
python -c "from blake3 import blake3; p='ai_training/ippan_d_gbdt_v2.json'; print(blake3(open(p,'rb').read()).hexdigest())"

# Or using b3sum (if installed)
b3sum ai_training/ippan_d_gbdt_v2.json
```

**Output**: 64-character hex string (e.g., `ac5234082ce1de0c52ae29fab9a43e9c52c0ea184f24a1e830f12f2412c5cb0d`)

## Create model card

Create or update `ai_training/model_card_ippan_d_gbdt_v2.toml`:

```toml
id = "ippan_d_gbdt_v2"
hash = "<computed_hash_from_above>"
scale = 1000000
feature_cols = ["uptime_ratio_7d", "validated_blocks_7d", "missed_blocks_7d", "avg_latency_ms", "slashing_events_90d", "stake_normalized", "peer_reports_quality"]
target_col = "fairness_score"
notes = "Training provenance and parameters..."
```

## Reproducibility

The entire pipeline is deterministic:

1. **Dataset generation**: Fixed seed (42) → same CSV every time
2. **Training**: Fixed seeds for LightGBM → same model JSON every time
3. **Hashing**: BLAKE3 of exact file bytes → same hash every time
4. **Runtime**: Hash verification ensures all nodes load identical bytes

**To retrain from scratch**:

```bash
# 1. Generate dataset (deterministic)
python ai_training/generate_synthetic_dataset.py

# 2. Train model (deterministic)
python ai_training/train_ippan_d_gbdt.py --csv ai_training/data/ippan_training.csv --out ai_training/ippan_d_gbdt_v2.json

# 3. Compute hash
python -c "from blake3 import blake3; p='ai_training/ippan_d_gbdt_v2.json'; print(blake3(open(p,'rb').read()).hexdigest())"

# 4. Update config and model card with hash
# 5. Vendor to crates/ai_registry/models/
# 6. Test and commit
```

## Notes

- Training is **OFFLINE ONLY**; runtime nodes never train
- Models are vendored in the repo (tracked in Git, LFS only if >100MB)
- Hash verification is **mandatory** at node startup (fail-fast)
- Deterministic behavior comes from: frozen JSON + integer-only inference + pinned hash
