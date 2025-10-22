# Fees and Emission

This document summarizes the deterministic, integer-only logic used for protocol fees and DAG-Fair round emission.

## Emission (Round-based)

- Emission per round is computed with integer halving:
  - `R(t) = R0 >> floor(t / HALVING_ROUNDS)`
- The split follows a proposer bonus and verifier remainder:
  - Proposer receives `PROPOSER_BONUS_BPS` basis points of the pool
  - The remaining pool is split equally across verifiers; any remainder is recycled
- Implementation: see `crates/consensus/src/emission.rs`
  - `EmissionParams { r0, halving_rounds, proposer_bonus_bps }`
  - `round_reward(round, &params)` – per-round pool
  - `split_proposer_and_verifiers(pool, verifier_count, &params)` – 20/80 by default
  - `per_block_slice(pool, block_count)` – equal per-block slice + remainder

## Fee Caps (Protocol-level)

- Transactions have an estimated fee derived from their size-like features.
- A hard cap `MAX_FEE_PER_TX` is enforced at:
  - Mempool admission (reject over-cap)
  - Consensus block proposal (filter over-cap)
- The estimator is deterministic and mirrors the mempool logic to avoid cycles.
- Implementation:
  - `crates/mempool/src/lib.rs` – admission check with the local constant
  - `crates/consensus/src/fees.rs` – estimator and `within_cap(tx)` for proposal-time filtering

## Parameters

- Example defaults (tunable via code/config):
  - `R0`: 10_000 (µIPN per round, example)
  - `HALVING_ROUNDS`: 100 (example)
  - `PROPOSER_BONUS_BPS`: 2000 (20%)
  - `MAX_FEE_PER_TX`: 10_000_000 (example cap)

All logic is integer-only and architecture-independent to preserve determinism across nodes.
