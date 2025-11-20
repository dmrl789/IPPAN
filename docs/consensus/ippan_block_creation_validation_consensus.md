# IPPAN — Block Creation, Validation, zk-STARK Verification, and Consensus

## 1. Implementation overview

IPPAN’s consensus stack is implemented across the Rust crates that live in this
repository:

- `crates/types` defines the canonical [`Block`](../../crates/types/src/block.rs),
  [`Transaction`](../../crates/types/src/transaction.rs), and
  [`HashTimer`](../../crates/types/src/hashtimer.rs) types.
- `crates/consensus` provides the Deterministic Learning Consensus (DLC)
  round loop (with PoA kept only as a compatibility fallback),
  deterministic ordering utilities, and the
  [`ParallelDag`](../../crates/consensus/src/parallel_dag.rs)
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
