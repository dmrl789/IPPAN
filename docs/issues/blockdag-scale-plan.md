# BlockDAG Scale Initiative — Issue Breakdown

This document enumerates the issue cards to open for Codex in order to evolve the IPPAN node from the current single-thread PoA implementation toward the PRD target of 3–10 million TPS. Each issue includes a recommended title, scope summary, key file paths, labels, and an initial branch name. All issues should carry the parent label `initiative/blockdag-scale` once created.

---

## 1. Convert Node Runtime to Async Tokio Core
- **Proposed issue title:** `codex: migrate node runtime to async tokio core`
- **Branch name:** `codex/async-runtime`
- **Labels:** `codex`, `p0`, `initiative/blockdag-scale`
- **Summary:** Refactor node, consensus, storage, and networking crates to run on a unified Tokio runtime. Replace blocking loops with async tasks and wrap sled access via `tokio::task::spawn_blocking` until storage v2 lands.
- **Key paths:**
  - `crates/node/src/main.rs`
  - `crates/consensus/`
  - `crates/network/`
  - `crates/storage/`
- **Acceptance checks:** Tokio runtime boot, async block proposal loop, no regressions in `cargo test`.

## 2. Introduce Concurrent DAG Data Structure
- **Proposed issue title:** `codex: add dag crate and parallel block scheduler`
- **Branch name:** `codex/dag-scheduler`
- **Labels:** `codex`, `p0`, `initiative/blockdag-scale`
- **Summary:** Create `crates/dag/` with a concurrent adjacency structure that supports multi-parent vertices, ensures acyclicity via HashTimer ordering, and exposes APIs for the consensus engine. Replace the single queue block dispatcher with the DAG scheduler.
- **Key paths:**
  - `crates/dag/`
  - `crates/consensus/src/engine.rs`
- **Acceptance checks:** Unit tests covering DAG insertions, cycle prevention, and deterministic ordering snapshots.

## 3. Overhaul Networking with libp2p QUIC Gossip
- **Proposed issue title:** `codex: implement libp2p gossip network`
- **Branch name:** `codex/p2p-gossip`
- **Labels:** `codex`, `p1`, `initiative/blockdag-scale`
- **Summary:** Replace the HTTP polling peer layer with a QUIC-based libp2p stack. Add transaction and block gossipsub topics, binary serialization via `bincode`, and async handlers.
- **Key paths:**
  - `crates/p2p/`
  - `crates/network/`
  - `crates/node/src/config.rs`
- **Acceptance checks:** Integration test that propagates blocks across three nodes with QUIC transport.

## 4. Lock-Free Global Mempool and Parallel Block Builder
- **Proposed issue title:** `codex: implement lock-free global mempool`
- **Branch name:** `codex/global-mempool`
- **Labels:** `codex`, `p1`, `initiative/blockdag-scale`
- **Summary:** Replace the monolithic mempool with a lock-free global queue (e.g., crossbeam ring buffer) that supports multi-producer/multi-consumer batching, preserves HashTimer order, and enables concurrent block assembly without explicit sharding.
- **Key paths:**
  - `crates/mempool/`
  - `crates/consensus/src/block_builder.rs`
- **Acceptance checks:** Benchmarks demonstrating ≥500k tx/sec ingestion on a single node with deterministic ordering and no contention regressions under synthetic 3M TPS load.

## 5. Storage v2: Batched WAL and Parallel Execution
- **Proposed issue title:** `codex: introduce storage_v2 wal with parallel state apply`
- **Branch name:** `codex/storage-v2`
- **Labels:** `codex`, `p1`, `initiative/blockdag-scale`
- **Summary:** Create `crates/storage_v2/` implementing a batched write-ahead log on RocksDB or Redb, binary serialization, shard-aware state application via Rayon, and epoch snapshots every 1000 blocks.
- **Key paths:**
  - `crates/storage_v2/`
  - `crates/node/src/state/`
- **Acceptance checks:** Replay tests that confirm deterministic state roots after batch commits.

## 6. Consensus DAG Finality, Transaction Propagation & Overlapping Rounds
- **Proposed issue title:** `codex: implement consensus dag finality engine`
- **Branch name:** `codex/consensus-dag`
- **Labels:** `codex`, `p0`, `initiative/blockdag-scale`
- **Summary:** Add `crates/consensus_dag/` with quorum-driven finality over the DAG, integrate median HashTimer ordering, incorporate transaction-level DAG propagation hooks, and enable overlapping rounds without the current 100 ms clamp.
- **Key paths:**
  - `crates/consensus_dag/`
  - `crates/consensus/src/engine.rs`
- **Acceptance checks:** Simulated network test showing deterministic order across validators despite concurrent proposals and ensuring transaction DAG gossip converges identically across nodes.

## 7. Benchmarks & Deterministic Replay Validation
- **Proposed issue title:** `codex: add multi-million tps benchmarks and replay suite`
- **Branch name:** `codex/benchmarks-replay`
- **Labels:** `codex`, `p1`, `initiative/blockdag-scale`
- **Summary:** Provide synthetic transaction generators, Prometheus metrics, and replay tests that validate deterministic execution. Include guidance for tuning parameters toward the 3M TPS goal.
- **Key paths:**
  - `crates/benchmarks/`
  - `crates/node/src/telemetry.rs`
  - `docs/benchmarks/`
- **Acceptance checks:** Automated benchmark harness hitting ≥3M TPS in simulation, replay tests gated behind `--features replay`.

---

### Execution Notes
- Issues 1, 2, and 6 should be prioritized first because they unblock the DAG parallelism and transaction-level propagation hooks. Issues 3–5 can proceed in parallel once the async runtime lands. Issue 7 should start once DAG consensus is merged to set baseline metrics.
- Issue 4 should also lay groundwork for an eventual DAG-native transaction pool (Option B in the mempool discussion) so that consensus Issue 6 can adopt vertex-level propagation without another structural rewrite.
- Each issue should link back to the BlockDAG scale PRD and include cross-team coordination for networking and storage reviewers.

