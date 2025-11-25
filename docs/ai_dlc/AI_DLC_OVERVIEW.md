# AI & DLC Overview

This folder contains deterministic AI (GBDT) and Discreet Log Contract (DLC) documentation used by consensus and auditors.

## Deterministic AI / D-GBDT
- [Deterministic math implementation](./AI_CORE_DETERMINISTIC_MATH_IMPLEMENTATION.md) explains fixed-point arithmetic and hashing rules that avoid nondeterminism.
- [AI features readme](./AI_FEATURES_README.md) lists supported model capabilities and configuration knobs.
- [Registry and production improvements](./AI_REGISTRY_IMPROVEMENTS.md) summarize operational hardening for AI crates.
- [Consensus GBDT analysis and fixes](./CONSENSUS_GBDT_ANALYSIS.md) and [deterministic fixes](./DETERMINISTIC_GBDT_FIXES.md) capture major iterations.

## DLC Integration
- [DLC status](./CONSENSUS_DLC_STATUS.md) and [migration complete](./DLC_MIGRATION_COMPLETE.md) outline how DLC hooks into consensus.
- [DLC implementation summary](./CURSOR_DLC_IMPLEMENTATION_SUMMARY.md) highlights API surfaces and message formats.
- [Quickstart for DLC flows](./QUICKSTART_DLC.md) and [production GBDT README](./PRODUCTION_GBDT_README.md) provide operator-facing notes.

## Reports

Dated reproducibility and simulation reports are preserved for auditors in [`../archive/2025_rc1`](../archive/2025_rc1/):
- AI determinism repro/X86 runs (2025-11-24)
- ACT DLC simulation reports (2025-11-24)

## See Also
- [DLC implementation complete](./DLC_IMPLEMENTATION_COMPLETE.md)
- [AI determinism CI fixes](./AI_DETERMINISM_CI_FIX.md) and [AI determinism fix summary](./AI_DETERMINISM_FIX_SUMMARY.md)
- [AI crates production improvements](./AI_CRATES_PRODUCTION_IMPROVEMENTS.md)
