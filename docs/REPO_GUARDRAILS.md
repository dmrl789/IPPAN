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

### Strict Model Loading (v1)

The fairness model v1 is loaded via `ippan_ai_registry::d_gbdt::get_active_fairness_model_strict()`:
- **Location**: `crates/ai_registry/models/ippan_d_gbdt_v1.json`
- **Pinned Hash**: `ac5234082ce1de0c52ae29fab9a43e9c52c0ea184f24a1e830f12f2412c5cb0d`
- **Behavior**: Consensus initialization calls the strict loader at startup
- **Failure Mode**: If the model file is missing, hash mismatches, or JSON cannot be deserialized, the node **FAILS FAST** (startup/init returns error)
- **No Fallbacks**: There is no code path that silently continues without the strict model - consensus will not start without a valid, hash-verified model

### Shadow Verifier Selection

Shadow verifier selection is score-ranked using the deterministic GBDT model v1:
- Features are fixed-point i64 scaled by 1_000_000 (SCALE)
- Model scores all validators using 7 features: uptime_ratio_7d, validated_blocks_7d, missed_blocks_7d, avg_latency_ms, slashing_events_90d, stake_normalized, peer_reports_quality
- Shadows are selected deterministically by highest score first, with validator ID as tie-breaker
- Model is hash-verified at startup; mismatch => node refuses to start

### Reward Weighting

Reward distribution is weighted by fairness model v1 score:
- Multiplier cap 0.8x–1.2x (MIN_MULT/MAX_MULT) applied to validator scores
- Weights normalized to keep total payout unchanged (sum(weights) == SCALE)
- Deterministic remainder distribution with tie-break by validator ID
- Strict model hash verification remains startup-critical (no fallback)

