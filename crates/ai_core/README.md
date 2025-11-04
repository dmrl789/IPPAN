# IPPAN AI Core

## Overview
- Provides deterministic, integer-only model execution for L1 validator scoring.
- Bundles feature extraction, gradient boosted tree evaluation, and runtime guard rails.
- Designed to keep AI outputs reproducible across consensus nodes.

## Key Modules
- `features` and `feature_engineering`: build telemetry feature vectors and statistics.
- `deterministic_gbdt` and `gbdt`: evaluate gradient boosted models without floating point drift.
- `model_manager` and `models`: package, verify, and hot swap production models.
- `production_config` and `deployment`: define resource limits and deployment workflows.
- `monitoring`, `health`, and `log`: expose metrics, health checks, and audit traces.

## Integration Notes
- Consume `compute_validator_score` from `lib.rs` for consensus scoring.
- Ship models through `ModelManager` to enforce hash verification and versioning.
- Use the provided `tests` suite as reference when onboarding new model artifacts.
