# IPPAN — Global Layer-1 Blockchain, Data Availability & Parallel Consensus

*(Updated December 2025 — Deterministic Learning Consensus integration)*

---

## 1. IPPAN as a Global Layer-1 Blockchain

### 1.1 Mission
IPPAN is a **global, neutral, permissionless Layer-1 (L1) ledger** anchoring time, identity, and trust for any distributed system. L1 is deliberately minimal and deterministic — it stores only what is needed for **global ordering, auditability, and interoperability** — while application logic, large payloads, and high-volume data live in **Layer-2 (L2) systems**.

### 1.2 Design Principles
- **Deterministic Time & Ordering** — IPPAN Time (median network time with 100 ms precision) and HashTimer anchors yield a single, provable timeline.
- **Minimal Canonical State** — Only block/round headers, anchors, handle ownership proofs, and L2 chain roots go on L1.
- **Decentralization & Survivability** — Network can restart from as few as two IPNWorker nodes; libp2p + DHT peer discovery; NAT hole punching and offline recovery.

### 1.3 L1 vs L2 Data Allocation

| Layer | Content | Typical Use |
|-------|---------|-------------|
| **L1** | Headers, HashTimers, handle ownership anchors, anchor roots for L2 chains/DHTs, ZK/STARK proofs | Timestamping, identity ownership proofs, compliance anchors, cross-chain interoperability |
| **L2** | Transaction bodies, app states, large content, confidential or regulated data | Payments, DeFi, healthcare, IoT, file storage |

- **Anchors:** ≤512 B per L2 commit  
- **Retrieval:** Clients verify on L1; fetch full data only when needed

### 1.4 Confidentiality & Compliance
- Personally identifiable or sensitive data **never resides in plaintext on L1**.  
- Confidential transactions: encrypted payloads, public commitments, optional ZK/STARK validity proofs.  
- Regulatory selective disclosure: keys can be revealed or STARK proofs supplied.

### 1.5 DNS & Human-Readable Identity
- **Global naming**: `@user.ipn` plus premium TLDs (`.cyborg`, `.iot`, `.m`) stored on L2.  
- L1 only stores ownership anchors (minimal proofs of handle ownership).
- Handle mappings and metadata live on L2 for scalability.
- Handle updates pay small IPN fees; fees redistributed to validators.  
- L2 namespaces can extend the handle registry.

### 1.6 Scalability & Performance
- **BlockDAG + Rounds** → 10–50 ms block frequency, 200–250 ms round finality, 1–10 M TPS.
- Ultra-light headers (4–32 KB blocks, 128 KB max).  
- Interoperability anchors: Ethereum/Bitcoin/other chains can commit to IPPAN.

### 1.7 Cost & Energy Model
- Lightweight participation; micro-fees (e.g. 10⁻⁸ IPN) for announcements and handle ownership anchor updates to deter spam but keep user cost negligible.

---

## 2. Deterministic Learning Consensus (DLC) Model

### 2.1 Revolutionary Paradigm Shift

IPPAN departs from traditional Byzantine Fault Tolerant (BFT) consensus mechanisms, introducing a new class of consensus: **Deterministic Learning Consensus (DLC)**. This model replaces voting-based agreement with temporal determinism and adaptive learning.

### 2.2 Core Architecture

**Deterministic Learning Consensus Stack:**
```
┌─────────────────────────────────────────────────────────────┐
│                Application Layer                            │
│  • Transaction Processing  • Smart Contracts  • DApps      │
├─────────────────────────────────────────────────────────────┤
│              Learning & Adaptation Layer                    │
│  • D-GBDT Models  • Reputation Scoring  • AI Optimization  │
├─────────────────────────────────────────────────────────────┤
│              Temporal Consensus Layer                       │
│  • HashTimer™  • BlockDAG  • Deterministic Ordering        │
├─────────────────────────────────────────────────────────────┤
│              Cryptographic Foundation                       │
│  • Ed25519 Signatures  • zk-STARKs  • Merkle Trees         │
└─────────────────────────────────────────────────────────────┘
```

### 2.3 Temporal Determinism via HashTimer™

**HashTimer Structure:**
```rust
pub struct HashTimer {
    timestamp_us: i64,           // Microsecond precision timestamp
    entropy: [u8; 32],          // Cryptographic entropy
    signature: Option<Signature>, // Ed25519 signature
    public_key: Option<PublicKey>, // Validator public key
}
```

**Key Properties:**
- **Median network time** with microsecond precision
- **Bounded time adjustments** (±5ms maximum per update)
- **Monotonic guarantees** preventing time regression
- **Cryptographic verification** ensuring tamper-proof timestamps

### 2.4 BlockDAG Structure

**Block Header:**
| Field | Description |
|-------|-------------|
| `id` | 32-byte block hash |
| `creator` | Validator public key hash |
| `round` | Round identifier |
| `hashtimer` | HashTimer temporal anchor |
| `parent_ids` | Multiple parent block IDs (DAG) |
| `payload_ids` | Transaction batch references |
| `merkle_payload` | Root of included transactions |
| `merkle_parents` | Root of parent block hashes |
| `signature` | Ed25519 signature |

**Round Structure:**
- Fixed time window (200–250 ms) via HashTimer synchronization
- Fields: `RoundId`, `RoundWindow`, `RoundCertificate`, `RoundFinalizationRecord`
- **No voting required** — temporal determinism ensures agreement

### 2.5 AI-Driven Optimization

**L1 AI Consensus Engine:**
```rust
pub struct L1AIConsensus {
    validator_selection_model: Option<GBDTModel>,
    fee_optimization_model: Option<GBDTModel>,
    network_health_model: Option<GBDTModel>,
    block_ordering_model: Option<GBDTModel>,
}
```

**AI Evaluation Factors:**
- **Reputation score** (40% weight)
- **Block production rate** (30% weight)  
- **Uptime percentage** (20% weight)
- **Network contribution** (10% weight)

### 2.6 Protocol Flow

1. **HashTimer Synchronization**  
   All nodes synchronize to median network time with microsecond precision.

2. **Parallel Block Creation**  
   Validators create blocks referencing multiple parents from previous round.

3. **AI Evaluation & Selection**  
   D-GBDT models evaluate validators and optimize resource allocation.

4. **Deterministic Ordering**  
   Blocks ordered by HashTimer timestamp, then by hash for tie-breaking.

5. **Round Finalization**  
   Deterministic execution and state computation without voting rounds.

### 2.7 Performance Characteristics

| Metric | Traditional BFT | IPPAN DLC | Improvement |
|--------|-----------------|-----------|-------------|
| **Max Validators** | ~100 | 1000+ | 10x |
| **TPS** | ~1000 | 10M+ | 10,000x |
| **Finality** | 1-10s | 100-250ms | 40x |
| **Communication** | O(n²) | O(n) | n× |
| **Energy** | High | Low | 100x |

### 2.8 Safety & Security

- **Temporal determinism** prevents ordering attacks
- **Statistical consensus** provides fault tolerance  
- **AI reputation system** detects and penalizes malicious behavior
- **Economic incentives** align validator interests with network health
- **Cryptographic security** with Ed25519 and zk-STARKs

### 2.9 Networking

- **Gossip topics**: `block-headers:r`, `block-payloads`, `ai-telemetry:r`, `round-finalizations:r`
- **Anti-DoS**: micro-fee for announcements, AI-based spam detection
- **Peer discovery**: mDNS + Kademlia DHT for global connectivity

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

### 7.1 Fractional IPN Unit Architecture (Atomic Accounting)

IPPAN accounts value in atomic units with yocto-level precision to match the HashTimer cadence and DAG-Fair emission granularity.

#### Denominations

| Name | Symbol | Value in IPN | Typical use |
|------|--------|--------------|-------------|
| IPN  | 1 IPN  | 1            | governance, staking |
| mIPN | milli  | 10⁻³         | validator micro-rewards |
| µIPN | micro  | 10⁻⁸         | transaction fees (UI default unit) |
| aIPN | atto   | 10⁻¹⁸        | IoT/AI micro-service calls |
| zIPN | zepto  | 10⁻²¹        | sub-millisecond machine triggers |
| yIPN | yocto  | 10⁻²⁴        | HashTimer precision-level payments |

- Smallest accepted fraction: \(1\,\text{yIPN} = 10^{-24}\,\text{IPN}\).
- Total atomic supply: \(\text{Supply}_{\text{atomic}} = 21{,}000{,}000 \times 10^{24}\).

#### Rationale

- Deterministic micro-settlement aligned to HashTimer events (10–50 ms rounds)
- Fair parallel reward splits without rounding drift
- Machine-to-machine economies (AI, IoT, DePIN) with negligible unit exhaustion risk
- Integer math only — no floating point in consensus-critical paths

#### Ledger representation (Rust)

```rust
/// IPN is stored as fixed-point integer with 24 decimal places.
/// 1 IPN = 10^24 atomic units.
pub type AtomicIPN = u128;

pub const IPN_DECIMALS: u32 = 24;
pub const ATOMIC_PER_IPN: AtomicIPN = 10u128.pow(IPN_DECIMALS);

/// Example: convert 0.000000000000000000000001 IPN to atomic units
let one_yocto: AtomicIPN = 1u128; // 1 atomic unit
```

Human interfaces default to 8–12 fractional digits (e.g., µIPN) while preserving full 24-decimal accuracy in storage, wallet math, and transaction validation.

#### Economic consistency

| Property | Effect |
|----------|--------|
| No inflation | Fixed atomic supply: 21 M × 10²⁴ |
| Rounding-safe | DAG-Fair splits in integers; deterministic remainder handling |
| Deterministic | Pure integer arithmetic; no float drift across nodes |
| Audit-ready | Round records include reward and sub-unit checksum proofs |

#### Example — validator reward split

Given round reward \(R(t) = 10^{-4}\,\text{IPN}\):

\[
R_{\text{atomic}} = R(t) \times 10^{24} = 10^{20}\,\text{units}
\]

For \(B_r = 1000\) blocks in the round, an equal split yields:

\[
\left\lfloor \frac{10^{20}}{1000} \right\rfloor = 10^{17}\,\text{units} = 10^{-7}\,\text{IPN}\;\text{per block}
\]

Any remainder \(r < B_r\) is handled deterministically (e.g., assigned by HashTimer order or carried-forward/burned per protocol), preserving conservation of supply with no ambiguous rounding.

### 7.2 Interactions: DAG-Fair Emission and HashTimer Micropayments

- DAG-Fair emission computes validator and contributor shares in atomic units, ensuring exactness across thousands of micro-blocks per round.
- HashTimer-anchored micropayments settle at sub-IPN scales with yocto precision, matching the temporal resolution of the ordering layer.
- Wallets and RPC present user-friendly denominations (e.g., µIPN) while nodes exchange and verify integer atomic values.

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

- Scales to 10M+ TPS through Deterministic Learning Consensus while keeping L1 light and auditable
- Supports privacy and compliance with encrypted data and future ZK proofs  
- Delivers deterministic global ordering and 100-250ms finality
- Enables L2 ecosystems anchored in a robust global trust layer
- **Revolutionary consensus**: Replaces traditional BFT with temporal determinism and AI learning

---

## References

- [Beyond BFT: Deterministic Learning Consensus Model](../BEYOND_BFT_DETERMINISTIC_LEARNING_CONSENSUS.md) — Complete theoretical foundation and mathematical proofs
- [IPPAN Vision 2025](./ippan-vision-2025.md) — Product strategy and business objectives
- [DAG-Fair Emission System](../DAG_FAIR_EMISSION_SYSTEM.md) — Economic model specification

