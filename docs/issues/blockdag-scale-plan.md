---
codex:
  action: implement
  priority: high
  type: development
  phase: parallel-dag-scaling
  output:
    - crates/consensus/parallel_dag.rs
    - crates/network/parallel_gossip.rs
    - benchmarks/throughput_report.md
---

# BlockDAG Scale Initiative — Issue Breakdown

## Initiative Summary

The BlockDAG Scale program is the engineering track that moves the IPPAN validator from the present single-threaded Proof of Authority (PoA) design to the PRD-mandated 3–10 million TPS envelope. The work decomposes into focused issue cards aimed at concurrency, data-structure, networking, storage, and validation milestones. Each issue below contains a recommended title, branch scaffold, key labels, scope expectations, and measurable acceptance checks. All issues should apply the umbrella label `initiative/blockdag-scale` and cross-reference the BlockDAG Scale PRD.

### Objectives & Success Metrics

| Objective | Target Metric | Validation Signal |
| --- | --- | --- |
| Raise end-to-end throughput | Sustain ≥3M TPS synthetic load with deterministic replay | Benchmarks in Issue 7 executed on 3-node cluster |
| Reduce consensus latency | Finality decision <500 ms under 3M TPS | Consensus simulation harness (Issue 6) |
| Modernize networking stack | QUIC gossip round-trip <150 ms between regions | Integration test suite (Issue 3) |
| Harden storage and replay | Deterministic state root after 1M block replay | Storage replay harness (Issue 5) |

### Execution Cadence & Dependencies

1. **Foundational runtime & DAG work (Issues 1, 2, 6)** — mandatory before parallelization unlocks.
2. **Networking + mempool + storage upgrades (Issues 3, 4, 5)** — can proceed once async runtime is merged; coordinate interfaces through weekly syncs with networking and storage owners.
3. **Benchmarking and validation (Issue 7)** — kicks in once the consensus DAG is feature-complete to capture baseline numbers and regression alerts.

### Cross-Team Coordination

* **Consensus + Runtime:** Codex maintains primary ownership; reviewers from core consensus group required for Issues 1, 2, 6.
* **Networking:** Work closely with gateway team when introducing libp2p to ensure compatibility with existing telemetry.
* **Storage:** Coordinate RocksDB/Redb decision with storage architects; ensure CI images include necessary native libs.
* **DevOps:** InfraBot to provision short-lived benchmark clusters (Issue 7) and monitor metrics roll-out.

### Risk Register

| Risk | Mitigation |
| --- | --- |
| Async conversion deadlocks legacy blocking calls | Add temporary `spawn_blocking` wrappers and instrumentation (Issue 1) |
| DAG scheduler introduces ordering regressions | Snapshot-based deterministic replay harness (Issues 2 & 7) |
| QUIC transport increases resource usage | Bench dedicated QUIC connection manager, enable adaptive peer scoring (Issue 3) |
| Lock-free mempool causes starvation | Apply bounded crossbeam queues with fairness heuristics (Issue 4) |
| Storage WAL change delays bootstrapping | Provide migration tooling & fallback path (Issue 5) |

---

## Issue Cards

Each issue description enumerates the required changes, dependencies, tests, and follow-up items. Branch names are suggestions; Codex may adapt them to fit repo policy provided the intent remains clear.

---

## 1. Convert Node Runtime to Async Tokio Core
- **Proposed issue title:** `codex: migrate node runtime to async tokio core`
- **Branch name:** `codex/async-runtime`
- **Labels:** `codex`, `p0`, `initiative/blockdag-scale`
- **Primary scope:**
  - Replace the blocking main loop with a single Tokio runtime that supervises consensus, networking, mempool, and storage tasks.
  - Introduce structured concurrency via `tokio::task::JoinSet` for task orchestration and graceful shutdown.
  - Audit all blocking calls (sled, filesystem, crypto) and gate them behind `spawn_blocking` shims until their async counterparts are added in later milestones.
  - Plumb async-aware configuration through `crates/node/src/config.rs` to remove hard-coded thread counts.
- **Key paths & owners:**
  - `crates/node/src/main.rs` — runtime bootstrapping (Runtime WG).
  - `crates/consensus/` — adapt proposal loop to async streams (Consensus WG).
  - `crates/network/` — convert peer handlers to async traits (Networking WG).
  - `crates/storage/` — wrap storage access for non-blocking execution (Storage WG).
- **Dependencies:** None; foundational for Issues 2–5.
- **Deliverables:** New async runtime module, migration notes in `docs/architecture/runtime-async.md` (short overview), instrumentation for task lifecycle logging.
- **Acceptance checks:**
  - Node boots under Tokio runtime and processes blocks end-to-end in devnet mode.
  - `cargo test` passes; add targeted async smoke test covering start/stop cycles.
  - Metrics endpoint exports task queue depth and executor saturation gauges.
- **Follow-up hooks:** Provide compatibility layer for legacy CLI commands; coordinate with DevOps to update systemd service templates.

## 2. Introduce Concurrent DAG Data Structure
- **Proposed issue title:** `codex: add dag crate and parallel block scheduler`
- **Branch name:** `codex/dag-scheduler`
- **Labels:** `codex`, `p0`, `initiative/blockdag-scale`
- **Primary scope:**
  - Stand up a new `crates/dag/` crate housing adjacency lists, vertex metadata, and HashTimer ordering primitives with lock-free read paths.
  - Implement concurrent insertion logic with optimistic verification to prevent cycles and enforce deterministic timestamp ordering.
  - Replace the single queue block dispatcher with a DAG scheduler that surfaces ready vertices to the consensus engine.
  - Provide snapshot/export utilities for replay harnesses and telemetry.
- **Key paths & owners:**
  - `crates/dag/` — new crate ownership shared by Consensus + Runtime WGs.
  - `crates/consensus/src/engine.rs` — integrate scheduler hooks.
  - `crates/node/src/telemetry.rs` — expose DAG depth/width metrics.
- **Dependencies:** Requires async runtime foundation from Issue 1 for concurrent tasks.
- **Deliverables:** DAG crate with documentation (`README.md`), scheduler integration notes, instrumentation for DAG growth.
- **Acceptance checks:**
  - Unit tests covering insertion, cycle prevention, deterministic ordering snapshots, and concurrent writer stress test.
  - Feature flag enabling DAG scheduler behind `--features parallel-dag` for staged rollout.
  - Bench harness producing DAG metrics for >10k concurrent vertices without contention spikes.
- **Follow-up hooks:** Align DAG data model with transaction propagation schema planned in Issue 6.

## 3. Overhaul Networking with libp2p QUIC Gossip
- **Proposed issue title:** `codex: implement libp2p gossip network`
- **Branch name:** `codex/p2p-gossip`
- **Labels:** `codex`, `p1`, `initiative/blockdag-scale`
- **Primary scope:**
  - Replace HTTP polling peer layer with libp2p QUIC transport configured for secured multiplexed streams.
  - Define dedicated gossipsub topics for transactions, blocks, and DAG vertex metadata; adopt `bincode` serialization for wire efficiency.
  - Implement peer scoring, heartbeat, and bandwidth throttling to mitigate spam and maintain determinism.
  - Extend node configuration to surface peer identity, listen addresses, and discovery parameters.
- **Key paths & owners:**
  - `crates/p2p/` — new libp2p stack (Networking WG).
  - `crates/network/` — adapt network abstractions to libp2p (Networking WG).
  - `crates/node/src/config.rs` — configuration wiring (Runtime WG).
  - `deploy/` manifests — ensure ports/open firewall docs updated (Infra WG).
- **Dependencies:** Async runtime (Issue 1) must land; DAG scheduler integration (Issue 2) informs block gossip payloads.
- **Deliverables:** QUIC-enabled networking crate, documentation on peer configuration, metrics for gossip lag and peer health.
- **Acceptance checks:**
  - Integration test spinning up three nodes demonstrating block propagation within target latency.
  - Fuzz test for malformed gossip messages with graceful degradation.
  - Bench numbers comparing throughput/latency vs. legacy HTTP layer.
- **Follow-up hooks:** Coordinate with gateway to expose libp2p metrics via Prometheus exporters; align TLS/identity management with DevOps.

## 4. Lock-Free Global Mempool and Parallel Block Builder
- **Proposed issue title:** `codex: implement lock-free global mempool`
- **Branch name:** `codex/global-mempool`
- **Labels:** `codex`, `p1`, `initiative/blockdag-scale`
- **Primary scope:**
  - Design a lock-free global queue using crossbeam or loom-validated data structures supporting multi-producer/multi-consumer throughput.
  - Preserve HashTimer ordering via timestamped envelopes and fairness heuristics; ensure compatibility with DAG vertex batching.
  - Refactor block builder to assemble blocks in parallel batches, coordinating with DAG scheduler readiness signals.
  - Introduce instrumentation for queue depth, enqueue latency, and dropped transaction counters.
- **Key paths & owners:**
  - `crates/mempool/` — mempool redesign (Mempool WG).
  - `crates/consensus/src/block_builder.rs` — parallel builder integration (Consensus WG).
  - `crates/node/src/telemetry.rs` — metrics integration (Runtime WG).
- **Dependencies:** Async runtime (Issue 1) and DAG scheduler (Issue 2) must expose async interfaces; libp2p gossip (Issue 3) feeds transactions.
- **Deliverables:** New mempool module, benchmarking scripts in `crates/benchmarks/`, documentation for tuning mempool parameters.
- **Acceptance checks:**
  - Benchmarks demonstrate ≥500k tx/sec ingestion on a single node with deterministic ordering and <5% variance across runs.
  - Concurrency stress test with synthetic 3M TPS load shows no panics or starvation; queue saturation alerts fire via telemetry.
  - Compatibility test ensures mempool upgrade remains backward compatible with legacy transaction format.
- **Follow-up hooks:** Evaluate DAG-native transaction pool prototype for future iteration; coordinate with Issue 6 for vertex-level propagation.

## 5. Storage v2: Batched WAL and Parallel Execution
- **Proposed issue title:** `codex: introduce storage_v2 wal with parallel state apply`
- **Branch name:** `codex/storage-v2`
- **Labels:** `codex`, `p1`, `initiative/blockdag-scale`
- **Primary scope:**
  - Stand up `crates/storage_v2/` with a batched WAL implemented atop RocksDB or Redb, including column family layout and schema migrations.
  - Introduce binary serialization using `bincode`/`serde` for state transitions and transaction receipts.
  - Add parallel state apply pipelines using Rayon with shard-aware partitioning and conflict detection.
  - Generate epoch snapshots every 1000 blocks and provide tooling for snapshot import/export.
- **Key paths & owners:**
  - `crates/storage_v2/` — new storage engine (Storage WG).
  - `crates/node/src/state/` — integration layer (Runtime WG).
  - `deploy/` scripts — ensure new storage requirements documented (Infra WG).
- **Dependencies:** Async runtime (Issue 1); mempool redesign (Issue 4) ensures batched writes align with builder throughput.
- **Deliverables:** Storage v2 crate with migration guide, snapshot tooling CLI, Prometheus metrics for WAL flush latency and compaction.
- **Acceptance checks:**
  - Replay tests confirm deterministic state roots after batch commits across multiple nodes.
  - Benchmark demonstrates ≥5x throughput improvement over legacy sled-based storage under 3M TPS synthetic load.
  - Snapshot import/export validated on fresh node bootstrapping scenario.
- **Follow-up hooks:** Plan for storage pruning and archiving strategy in subsequent milestone; coordinate with ReleaseBot for migration docs.

## 6. Consensus DAG Finality, Transaction Propagation & Overlapping Rounds
- **Proposed issue title:** `codex: implement consensus dag finality engine`
- **Branch name:** `codex/consensus-dag`
- **Labels:** `codex`, `p0`, `initiative/blockdag-scale`
- **Primary scope:**
  - Create `crates/consensus_dag/` implementing quorum-driven finality atop the DAG with HashTimer-based leader rotation.
  - Integrate transaction-level DAG propagation hooks to guarantee vertex availability ahead of finality decisions.
  - Enable overlapping consensus rounds by removing the 100 ms clamp and introducing adaptive backpressure tied to DAG depth.
  - Surface finality proofs and checkpoints for downstream consumers (storage, benchmarking harness).
- **Key paths & owners:**
  - `crates/consensus_dag/` — new consensus engine (Consensus WG).
  - `crates/consensus/src/engine.rs` — integration with existing consensus facade (Consensus WG).
  - `crates/network/` — ensure gossip compatibility for finality messages (Networking WG).
- **Dependencies:** Requires Issues 1 and 2 to be merged; benefits from preliminary work in Issues 3 and 4.
- **Deliverables:** Consensus DAG crate, documentation on finality rules, new telemetry for round overlap and quorum progress.
- **Acceptance checks:**
  - Simulated network test shows deterministic ordering across validators despite concurrent proposals.
  - Chaos test injecting 5% message loss still achieves finality under targeted latency.
  - Audit log of finality proofs persisted for replay harness consumption.
- **Follow-up hooks:** Coordinate with Benchmarking (Issue 7) for metrics; update protocol spec to reflect DAG finality semantics.

## 7. Benchmarks & Deterministic Replay Validation
- **Proposed issue title:** `codex: add multi-million tps benchmarks and replay suite`
- **Branch name:** `codex/benchmarks-replay`
- **Labels:** `codex`, `p1`, `initiative/blockdag-scale`
- **Primary scope:**
  - Build synthetic transaction generators and workload profiles (steady, bursty, adversarial) for benchmarking DAG consensus.
  - Instrument Prometheus metrics capturing TPS, latency percentiles, executor saturation, and mempool pressure.
  - Implement deterministic replay harness toggled via `--features replay` to validate ordering and state convergence across nodes.
  - Author documentation capturing tuning parameters, recommended hardware, and interpretation guidelines.
- **Key paths & owners:**
  - `crates/benchmarks/` — workload generators and harness (Performance WG).
  - `crates/node/src/telemetry.rs` — metrics integration (Runtime WG).
  - `docs/benchmarks/` — documentation updates (Docs WG).
- **Dependencies:** Relies on Issues 2, 4, 5, and 6 to expose metrics and replay hooks; coordinate with InfraBot for cluster provisioning.
- **Deliverables:** Automated benchmark CI job (nightly), replay suite documentation, Grafana dashboards for BlockDAG metrics.
- **Acceptance checks:**
  - Automated harness reaches ≥3M TPS in simulation environment with reproducible metrics snapshot.
  - Replay tests validate deterministic execution after injecting synthetic faults (clock skew, message delay).
  - CI publishes benchmark artifacts (plots/logs) for regression tracking.
- **Follow-up hooks:** Feed metrics into release readiness reviews; align with ReleaseBot for changelog entries when thresholds improve.

---

### Execution Notes
- **Sequencing:** Issues 1 & 2 ship together in Sprint A (runtime + DAG skeleton). Issue 6 begins in Sprint B once scheduler API is stable. Issues 3–5 can be staggered across Sprints B–C with shared reviewers to avoid bottlenecks. Issue 7 should start in Sprint C to record the first benchmark baseline.
- **Resourcing:** Assign at least two Codex engineers per p0 card (Issues 1, 2, 6) with rotating reviewer coverage from consensus architects. Networking and storage guilds should dedicate one reviewer each throughout the initiative.
- **Documentation:** Every issue must link back to the BlockDAG Scale PRD and append a changelog snippet for ReleaseBot consolidation. Update relevant diagrams in `docs/diagrams/` when major interfaces (runtime, consensus) change.
- **Operational readiness:** InfraBot to draft rollout playbooks before merging Issues 3, 4, or 5 into main. Ensure DevOps captures new ports, env vars, or resource requirements.
- **Future-looking:** Issue 4 should lay groundwork for a DAG-native transaction pool (Option B from mempool workshop) so Issue 6 can adopt vertex-level propagation without another rewrite. Capture open questions in a follow-up ADR draft.

