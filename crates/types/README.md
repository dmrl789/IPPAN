# IPPAN Types

## Overview
- Centralizes the canonical data structures shared across IPPAN crates.
- Re-exports block, transaction, round, L2, and currency types under a single crate.
- Couples closely with `ippan_time` for HashTimer and time services.

## Key Modules
- `block`, `transaction`, and `receipt`: DAG data models and transaction envelopes.
- `round` and `snapshot`: consensus round metadata and state capture helpers.
- `l2`: Layer 2 network descriptors, commits, and exits.
- `currency`: amount utilities, denominations, and supply constants.
- `time_service`: adapters for HashTimer-based time synchronization.

## Integration Notes
- Import `ippan_types` in application crates to share consistent structures and helper functions.
- Use `Amount` helpers to convert between atomic units and human-readable denominations.
- Leverage `snapshot` types when exporting state to explorers or archival systems.
