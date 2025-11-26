# Repository Guardrails

## Cursor AI Rules

Cursor IDE automatically applies rules from `.cursor/rules/*.md`:
- `IPPAN_CORE_RULES.md`: Core development rules (master-only, no floats in runtime)
- `IPPAN_AI_TRAINING_RULES.md`: AI training policy (offline only, LFS for large data)

## No Floats in Runtime Policy

Runtime and consensus Rust crates (`crates/*`) must never use `f32` or `f64`.

**Enforcement**: Opt-in pre-commit hook at `.githooks/pre-commit` scans `crates/` for floats (excluding tests/examples/benches).

**To enable the hook**:
```bash
./.githooks/install.sh
```

This sets `git config core.hooksPath .githooks` so Git uses the versioned hooks.

## Large Files Policy

GitHub rejects normal pushes for files >100MB. Large artifacts must be:

1. **Placed only in `ai_assets/`** (dedicated folder)
2. **Tracked with Git LFS**

**Setup Git LFS** (one-time):
```bash
git lfs install
```

**Usage**: Keep large datasets, model artifacts, and archives under `ai_assets/`. The `.gitattributes` file configures LFS tracking for common large file types in that folder.

**Note**: If Git LFS is not installed, you must install it on your machine before pushing large files.

## Runtime Model Artifacts

Runtime model artifacts (e.g., `crates/ai_registry/models/*.json`) are versioned and hash-verified:
- Models are vendored in the `ai_registry` crate for deterministic loading
- BLAKE3 hashes are pinned in code and verified at runtime
- Hash mismatches cause load failures (fail-fast security)
- Training scripts in `ai_training/` are **OFFLINE ONLY**; runtime never trains

### Strict Model Loading (v2)

The fairness model v2 is loaded via strict hash verification at node startup:

- **Location**: `crates/ai_registry/models/ippan_d_gbdt_v2.json` (vendored in repo)
- **Pinned Hash**: `ac5234082ce1de0c52ae29fab9a43e9c52c0ea184f24a1e830f12f2412c5cb0d` (set in `config/dlc.toml` under `[dgbdt.model]`)
- **Loader Function**: `ippan_ai_registry::d_gbdt::load_fairness_model_strict(path, expected_hash)`
- **Verification Steps**:
  1. Read model file bytes
  2. Compute BLAKE3 hash of raw bytes
  3. Compare to `expected_hash` (case-insensitive)
  4. If mismatch → return error immediately (fail-fast)
  5. Only then deserialize JSON and validate structure
- **Behavior**: Consensus initialization calls strict loader; if it fails, node startup aborts
- **Failure Mode**: Missing file, hash mismatch, or invalid JSON → node **FAILS FAST** (panic in non-test mode)
- **No Fallbacks**: There is no code path that silently continues without the strict model - consensus will not start without a valid, hash-verified model
- **Training Source**: v2 trained on synthetic dataset (deterministic seed 42); can be retrained from `ai_training/data/ippan_training.csv`

### Shadow Verifier Selection

Shadow verifier selection is score-ranked using the deterministic GBDT model v2:
- Features are fixed-point i64 scaled by 1_000_000 (SCALE)
- Model scores all validators using 7 features: uptime_ratio_7d, validated_blocks_7d, missed_blocks_7d, avg_latency_ms, slashing_events_90d, stake_normalized, peer_reports_quality
- Shadows are selected deterministically by highest score first, with validator ID as tie-breaker
- Model is hash-verified at startup; mismatch => node refuses to start

### Reward Weighting

Reward distribution is weighted by fairness model v2 score:
- Multiplier cap 0.8x–1.2x (MIN_MULT/MAX_MULT) applied to validator scores
- Weights normalized to keep total payout unchanged (sum(weights) == SCALE)
- Deterministic remainder distribution with tie-break by validator ID
- Strict model hash verification remains startup-critical (no fallback)

