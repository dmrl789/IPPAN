# IPPAN — Global Layer-1 Blockchain, Data Availability & Parallel Consensus

*(Updated October 2025 — unified specification and developer reference)*

---

## 1. IPPAN as a Global Layer-1 Blockchain

### 1.1 Mission
IPPAN is a **global, neutral, permissionless Layer-1 (L1) ledger** anchoring time, identity, and trust for any distributed system. L1 is deliberately minimal and deterministic — it stores only what is needed for **global ordering, auditability, and interoperability** — while application logic, large payloads, and high-volume data live in **Layer-2 (L2) systems**.

### 1.2 Design Principles
- **Deterministic Time & Ordering** — IPPAN Time (median network time with 100 ms precision) and HashTimer anchors yield a single, provable timeline.
- **Minimal Canonical State** — Only block/round headers, anchors, handle ownership, and L2 chain roots go on L1.
- **Decentralization & Survivability** — Network can restart from as few as two IPNWorker nodes; libp2p + DHT peer discovery; NAT hole punching and offline recovery.

### 1.3 L1 vs L2 Data Allocation

| Layer | Content | Typical Use |
|-------|---------|-------------|
| **L1** | Headers, HashTimers, handle registry, anchor roots for L2 chains/DHTs, ZK/STARK proofs | Timestamping, DNS/identity, compliance anchors, cross-chain interoperability |
| **L2** | Transaction bodies, app states, large content, confidential or regulated data | Payments, DeFi, healthcare, IoT, file storage |

- **Anchors:** ≤512 B per L2 commit  
- **Retrieval:** Clients verify on L1; fetch full data only when needed

### 1.4 Confidentiality & Compliance
- Personally identifiable or sensitive data **never resides in plaintext on L1**.  
- Confidential transactions: encrypted payloads, public commitments, optional ZK/STARK validity proofs.  
- Regulatory selective disclosure: keys can be revealed or STARK proofs supplied.

### 1.5 DNS & Human-Readable Identity
- **Global naming**: `@user.ipn` plus premium TLDs (`.cyborg`, `.iot`, `.m`).  
- Handle updates pay small IPN fees; fees redistributed to validators.  
- L2 namespaces can extend L1 registry.

### 1.6 Scalability & Performance
- **BlockDAG + Rounds** → 10–50 ms block frequency, 100–250 ms round finality, 1–10 M TPS.
- Ultra-light headers (4–32 KB blocks, 128 KB max).  
- Interoperability anchors: Ethereum/Bitcoin/other chains can commit to IPPAN.

### 1.7 Cost & Energy Model
- Lightweight participation; micro-fees (e.g. 10⁻⁸ IPN) for announcements and handle updates to deter spam but keep user cost negligible.

---

## 2. Parallel Block Creation with Global Round Finalization

### Goal
Allow all validators to propose blocks in parallel and finalize them deterministically with a single **global round**.

### Architecture

**Block**

| Field | Description |
|-------|-------------|
| `id` | 32-byte block hash |
| `creator` | Validator public key hash |
| `round` | Round identifier |
| `hashtimer` | Deterministic timestamp + entropy |
| `parent_ids` | List of previous round block IDs |
| `payload_ids` | Transaction batch refs |
| `merkle_payload` | Root of included transactions |
| `vrf_proof` | (optional) Validator eligibility proof |
| `signature` | Ed25519 signature |

**Round**
- Fixed time window (100–250 ms) via median IPPAN-Time.
- Fields: `RoundId`, `RoundWindow`, `RoundCertificate` (≥2f+1 sigs), `RoundFinalizationRecord` (deterministic tx/state result).

### Protocol Flow

1. **Parallel Proposal (Phase A)**  
   Each validator builds one block referencing all known parents from previous round and broadcasts header.

2. **Global Finalization (Phase B)**  
   Nodes collect ≥2f+1 blocks, aggregate signatures into `RoundCertificate`.  
   Deterministic ordering: topological sort by `(hashtimer → creator → block_id)`, then tx by `(hashtimer, tx_id)`.  
   Execute, compute `state_root`, publish `RoundFinalizationRecord`.

### Safety & Performance
- One block per validator per round; equivocation punished.
- Finality ≤250 ms per round.
- Compatible with HashTimer ordering and confidentiality.

### Networking
- Gossip topics: `block-headers:r`, `block-payloads`, `round-certificates:r`, `round-finalizations:r`.
- Anti-DoS: micro-fee for announcements, VRF sampling.

---

## 3. Data Availability & Storage Model

### 3.1 Block Layout

| Field | Description |
|-------|-------------|
| `prev_hashes[]` | Parent tips |
| `hash_timer` | Canonical timestamp + entropy |
| `tx_root` | Root of transactions (public or ciphertext) |
| `erasure_root` | Root of erasure-coded shards |
| `receipt_root` | Root of receipts |
| `state_root` | Verkle root of post-execution state |
| `validator_sigs[]` | Aggregated validator signatures |

- **Header** = metadata only (globally replicated)  
- **Body** = raw tx + receipts, optional state diffs (not universal)

### 3.2 Data Availability
- Reed–Solomon erasure coding (default n=16, k=10)  
- Shards announced to IPNDHT with provider metadata  
- Validators randomly sample shards; ≥95 % success required to finalize

### 3.3 Retention

| Node | Headers | Bodies | Receipts | Snapshots |
|------|---------|--------|----------|-----------|
| Validator | Forever | 24–72 h | ≥90 d | ≥90 d |
| Full | Forever | Configurable/pinned | ≥90 d | ≥90 d |
| Archival | Forever | ≥1 y | ≥1 y | ≥1 y |

---

## 4. Fast Sync

1. **Fetch signed snapshot**: `{state_root, block_height, round_id, validator_set, signature}`  
2. **Stream headers forward**  
3. **Optionally fetch bodies** if history needed  
4. **Run DA sampling** on recent blocks

---

## 5. Confidential Transactions & ZK-STARK Roadmap

- Encrypted payload envelope with recipient key list  
- DA sampling remains ciphertext-agnostic  
- Future STARK phases: per-tx validity → batched proofs → app-specific circuits

---

## 6. Networking / APIs

- DHT endpoints: `PUT /dht/shard`, `GET /dht/shard`, `HEAD /dht/block/availability`  
- Sync endpoints: `GET /sync/checkpoint/latest`, `GET /sync/headers`, etc.

---

## 7. Economics & Incentives

- Micro-fees for shard announcements  
- Shard serving rewards and archival contracts  
- Handle/domain fees fund validator pool

---

## 8. Implementation Notes

- Rust crates: `types`, `consensus/round.rs`, `storage`, `rpc`  
- Use Verkle or Merkle for roots; AES-GCM / XChaCha20 for envelopes; HPKE roadmap  
- CI: round simulation, DA sampling metrics, pruning tooling

---

## 9. Developer Reference Types & Examples

### 9.1 Transaction & Confidential Envelope

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionVisibility {
    Public,
    Confidential,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessKey {
    pub recipient_pub: String,
    pub enc_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfidentialEnvelope {
    pub enc_algo: String,
    pub iv: String,
    pub ciphertext: String,
    pub access_keys: Vec<AccessKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
pub enum TransactionBody {
    Public(Vec<u8>),
    Confidential(ConfidentialEnvelope),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub tx_id: String,
    pub visibility: TransactionVisibility,
    pub from_hint: Option<String>,
    pub topics: Vec<String>,
    pub body: TransactionBody,
    pub signature: Option<String>,
}
```

### 9.2 Receipt

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub tx_id: String,
    pub status: u16,
    pub resource_used: u64,
    pub touched_keys_root: String,
    pub proof_blob: Option<Vec<u8>>,
}
```

### 9.3 Block

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub prev_hashes: Vec<String>,
    pub hash_timer: String,
    pub tx_root: String,
    pub erasure_root: String,
    pub receipt_root: String,
    pub state_root: String,
    pub validator_sigs: Vec<String>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockBody {
    pub transactions: Vec<Transaction>,
    pub receipts: Vec<Receipt>,
    pub state_diffs: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub body: Option<BlockBody>,
}
```

### 9.4 Snapshot Checkpoint

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotCheckpoint {
    pub state_root: String,
    pub block_height: u64,
    pub round_id: String,
    pub validator_set: Vec<String>,
    pub signature: String,
}
```

### 9.5 Confidential Transaction Example (JSON)

```json
{
  "tx_id": "0xabc123",
  "visibility": "confidential",
  "from_hint": "@alice.ipn",
  "topics": ["payment", "escrow"],
  "body": {
    "kind": "confidential",
    "data": {
      "enc_algo": "AES-256-GCM",
      "iv": "base64iv",
      "ciphertext": "base64cipher",
      "access_keys": [
        {"recipient_pub": "ed25519:abc...", "enc_key": "base64key1"},
        {"recipient_pub": "ed25519:def...", "enc_key": "base64key2"}
      ]
    }
  },
  "signature": "base64sig"
}
```

---

## Outcome

- Scales to 1–10 M TPS while keeping L1 light and auditable
- Supports privacy and compliance with encrypted data and future ZK proofs
- Delivers deterministic global ordering and near-instant finality
- Enables L2 ecosystems anchored in a robust global trust layer

