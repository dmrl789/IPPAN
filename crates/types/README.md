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

## Testing

### Comprehensive Test Suite
This crate includes **66 tests** covering:
- ✅ Transaction creation, signing, and validation
- ✅ Block construction and validation with merkle proofs
- ✅ Round windows, certificates, and finalization records
- ✅ **Serialization/deserialization consistency** (13 round-trip tests)
- ✅ HashTimer determinism and temporal ordering
- ✅ Confidential transactions with ZK proofs
- ✅ Currency operations and denominations

### Run Tests
```bash
# All tests
cargo test --package ippan-types

# Only serialization tests
cargo test --package ippan-types serialization_tests

# With output
cargo test --package ippan-types -- --nocapture
```

### Test Documentation
See [TESTING.md](./TESTING.md) for detailed test documentation.

### Serialization Validation
All core types have been validated for serialization consistency:
- **Transactions**: Public and confidential, with topics and proofs
- **Blocks**: With all optional fields (erasure_root, receipt_root, state_root, validator_sigs)
- **Rounds**: Windows, certificates, and finalization records
- **Determinism**: Repeated serialization produces identical output (critical for consensus)

For a complete serialization validation report, see [SERIALIZATION_VALIDATION_SUMMARY.md](../../SERIALIZATION_VALIDATION_SUMMARY.md).

## Production Readiness
- ✅ All tests passing (66/66)
- ✅ Zero clippy warnings
- ✅ Deterministic serialization verified
- ✅ Edge cases covered (empty blocks, max values, optional fields)
- ✅ Backward compatibility via `#[serde(default)]`
- ✅ Forward compatibility via `#[serde(skip_serializing_if)]`
