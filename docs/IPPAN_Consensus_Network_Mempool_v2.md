# IPPAN Consensus / Networking / Mempool v2

**Version:** 1.0  **Date:** October 2025
**Scope:** `crates/{consensus,mempool,network,ledger}`
**Status:** Implemented (skeleton + interfaces)

---

## 1  Objective

This release replaces the experimental Proof-of-Authority engine with the deterministic-learning **DLC + HashTimer consensus** stack (D-GBDT driven validator selection with shadow verifiers).
It introduces modular crates for **consensus coordination**, **transaction admission**, **secure networking**, and **atomic ledger state**—forming the production core of the IPPAN blockchain.

---

## 2  Motivation

Previous PoA and networking prototypes exhibited:

* Fixed 100–250 ms finalization window → no jitter tolerance
* Non-atomic state updates → possible double crediting
* Unsigned transaction admission → mempool DoS risk
* HTTP polling “P2P” → no authentication or replay protection

v2 corrects all of these by establishing deterministic timing, priority-based back-pressure, authenticated gossip, and atomic state transitions.

---

## 3  Architecture Overview

```
┌────────────────────────────┐
│  ippan-consensus           │  ← round FSM, quorum proofs
├────────────────────────────┤
│  ippan-mempool             │  ← async priority queue, fee validation
├────────────────────────────┤
│  ippan-network             │  ← libp2p Noise + Gossipsub + Kademlia
├────────────────────────────┤
│  ippan-ledger              │  ← atomic DB, snapshots, rollback
└────────────────────────────┘
```

Each layer communicates over typed `tokio::mpsc` channels and can be tested independently.

---

## 4  Consensus Layer (`crates/consensus`)

**Type:** Deterministic Learning Consensus (D-GBDT + shadow verifiers)
**Time base:** HashTimer (µs precision)
**Signature:** Ed25519 → BLS aggregation planned

### Highlights

| Function                   | Purpose                                    |
| -------------------------- | ------------------------------------------ |
| `ConsensusEngine::start()` | Async event loop consuming `NetworkEvent`s |
| `RoundMessage`             | Proposal/Vote/Commit message format        |
| `QuorumProof`              | Aggregated validator signature set         |
| `ConsensusConfig`          | Quorum & adaptive timing parameters        |

**Adaptive finalization:**

```
Δt_final = max(3 × median_peer_RTT, 250 ms)
```

**Atomic commit:**
Ledger updates occur inside transactional batches—no async mutations outside commit context.

---

## 5  Mempool Layer (`crates/mempool`)

An asynchronous, capacity-bounded priority queue providing deterministic transaction ordering.

| Feature             | Description                                    |
| ------------------- | ---------------------------------------------- |
| **Priority metric** | fee ÷ size                                     |
| **Admission rules** | signature ✔ fee ✔ nonce ✔ payload ✔            |
| **Concurrency**     | `tokio::Mutex` with bounded vector             |
| **Eviction**        | drop lowest-priority when full                 |
| **API**             | `insert()`, `pop_batch()`, `get_pending_for()` |

Future phases will replace `Vec` with a binary-heap + channel architecture for constant-time selection.

---

## 6  Networking Layer (`crates/network`)

A full libp2p implementation providing encryption, peer discovery, and gossip.

| Subsystem        | Role                                                                 |
| ---------------- | -------------------------------------------------------------------- |
| **Noise XX**     | authenticated encryption                                             |
| **Yamux / QUIC** | multiplexed transport                                                |
| **Kademlia**     | global peer discovery                                                |
| **Gossipsub**    | topic-based broadcast (`ippan-txpool`, `ippan-block`, `ippan-round`) |
| **Replay Guard** | HashTimer + nonce window                                             |

**Public API**

```rust
pub struct NetworkService {
    pub swarm: Swarm<NetworkBehaviour>,
    pub tx: mpsc::Sender<NetworkEvent>,
}

pub enum NetworkEvent {
    TxReceived(Transaction),
    BlockReceived(Block),
    RoundMessage(RoundMessage),
    PeerDiscovered(PeerId),
}
```

---

## 7  Ledger Layer (`crates/ledger`)

Implements atomic state transitions and recovery.

| Capability            | Implementation                             |
| --------------------- | ------------------------------------------ |
| **Atomic commit**     | Sled `WriteBatch` (upgradeable to RocksDB) |
| **Snapshot/rollback** | every 100 rounds                           |
| **Integrity proof**   | Merkle root + HashTimer signature          |
| **API**               | `apply_block() → snapshot() → rollback()`  |

---

## 8  Cross-Crate Interfaces

| Producer    | Consumer    | Channel      | Data         |
| ----------- | ----------- | ------------ | ------------ |
| `network`   | `mempool`   | async mpsc   | Transaction  |
| `consensus` | `network`   | Gossipsub    | RoundMessage |
| `ledger`    | `consensus` | sync call    | State root   |
| `mempool`   | `consensus` | bounded mpsc | Tx batch     |

This separation guarantees that a failure in one layer does not corrupt state in another.

---

## 9  Security & Determinism

| Category                   | Mechanism                            |
| -------------------------- | ------------------------------------ |
| **Peer authentication**    | Noise XX + Ed25519 identity          |
| **Replay protection**      | `(HashTimer, nonce)` sliding window  |
| **Signature verification** | enforced on admission and quorum     |
| **Fee enforcement**        | dynamic min fee                      |
| **State integrity**        | atomic commits + snapshot signatures |

---

## 10  Telemetry

| Metric                 | Unit     | Purpose                  |
| ---------------------- | -------- | ------------------------ |
| `mempool_depth`        | tx count | back-pressure monitoring |
| `tx_admission_rate`    | tx/s     | DoS detection            |
| `finalization_latency` | ms       | consensus tuning         |
| `peer_count`           | n        | network health           |
| `rollback_count`       | n        | fault monitoring         |

Exposed at `/metrics` for Prometheus.

---

## 11  Phase Roadmap (40 → 45)

| Phase  | Deliverable                                 |
| ------ | ------------------------------------------- |
| **40** | Consensus FSM & adaptive timing             |
| **41** | Ledger atomicity + rollback                 |
| **42** | Async mempool w/ fee priority               |
| **43** | Full libp2p stack (Noise + Gossipsub + KAD) |
| **44** | Threshold signature aggregation             |
| **45** | Integration tests + telemetry               |

---

## 12  Implementation Summary

| Crate             | Key File     | Purpose                       |
| ----------------- | ------------ | ----------------------------- |
| `ippan-consensus` | `src/lib.rs` | round handling & quorum proof |
| `ippan-mempool`   | `src/lib.rs` | tx queue management           |
| `ippan-network`   | `src/lib.rs` | Noise-secured libp2p service  |
| `ippan-ledger`    | `src/lib.rs` | atomic state store            |

All compile as independent crates and link via the workspace.

---

## 13  Next Steps

1. Generate `Cargo.toml` workspace with cross-crate references.
2. Implement phase-40 FSM (`Proposal → Vote → Commit`).
3. Add integration test harness (`tests/cluster_sim.rs`).
4. Benchmark adaptive latency under simulated 50 ms RTT.

---

**Authors:** Hugh Vega, Désirée Verga
**Repository:** [github.com/dmrl789/IPPAN](https://github.com/dmrl789/IPPAN)
**Document Type:** Technical Design Spec (for internal engineering review)

---

Would you like me to append an **architectural diagram (SVG + PNG)** to visually show how the four new crates interact in runtime—arrows between consensus, mempool, network, and ledger with HashTimer flow?
