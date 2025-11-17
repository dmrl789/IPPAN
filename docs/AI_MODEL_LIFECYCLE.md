# AI Model Lifecycle

This guide documents the deterministic lifecycle for producing, validating, and
shipping D-GBDT models that are consumed by `ai_registry` and
`consensus_dlc`.

## 1. Collect telemetry

* Aggregate validator telemetry from production or testnet nodes into a staging
  database.
* Export snapshots using the CSV schema defined in
  [`docs/AI_TRAINING_DATASET.md`](AI_TRAINING_DATASET.md). All values must be
  integers (micros/counts/atomic IPN) and rows must be sorted by
  `(validator_id, timestamp)`.

## 2. Prepare dataset

* Normalize the telemetry export into a CSV identical to
  `data/ai_training/sample_validator_telemetry.csv`.
* Verify sorting, column order, and that there are no missing rows.

## 3. Train offline

Use the trainer crate to fit a deterministic model:

```bash
cargo run -p ippan-ai-trainer -- \
  train \
  --dataset data/ai_training/sample_validator_telemetry.csv \
  --out models/dlc/dlc_model_example.json \
  --tree-count 64 \
  --max-depth 4 \
  --min-samples-leaf 8 \
  --learning-rate-micro 100000 \
  --quantization-step 10000
```

This command:

1. Loads the dataset and validates deterministic ordering.
2. Trains an integer-only ensemble with the provided hyper-parameters.
3. Writes canonical JSON to `--out`.
4. Prints the canonical BLAKE3 hash via `model_hash=<hex>`.

## 4. Verify model hash

* Record the `model_hash` printed by the CLI.
* Optionally re-run `ippan-ai-core` helpers to cross-check:

```bash
cargo test -p ippan-ai-core model_hash_tests -- --ignored
```

## 5. Update configuration

1. Place the canonical JSON file under `models/dlc/` (see `models/README.md`).
2. Update `config/dlc.toml`:

```toml
[dgbdt]
  [dgbdt.model]
  path = "models/dlc/dlc_model_example.json"
  expected_hash = "c549f359dc77fab3739e571d1d404143ac6c31f652588e8846e3770df8d63c26"
```

3. Commit the model and config updates together to keep hashes in sync.

## 6. Deploy and verify

* Restart nodes so that `ai_registry` reloads the model.
* During startup, `ai_registry` computes the canonical hash and compares it to
  `expected_hash`. A mismatch aborts activation.
* Once activated, `/ai/status` should report the new hash with `using_stub = false`.

## Determinism guarantees

* Training is performed **offline** and may use floating point intermediates,
  but the final `ippan-ai-core` model is quantized to integer fixed-point.
* Runtime inference is **pure integer math**. Every node that receives the same
  JSON model will produce identical scores.
* Canonical JSON + BLAKE3 hashing eliminates drift across architectures.

Following this lifecycle ensures that validator fairness models can be audited,
verified, and reproduced on demand.
