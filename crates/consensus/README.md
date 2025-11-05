# IPPAN Consensus

## Overview
- Implements the hybrid PoA and Deterministic Learning Consensus (DLC) engine.
- Coordinates HashTimer windows, BlockDAG ordering, validator bonding, and DAG-Fair emission.
- Integrates AI-driven proposer selection and telemetry-backed reputation tracking.

## Key Modules
- `dlc`, `dlc_integration`, and `round`: manage round state, proposer logic, and DAG ordering.
- `dgbdt`, `l1_ai_consensus`, and `telemetry`: score validators with deterministic AI inputs.
- `bonding`, `emission`, `emission_tracker`, and `fees`: enforce staking, rewards, and fee recycling.
- `parallel_dag`, `ordering`, and `metrics`: finalize rounds and expose runtime metrics.

## Integration Notes
- Use `PoAConsensus` from `lib.rs` for node orchestration and round lifecycle.
- Feature flag `ai_l1` enables AI-based proposer selection; default path is deterministic fallback.
- Provide `ippan_storage` and `ippan_mempool` implementations to mirror production behavior.
