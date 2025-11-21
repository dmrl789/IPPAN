# Comprehensive Testing — Phase 1

This document tracks the first phase of comprehensive testing for IPPAN. It focuses on deterministic invariants for timekeeping, DLC consensus behavior over long runs, and storage replay integrity.

## New test coverage

### IPPAN Time / HashTimer
- Monotonicity under mixed peer offsets and enforcement of median drift bounds.
- Skew rejection for offsets beyond the accepted window without polluting the sample set.
- Clamping guarantees ensuring `now_us()` never returns a negative timestamp.
- Stable HashTimer ordering and signature validation across sequences of timers.

### DLC long-run simulation
- Deterministic 256-round simulation with a fixed validator set and seed.
- Asserts emission and reward totals match cumulative block rewards.
- Verifies reputation scores stay within normalized bounds and every validator accrues rewards.

### Storage replay & snapshots
- Linear multi-block chain applied to storage with chain-state updates.
- Snapshot export/import round-trip validated against deterministic re-application from genesis.
- State roots, heights, and round counters must match after replay.

## How to run

```bash
# Time / HashTimer invariants
cargo test -p ippan-time -- --nocapture

# DLC long-run emission + fairness invariants
cargo test -p ippan-consensus-dlc --test emission_invariants -- --nocapture

# Storage replay and snapshot round-trip
cargo test -p ippan-storage --test replay_roundtrip -- --nocapture
```

## Next steps (Phase 2)
- Add fuzzing/property-based testing for consensus and networking paths.
- Schedule longer-duration stress runs on RC testnets to observe live behavior.
