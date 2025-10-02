# IPPAN Data Storage, Pruning, and Fast Sync

## 1. Motivation
IPPAN targets sustained block times between **10–50 ms**, throughput of **1–10 million TPS**, and long-term auditability. At these rates, raw transaction data can exceed **100+ TB per day** if every node stores everything forever. This document specifies IPPAN's data availability and storage model so that nodes can operate sustainably while preserving verifiability.

---

## 2. Block Data Layout
Each block `B` is composed of the following fields:

| Field | Description |
| --- | --- |
| `prev_hashes[]` | HashTimer hashes of parent tips (BlockDAG references). |
| `hash_timer` | Canonical IPPAN Time timestamp and entropy seed. |
| `tx_root` | Merkle/Verkle root of every transaction batched into `B`. |
| `erasure_root` | Merkle root of the erasure-coded shards stored in the IPNDHT. |
| `receipt_root` | Root of receipts proving execution and state changes. |
| `state_root` | Verkle root of global state after executing `B`. |
| `validator_sigs[]` | Aggregate signature of the block proposer plus round validators. |

The **header** consists of the fields above excluding the raw transaction list and receipts. The **body** contains raw transactions, full receipts, and any intermediate state diffs necessary for execution proofs.

---

## 3. Data Availability & Erasure Coding

1. **Body encoding.** Block bodies are erasure-coded before announcement. The default configuration uses **Reed–Solomon (n = 16, k = 10)**, yielding 10 data shards and 6 parity shards. Implementations may support alternate parameters for specialized deployments (e.g., 20/14 for long-haul archival networks).
2. **Distribution.** Shards are pushed to the IPNDHT together with metadata:
   ```json
   {
     "block_hash": "...",
     "shard_index": 0,
     "size": 1048576,
     "checksum": "blake3(...)",
     "provider_peers": ["peerId1", "peerId2", "..."]
   }
   ```
3. **Sampling.** Validators perform random shard sampling each round. A block with insufficient availability (e.g., < `k` shards retrievable within the sampling window) is rejected. Nodes may weigh sampling based on peer reputation and shard freshness.
4. **Repair.** Nodes that fail to retrieve a shard can request reconstruction from peers holding disjoint shard subsets. Repair proofs are gossiped to prevent slashable offenses against withholding validators.

---

## 4. Retention Model

| Node Role | Headers | Bodies | Receipts | Snapshots |
| --- | --- | --- | --- | --- |
| **Validator** | Forever | Hot window: 24–72 h | ≥ 90 d | ≥ 90 d |
| **Full Node** | Forever | Configurable (interest-based pinning) | ≥ 90 d | ≥ 90 d |
| **Archival Node** | Forever | ≥ 1 y or indefinite | ≥ 1 y | ≥ 1 y |

- **Hot window:** Ensures late peers can sync and disputes can be raised. Validators must guarantee that the hot window overlaps with the unfinalized fork-choice horizon.
- **Receipts & snapshots:** Enable proofs without raw transaction data. Receipts encode event logs, execution traces, and state transitions. Snapshots provide succinct state commitments for fast sync and light-client proofs.
- **Archival nodes:** Voluntary but incentivized via storage proofs and retrieval markets. They serve historical data upon request and seed the IPNDHT with cold shards.

### Runtime configuration

The reference node exposes retention tuning through environment variables (or matching config entries):

| Variable | Description | Default |
| --- | --- | --- |
| `IPPAN_RETENTION_HOT_BLOCK_HEIGHTS` | Minimum number of recent block bodies to keep regardless of age. | `7200` |
| `IPPAN_RETENTION_HOT_HOURS` | Time-based hot window for block bodies. | `72` |
| `IPPAN_RETENTION_RECEIPT_DAYS` | Minimum receipt retention horizon. | `90` |
| `IPPAN_RETENTION_SNAPSHOT_DAYS` | Minimum snapshot retention horizon. | `90` |
| `IPPAN_RETENTION_PRUNE_INTERVAL_SECS` | Background pruning cadence. | `300` |

These values are consumed by the storage layer's retention policies and enforced by a background task. Setting a field to `0` disables the corresponding policy dimension (e.g., `RETENTION_HOT_HOURS=0` disables time-based pruning for block bodies).

---

## 5. Fast Sync Procedure

IPPAN supports a fast-sync mechanism that allows new or recovering nodes to come online without replaying the entire transaction history.

1. **Bootstrap snapshot.** Fetch the latest signed snapshot checkpoint:
   ```json
   {
     "state_root": "...",
     "block_height": 12345678,
     "round_id": "0x...",
     "validator_set": [ ... ],
     "signature": "agg_sig"
   }
   ```
   The signature covers the entire payload and is verified against the validator set embedded in the checkpoint. Implementations should support multiple sources (trusted peers, public gateways, or on-chain beacon contracts).
2. **Header sync.** Sync block headers from the checkpoint height to the network tip. Headers are verified against validator aggregates and `prev_hashes[]` linkage. Any fork choice rules (e.g., LMD-GHOST on the HashTimer DAG) are applied during this phase.
3. **Proof acquisition.** For each header, retrieve the corresponding receipt and state proofs required to update the local state commitment. Nodes request erasure-coded shards only if receipt proofs are missing or fail validation.
4. **State reconstruction.** Starting from the checkpoint state, apply the receipts/state diffs sequentially to rebuild the state. Merkle/Verkle proofs ensure that each transition matches the advertised `state_root`.
5. **Finalization.** Once the reconstructed state reaches the latest finalized block, switch to normal operation (full validation or light verification depending on node role). Nodes may optionally backfill historical bodies to extend their local retention window.

The storage crate now persists block receipts alongside signed state snapshots. Snapshots can be listed or retrieved through the storage API and are automatically pruned according to the configured retention policies.

---

## 6. Pruning Strategy

- **Policy-driven pruning.** Nodes prune body shards older than their retention window while preserving headers, receipts, and snapshots. Policies can be time-based (e.g., retain 72 hours) or height-based (retain last `N` finalized rounds).
- **Graceful degradation.** Before pruning, nodes publish an intent message so peers relying on their shards can re-pin data. This is integrated with the IPNDHT provider list.
- **Audit trail.** Even after pruning, auditors can verify historical state transitions via receipts and snapshots. Archival nodes and third-party storage markets maintain cold copies of pruned shards.

---

## 7. Open Questions & Future Work

- Adaptive erasure coding parameters that respond to observed churn and network latency.
- Lightweight availability proofs for light clients that cannot perform full shard sampling.
- Incentive mechanisms that balance storage costs across validators, full nodes, and archival participants.
- Snapshots that include execution witnesses for zero-knowledge based fast sync.

