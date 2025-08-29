# IPPAN — Minimal PRD (Throughput MVP)

## 1) Goal
Prove IPPAN can process **up to 10,000,000 tx/s** (10M TPS) by implementing only the bare-minimum pipeline: **IPPAN Time → HashTimers → Mempool → Blocks → Rounds → Finality**, with **wallets** and **P2P** sufficient to feed and propagate load.

---

## 2) In-Scope (must have)

1) **IPPAN Time (Network Median Time)**
- Nodes compute median of peers’ clocks each round.
- Export `ippan_time_us()` (microseconds).
- Drift guard: ignore peers deviating >2 ms from local median.

2) **HashTimers**
- Each tx and block carries `HashTimer = H( prefix(IPPAN_time_us) ∥ entropy ∥ tx_id )`.
- Sort keys: `(HashTimer, tx_id)` for deterministic ordering.
- Validate monotonicity per account (nonce).

3) **Transactions (single type: Payment)**
- Fields (bytes): `ver(1) | from_pub(32) | to_addr(32) | amount_u64(8) | nonce_u64(8) | ippan_time_us(8) | hashtimer(32) | sig(64)`.
- Max size: **185 B**.
- No fees/complexities beyond nonce + signature verification.

4) **Wallets (Ed25519 + base58i)**
- Generate/restore keys, derive `i…` address.
- Sign payment tx.
- CLI only: `wallet new`, `wallet send`, `wallet balance` (balance via local state).

5) **P2P (Gossip + Discovery)**
- libp2p with **mDNS + Kademlia**.
- Topics: `tx-gossip`, `block-gossip`, `round-gossip`.
- Propagation: floodsub or gossipsub (1.1) with message dedupe (tx_id/block_id bloom).

6) **Mempool (lock-free, throughput-first)**
- Bounded by memory (e.g., 8–16 GB).
- Structures:
  - per-shard priority queues ordered by `(HashTimer, nonce)`.
  - account map: latest committed nonce to drop old/dup tx.
- Admission: sig verify → nonce check → enqueue.

7) **Block Builder (micro-blocks)**
- Target block size: **16 KB** (4–32 KB allowed).
- Build every **10–50 ms** or when byte/tx budget hit.
- Content: **tx references only** (hashes), plus Merkle root.
- Header: `parent(s) | round_id | block_time_us | builder_id | tx_count | merkle_root | hashtimer`.

8) **Rounds & Finality (Roundchain)**
- Round cadence: **100–250 ms**.
- Each round selects **verifier set** (VRF on previous round hash).
- Finality rule (MVP): **2f+1 signatures** on the round’s block set → finalize.
- No zk/STARK in MVP (stub only).

9) **State Apply**
- On finalize: apply blocks in deterministic HashTimer order.
- State: key–value balances + nonces (in-memory with periodic snapshot to disk).

10) **Observability**
- Per-stage counters: ingress tx/s, verified tx/s, mempool size, block tx, round tx, finalized tx/s.
- Latencies (p50/p95/p99): admit→finalize.
- Export Prometheus metrics + CSV.

---

## 3) Out-of-Scope (for this MVP)
- Domains/TLDs, storage/DHT files, staking/slashing, PQ crypto, ZK proofs, smart contracts, fees/complex tokenomics, bridges, UI apps (beyond CLI).

---

## 4) Architecture (fast path)

`P2P Ingress → Sig Verify → Nonce Check → Mempool (per-shard PQ) → Block Build (10–50 ms) → P2P Block Gossip → Round Aggregate (100–250 ms) → Quorum Sign → Finalize → Apply State → Metrics`

Sharding model (compute only, optional to reach 10M):
- **S shards** (CPU-bound), each with:
  - Verify workers (thread-pool, SIMD).
  - PQ mempool.
  - Block builder.
- Cross-shard ordering via HashTimer; tie-break by shard id then tx_id.

---

## 5) Data Limits & Defaults

- Tx size: ≤ 185 B (payment).
- Block size: 16 KB target (max 32 KB).
- Block interval: 10–50 ms.
- Round interval: 100–250 ms.
- Max parent refs per block: 4 (DAG width control).
- Gossip fanout: 6–12.
- Duplicate cache: 60 s bloom.

---

## 6) APIs / CLI (MVP)

**Node**
- `GET /metrics` → Prometheus text.
- `GET /health` → {status, peers, mempool_size}
- `POST /tx` → submit signed tx (binary or hex).

**Wallet CLI**
- `wallet new|recover`
- `wallet addr`
- `wallet send --to i… --amount <u64> --nonce auto --node http://127.0.0.1:8080`

**Loadgen CLI**
- `loadgen --tps <N> --accounts <M> --duration <s> --nodes <list> --shards <S>`

---

## 7) Environment for Benchmark

- **Cluster:** 50–200 nodes, 10–40 Gbit/s fabric (or cloud eqv), NUMA-aware.
- **Per node:** 32–96 vCPU, 64–256 GB RAM, NVMe.
- **Crypto:** Ed25519 via optimized library (ed25519-dalek w/ batch verify + AVX2).
- **Runtime:** Rust stable, `-C target-cpu=native`, LTO thin, jemalloc.

---

## 8) Success Criteria (10M TPS)

1) **Throughput**
- **Sustain ≥10,000,000 tx/s** for **≥300 seconds** cluster-wide (finalized).
- Per-node NIC not saturated (headroom ≥10%).

2) **Latency**
- Admit→Finalize: **p50 ≤ 350 ms**, **p95 ≤ 600 ms**, **p99 ≤ 900 ms**.

3) **Consistency**
- Zero reorgs after finality.
- Deterministic state root matches across all validators each round.

4) **Resource**
- CPU ≤ 85% avg; RAM ≤ 80% of budget; GC pauses negligible.

5) **Loss**
- Dropped/late tx ≤ 0.01%.

---

## 9) Test Plan

**A. Functional Smoke**
- Single node: submit 10k tx → finalize → balances correct.

**B. Scale-Up**
- 1→5→10→20 shards on 1 node; find per-node ceiling.

**C. Scale-Out**
- 10→50→100→200 nodes; uniform load; measure throughput/latency.

**D. Adversarial**
- Burst 2× average TPS for 10 s every minute → no stalls.
- 5% byzantine nodes (drop/duplicate blocks) → finality holds.

**E. Faults**
- Kill 10% nodes mid-run → quorum recovers within 2 rounds.

Deliverables: CSV metrics, Grafana dashboards, runbook, flamegraphs (verify, mempool, build, finalize).

---

## 10) Risks & Mitigations

- **Sig verify bottleneck** → batch verification, CPU pinning, SIMD.
- **Mempool contention** → per-shard queues, lock-free ring buffers.
- **Gossip storms** → gossipsub mesh params tuning, dedupe bloom, TXID cut-through.
- **Clock skew** → median-time + outlier rejection; HashTimer monotonic per account.
- **Hot accounts** → account-local queues, nonce prefetch.

---

## 11) Definition of Done (MVP)

- Code paths implemented for items in §2.
- Bench harness reaches ≥10M finalized TPS for ≥300 s with SLAs in §8.
- Reproducible scripts (Terraform/Ansible or k8s) + one-page runbook.
- Post-mortem of bottlenecks and next optimizations.
