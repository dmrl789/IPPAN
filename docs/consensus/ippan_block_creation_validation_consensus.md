# IPPAN — Block Creation, Validation, zk-STARK Verification, and Consensus

## 1. Implementation overview

IPPAN’s consensus stack is implemented across the Rust crates that live in this
repository:

- `crates/types` defines the canonical [`Block`](../../crates/types/src/block.rs),
  [`Transaction`](../../crates/types/src/transaction.rs), and
  [`HashTimer`](../../crates/types/src/hashtimer.rs) types.
- `crates/consensus` provides a Proof-of-Authority (PoA) round loop,
  deterministic ordering utilities, and the [`ParallelDag`](../../crates/consensus/src/parallel_dag.rs)
  primitives used for high-throughput block intake.
- `crates/crypto` houses the confidential transaction verifier. Its
  [`zk_stark`](../../crates/crypto/src/zk_stark.rs) module exposes the STARK
  verifier that powers confidentiality guarantees.
- `crates/mempool`, `crates/storage`, and `crates/time` supply the transaction
  queue, persistent ledger, and deterministic time base that the consensus
  engine consumes.

The sections below connect those modules into a single creation → validation →
finalization pipeline and highlight where zk-STARK verification happens.

---

## 2. Block model and HashTimer anchors

Every block produced by the validator loop is an instance of
[`types::block::Block`](../../crates/types/src/block.rs). Its header embeds a
`HashTimer` (`header.hashtimer`) that anchors the block to deterministic IPPAN
Time, the parent set, payload merkle summaries, and optional receipt/state
roots.

```rust
pub struct BlockHeader {
    pub id: BlockId,
    pub creator: ValidatorId,
    pub round: RoundId,
    pub hashtimer: HashTimer,
    pub parent_ids: Vec<BlockId>,
    pub payload_ids: Vec<[u8; 32]>,
    pub merkle_payload: [u8; 32],
    pub merkle_parents: [u8; 32],
    // ... state/receipt metadata elided
}
```

`Block::new` deterministically derives the HashTimer by hashing the round, the
creator key, parent identifiers, and the ordered payload roots. The `HashTimer`
domain separation strings (`"block"`) and nonce derivation logic inside
`Block::derive_hashtimer_nonce` guarantee that independent validators derive the
same temporal anchor for the same inputs.

---

## 3. Block creation pipeline

### 3.1 Slot-driven proposal loop

`PoAConsensus::start` (in `crates/consensus/src/lib.rs`) spins an async task that
rotates proposers every `slot_duration_ms`. When a validator’s identifier
matches the active slot, `propose_block` is invoked:

```rust
let block = Block::new(
    parent_ids,
    block_transactions,
    latest_height + 1,
    proposer_id,
);
```

Parent selection prefers the previous round’s committed blocks tracked inside
`RoundTracker`. If no prior round data exists the engine falls back to the last
persisted block height in storage.

The mempool contribution is bounded by
`PoAConfig::max_transactions_per_block` so that block construction cost is
predictable and parallel validation stays within CPU budgets.

### 3.2 Parallel DAG utilities

While `propose_block` currently emits a single block per slot, the
`ParallelDagEngine` (also in `crates/consensus/src/parallel_dag.rs`) provides a
ready-made path to fan out large transaction batches:

```rust
let blocks = ParallelDagEngine::new(storage.clone())
    .create_blocks_parallel(pending_txs, max_txs_per_block, round, parent_ids, proposer);
```

Internally the engine partitions transactions via Rayon, allowing independent
worker threads to materialize candidate blocks that share the same round and
parent set. The same engine exposes `process_pending_transactions` which
pipelines creation, validation, and persistence using background Tokio workers
for storage I/O.

---

## 4. Structural validation and mempool hygiene

Before a proposed block is persisted, the consensus loop performs a series of
structural checks:

1. `Block::is_valid` enforces header invariants (merkle roots, parent list
   consistency, signature presence).
2. `validate_confidential_block` (re-exported from `crates/crypto`) walks every
   transaction to ensure that required envelopes and zero-knowledge proofs are
   present.
3. If validation succeeds the block is stored and all included transactions are
   pruned from the mempool via `cleanup_mempool` so that replay and double spend
   attempts are eliminated for the next round.

Any validation error aborts the proposal and leaves the mempool untouched.

---

## 5. zk-STARK verification

`crates/crypto/src/confidential.rs` is responsible for verifying the
cryptographic proofs that accompany confidential transactions. The `Stark`
variant of `ConfidentialProofType` dispatches to `validate_stark_proof`, which:

1. Decodes the base64 proof blob and parses the public input map.
2. Recomputes the canonical transaction identifier (`Transaction::message_digest`)
   to ensure the proof binds to the payload.
3. Verifies sender and receiver commitments derived from the transaction body.
4. Ensures the Fibonacci trace length is a supported power of two.
5. Constructs a `StarkProof` and calls `verify_fibonacci_proof` from
   `zk_stark.rs`.

If any check fails, `ConfidentialTransactionError` bubbles up and the enclosing
block is rejected by the proposer. Because verification runs inside Rayon
iterators (`block.transactions.par_iter()`), all proofs within a block can be
checked concurrently.

---

## 6. Deterministic ordering and conflict handling

Once enough blocks accumulate for a round, `finalize_round_if_ready` aggregates
and orders them. It delegates to `order_round` (`crates/consensus/src/ordering.rs`),
which sorts blocks lexicographically by `(HashTimer.time_prefix, creator, id)`
and then flattens transaction payloads. The helper deduplicates transaction
hashes, reports conflicts through a callback, and ensures that parents referenced
within the round actually exist in storage.

Conflicting payloads are recorded in the round’s `fork_drops` list. Validators
log each conflict with the offending transaction hash to support offline audits
and remediation.

---

## 7. Round finalization and persistence

When a round satisfies the finalization interval, the consensus engine builds a
`RoundCertificate` and `RoundFinalizationRecord`:

- `RoundCertificate` contains the round identifier, the ordered block ids, and a
  placeholder aggregate signature (`aggregate_round_signature`) that will later
  be replaced by BLS or VRF attestations.
- `RoundFinalizationRecord` combines the window start/end timestamps (`RoundWindow`),
  the ordered transactions, dropped conflicts, and the deterministic state root
  derived from hashing the previous state root, block ids, ordered tx ids, and
  conflicts.

Both artifacts are persisted through the storage trait so that other components
(e.g. RPC, analytics, archival nodes) can serve finalized state snapshots.

---

## 8. Parallel DAG metrics and observability

`ParallelDag` maintains per-vertex metadata and a lock-free ready queue. Its
`DagMetricsSnapshot` exposes counters for inserted vertices, ready transitions,
committed nodes, queue overflows, duplicates, and orphan commits. `snapshot()`
returns aggregate width/depth estimates that can be wired into telemetry so
operators understand backlog health and tuning opportunities for `max_parents`
and `ready_queue_bound`.

---

## 9. End-to-end pipeline summary

| Stage | Module / Function                                                                 | Parallelism                                   | zk-STARK involvement                |
| ----- | ---------------------------------------------------------------------------------- | --------------------------------------------- | ----------------------------------- |
| 1     | `PoAConsensus::propose_block`                                                      | Sequential per slot                           | —                                   |
| 2     | `ParallelDagEngine::create_blocks_parallel` / `process_pending_transactions`      | Rayon workers + Tokio blocking pool           | Proof verification per candidate    |
| 3     | `validate_confidential_block` → `validate_stark_proof`                             | Rayon `par_iter` over transactions            | Verifies STARK commitments          |
| 4     | `cleanup_mempool`                                                                  | Sequential                                    | —                                   |
| 5     | `order_round`                                                                      | CPU-bound sorting + conflict callbacks        | Proofs already checked              |
| 6     | `aggregate_round_signature` + `RoundFinalizationRecord` persistence                | Sequential hashing, storage writes on threads | Final state reflects verified blocks |

---

## 10. Summary

IPPAN’s PoA consensus engine coordinates deterministic block creation with
HashTimer anchors, validates every payload (including confidential transactions)
in parallel, and persists finalized rounds together with the data needed for
cryptographic audits. zk-STARK verification is a first-class step in the
validation pipeline, ensuring that confidentiality features do not weaken
consensus safety while preserving high throughput through parallel execution.
