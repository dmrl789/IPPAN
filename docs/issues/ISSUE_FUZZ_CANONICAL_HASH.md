# P0: Fuzz/property tests for canonical hashing + proof parsing

## Priority: P0 (Release Blocker)

## Description
Add fuzz targets and property tests for critical components: canonical hashing, proof bundles, and parsers.

## Target Crates
- `crates/rpc` - RPC message parsing
- `crates/ai_core` - canonical hashing, model parsing
- `crates/consensus_dlc` - proof bundles, DLC messages
- `crates/consensus` - block/transaction parsing

## Requirements
- Add `cargo-fuzz` targets or `proptest` property tests
- CI job for short fuzz smoke tests (PR gate, < 5 min)
- Nightly long fuzz runs (2+ hours)
- Coverage reporting for fuzz targets

## Acceptance Criteria
- [ ] Fuzz targets for canonical hashing (model, block, tx)
- [ ] Property tests for proof bundle round-trip
- [ ] Fuzz tests for RPC and consensus parsers
- [ ] CI runs short fuzz on PRs
- [ ] Nightly long fuzz runs with crash reports

## Related
See `docs/READINESS_100_PLAN.md` section B.2 for full details.

