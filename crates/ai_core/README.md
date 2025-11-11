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

## Cross-Compiling (aarch64)
- Install the Rust standard library for the target with `rustup target add aarch64-unknown-linux-gnu`.
- If you are compiling locally (without `cross`), install an aarch64 linker toolchain (e.g., `gcc-aarch64-linux-gnu`) and run `cargo test --no-run -p ippan-ai-core --features deterministic_math --target aarch64-unknown-linux-gnu`.
- Alternatively install [`cross`](https://github.com/cross-rs/cross) and ensure Docker or Podman is available, then run `cross test -p ippan-ai-core --target aarch64-unknown-linux-gnu`.
- The deterministic suite now emits a SHA-256 digest for a realistic validator telemetry scenario; both x86_64 and aarch64 golden references must remain identical.
