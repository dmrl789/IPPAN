# IPPAN D-GBDT Training Bootstrap (Offline Only)

This guide describes how to generate a synthetic dataset and train the first IPPAN D-GBDT fairness model on a **separate, trusted machine** (e.g., Hetzner or local Linux). IPPAN nodes **never train models**; they only load the frozen JSON artifact and perform deterministic integer inference.

## Environment setup

```bash
# Clone the repository and enter it
git clone https://github.com/dmrl789/IPPAN.git
cd IPPAN

# Create and activate a Python virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install required Python dependencies
pip install "numpy==1.26.4" "pandas==2.2.2" "scikit-learn==1.5.2" "lightgbm==4.3.0"
```

## Generate the training dataset

### Option 1: Synthetic dataset (bootstrap)

This uses a fixed RNG seed and writes `data/ippan_gbdt_training.csv`.

```bash
python ai_training/generate_synthetic_dataset.py
```

### Option 2: Localnet dataset (real-world proxy)

Export validator metrics from a running localnet to generate training data:

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

3. **Train as usual** (see below).

**Note**: Localnet exports produce "proxy 7d" features (windowed deltas from current metrics) suitable for bootstrap/testing. For production training, use longer collection periods or aggregate historical data from testnet/mainnet.

## Train the bootstrap fairness model

This trains LightGBM offline, quantizes leaves to integers, and writes `ai_training/ippan_d_gbdt_v1.json`.

```bash
python ai_training/train_ippan_d_gbdt.py
```

## Compute the canonical model hash (BLAKE3)

Use `b3sum` (or another BLAKE3 tool) to compute the hash for pinning.

```bash
b3sum ai_training/ippan_d_gbdt_v1.json
```

Record the hash and update `ai_training/model_card_ippan_d_gbdt_v1.toml` by replacing the placeholder value. Store both the JSON and hash securely for later governance/consensus pinning.

## Next steps (manual, offline)

- Copy `ai_training/ippan_d_gbdt_v1.json` and its BLAKE3 hash to a safe location.
- Later, governance/DLC configuration will pin the `id` and `hash` so all nodes load the same model.
- This pipeline is intentionally outside CI and must be run manually in a trusted environment.
- Deterministic behavior in IPPAN comes from freezing the JSON, using integer-only inference, and hashing the artifact; runtime nodes **do not** perform any training.
