# D-GBDT Training Spec (Devnet)

This spec defines the **devnet dataset contract**, the **offline training pipeline**, and the **promotion/versioning** flow for the deterministic D-GBDT fairness model.

**Scope**: docs-only contract for auditors + future implementation work.  
**Non-goal**: runtime training. IPPAN nodes **never train** models.

---

## 1. Data sources

- **Location on nodes**: `/var/lib/ippan/ai_datasets/devnet_dataset_*.csv.gz`
- **Exporter**:
  - Canonical exporter script: `ai_training/export_localnet_dataset.py`
  - Canonical devnet wrapper: `ops/devnet/export-dataset.sh`
  - Example invocation:
    - `./ai_training/export_localnet_dataset.py --mode rpc --rpc http://127.0.0.1:8080`

### Transport to trainer (chosen)

**Chosen transport**: **rsync over SSH** from a single devnet node (dataset source-of-truth) to the offline trainer machine.

- Rationale:
  - deterministic file transfer (byte-identical)
  - resumable + incremental
  - easy to audit (single source node, append-only dataset directory)

Example:

```bash
# On the trainer machine:
export SRC_NODE="root@<DEVNET_NODE_IP>"
export SRC_DIR="/var/lib/ippan/ai_datasets/"
export DST_DIR="./ai_assets/datasets/devnet/"

mkdir -p "$DST_DIR"
rsync -avz --progress "${SRC_NODE}:${SRC_DIR}devnet_dataset_*.csv.gz" "${DST_DIR}"
```

---

## 2. Dataset schema (columns)

**File format**

- **Encoding**: CSV with header, UTF-8
- **Delimiter**: `,`
- **Compression**: gzip (`.csv.gz`)
- **Line endings**: `\n` recommended (Windows `\r\n` tolerated by pandas)

**Column contract (exact names)**

The exporter writes the following columns (see `ai_training/export_localnet_dataset.py` `CSV_COLUMNS`):

- `timestamp_utc` (**string**, RFC3339/ISO 8601 UTC): collection timestamp for the sample.
- `round_id` (**int**, non-negative): consensus round observed at sampling time.
- `validator_id` (**string**, lowercase hex, 64 chars): stable validator identifier.

Feature columns (inputs to the trainer; **integer-only** in CSV):

- `uptime_ratio_7d` (**int**, fixed-point): uptime ratio scaled by `SCALE=1_000_000`, range `[0, 1_000_000]`.
- `validated_blocks_7d` (**int**, fixed-point count): `blocks_verified * SCALE`. Non-negative.
- `missed_blocks_7d` (**int**, fixed-point count): `missed_count * SCALE`. Non-negative.
- `avg_latency_ms` (**int**, fixed-point): normalized latency (ms proxy) scaled by `SCALE`. Non-negative.
- `slashing_events_90d` (**int**, count): slashing event count, non-negative.
- `stake_normalized` (**int**, fixed-point): normalized stake scaled by `SCALE`, range `[0, 1_000_000]`.
- `peer_reports_quality` (**int**, fixed-point): peer quality score scaled by `SCALE`, range `[0, 1_000_000]`.

Target column (label):

- `fairness_score_scaled` (**int**, fixed-point): target fairness score scaled by `SCALE`, range `[0, 1_000_000]`.
  - The trainer also accepts `fairness_score` as a float and converts it, but **devnet datasets MUST emit `fairness_score_scaled`** for auditability and to avoid float parsing ambiguity.

### Missing value handling

- **No missing values allowed** for the feature/target columns.
- If any required cell is missing/empty or non-integer:
  - **drop the row** during offline preprocessing, and
  - **record the drop count** in training metadata (see §6).

### Fixed-point scaling (no floats at runtime)

Runtime consensus scoring is integer-only. Therefore:

- All ratio-like columns (`uptime_ratio_7d`, `stake_normalized`, `peer_reports_quality`, `fairness_score_scaled`) are represented as **micro fixed-point ints** with `SCALE=1_000_000`.
- Any preprocessing that involves floats must happen **offline only**, and the resulting dataset/model artifacts must be integer-only.

---

## 3. Preprocessing rules (offline)

These steps are **train-time only** and MUST NOT be required at runtime.

### Required filtering (MUST)

Drop any row that violates:

- missing required columns
- non-integer in any integer column (feature/target columns)
- negative values where not permitted (all feature/target columns must be `>= 0`)
- out-of-range fixed-point ratios (`< 0` or `> SCALE`) for:
  - `uptime_ratio_7d`, `stake_normalized`, `peer_reports_quality`, `fairness_score_scaled`

### Optional normalization (MAY)

Because the current pipeline trains LightGBM with fixed seeds and then quantizes to integer thresholds, **feature scaling is not required** if the dataset already uses the canonical fixed-point scheme.

If additional normalization is introduced later:

- it must be deterministic (pure function of the dataset)
- it must result in integer columns
- it must be documented and versioned in the trainer (commit SHA recorded)

### Outlier handling (MAY)

If outlier clipping is used, it must be deterministic and integer-based, e.g.:

- clip `avg_latency_ms` to `[0, P99]` computed on the training split
- clip `validated_blocks_7d` / `missed_blocks_7d` to `[0, P99]`

If used, record:

- clipping thresholds
- fraction of rows affected

---

## 4. Model format & determinism

### Model format

- **Model format**: JSON-serialized deterministic D-GBDT model (canonical JSON)
- **Runtime expectation**: strict hash-verified JSON load (see `docs/REPO_GUARDRAILS.md`)

The canonical model artifact is produced by the trainer with:

- stable key ordering (`sort_keys=true`)
- stable separators (`separators=(",", ":")`)
- integer-only thresholds/leaves (fixed-point)

### Determinism rules

The training pipeline MUST pin all sources of non-determinism:

- fixed `RANDOM_SEED` (canonical value: `42`)
- exact trainer + dependency versions
  - Python + LightGBM version must be pinned (see `ai_training/requirements.lock.txt`)
- single-threaded training for determinism
  - e.g. LightGBM `n_jobs=1`, `deterministic=True`, `force_col_wise=True`

### Hashing

The hash used for runtime pinning and audit identity is:

- `model_hash = blake3(model_bytes)`
  - where `model_bytes` are the exact bytes of the canonical JSON file written by the trainer

**Storage/visibility**

- Stored in `config/dlc.toml` under `[dgbdt.model].expected_hash`
- Visible via RPC: `GET /ai/status` → `model_hash` (see `docs/AI_STATUS_API.md`)
- Verified in CI and locally via:
  - `cargo run -p ippan-ai-core --bin verify_model_hash -- config/dlc.toml`

---

## 5. Versioning + promotion

### Model ID

- **Model ID**: `ippan_d_gbdt_devnet_vN` where `N = 1,2,3…`
- **Artifact file name (recommended)**: `ippan_d_gbdt_devnet_vN.json`

### Dataset identity

Datasets are identified by:

- **dataset file name(s)**: `devnet_dataset_YYYYMMDDTHHMMSSZ.csv.gz`
- **dataset hash** (recommended): `dataset_hash = blake3(dataset_gzip_bytes)`

Training metadata MUST record the **ordered list** of dataset file names and their hashes.

### Promotion stages (devnet)

1. **Train offline** on devnet datasets (exported from `/var/lib/ippan/ai_datasets/`).
2. **Offline evaluation**:
   - metrics (at minimum: validation MSE for regression; optionally add AUC/calibration if a classifier is adopted)
   - compare vs current active devnet model
3. **Determinism check**:
   - train on at least two platforms (x86_64 and aarch64)
   - require `model_hash` identical across platforms
4. **Shadow scoring on devnet**:
   - run new model side-by-side for ≥24h (**no decision impact**)
   - compare score distributions and top-k selection stability
5. **Go / no-go**:
   - if OK → promote to active on devnet by pinning `expected_hash` in `config/dlc.toml` and rolling out
   - if not → keep current model and record reasons

---

## 6. Audit metadata

Every promoted model MUST have a model record under:

- `docs/ai/models/ippan_d_gbdt_devnet_vN.md`

The record MUST include:

- model_id
- model_hash (BLAKE3 hex)
- trainer commit (git SHA)
- trainer script + version (e.g. `ai_training/train_ippan_d_gbdt.py`)
- training datasets:
  - dataset filenames and (recommended) dataset hashes
- training date (UTC)
- metrics (and baseline comparison)
- determinism evidence:
  - platforms tested + hashes observed
- deployment:
  - promoted stage (devnet) + date
  - rollback plan


