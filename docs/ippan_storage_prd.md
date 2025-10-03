# IPPAN — Data Availability, Storage & Pruning, Fast Sync, and Confidential Transactions

## 1. Motivation

IPPAN targets sustained **10–50 ms blocks**, **1–10 M TPS**, and long-term auditability. At these rates, raw transaction data would exceed **tens to hundreds of terabytes per day** if every node stored every block body indefinitely. This document defines a model that keeps **headers universally replicated**, distributes **bodies via erasure-coded storage** (IPNDHT), enforces a **short hot retention window**, maintains **receipts and snapshots** for proof generation, and supports **optional confidentiality** for selected payloads.

---

## 2. Block Data Layout

Each block `B` consists of the following fields:

| Field | Description |
|-------|-------------|
| `prev_hashes[]` | HashTimer hashes of parent tips (BlockDAG references). |
| `hash_timer` | Canonical IPPAN Time timestamp plus entropy. |
| `tx_root` | Merkle/Verkle root of transactions (plaintext for public; ciphertext for confidential). |
| `erasure_root` | Merkle root of erasure-coded shards (ciphertext for confidential transactions). |
| `receipt_root` | Root of receipts proving execution and state transitions. |
| `state_root` | Verkle root of global state after executing `B`. |
| `validator_sigs[]` | Aggregate or multisignature of proposer plus round validators. |

- **Header** = all fields above (no raw transaction list, no full receipts/body).
- **Body** = raw transactions (public plaintext or confidential ciphertext), full receipts, and optional intermediate state diffs.

---

## 3. Data Availability (DA) & Erasure Coding

- Block bodies are **erasure-coded** prior to announcement. Default scheme: Reed–Solomon with **n = 16** and **k = 10** (10 data + 6 parity shards).
- Coded shards are published to the **IPNDHT** with metadata `{block_hash, shard_index, size, checksum, provider_peers[]}`.
- Validators perform **random shard sampling** every round. They fetch random shards from random peers; if the success rate is below **95%** within the timeout, the block is considered unavailable and the round cannot finalize.
- DA checks operate on **ciphertext** for confidential transactions—availability is orthogonal to readability.

---

## 4. Retention Model

| Node Role         | Headers  | Bodies                           | Receipts | Snapshots |
|-------------------|----------|----------------------------------|----------|-----------|
| **Validator**     | Forever  | Hot window **24–72 h**           | ≥ **90 d** | ≥ **90 d** |
| **Full Node**     | Forever  | Configurable (**interest-based pinning**) | ≥ **90 d** | ≥ **90 d** |
| **Archival Node** | Forever  | ≥ **1 y** (or indefinite)        | ≥ **1 y** | ≥ **1 y**  |

- The **hot window** allows late peers to sync and ensures dispute windows while bounding disk usage.
- **Receipts and snapshots** allow proofs and execution verification without raw transaction bodies.
- Archival nodes (voluntary and incentivized) preserve long-term history.

---

## 5. Fast Sync Procedure

1. **Bootstrap** — Fetch the latest **signed snapshot checkpoint**:
   ```json
   {
     "state_root": "0x...",
     "block_height": 12345678,
     "round_id": "0x...",
     "validator_set": ["ed25519:..."],
     "signature": "agg_sig"
   }
   ```
2. **Header sync** — Stream all headers from the checkpoint forward; verify signatures and DAG references.
3. **Optional body fetch** — For specific history, use `tx_root` and `erasure_root` to request shards and reconstruct bodies.
4. **DA verification** — Sample recent blocks’ shards to confirm network availability.

---

## 6. Economic & Incentive Layer

- **Announcement fee:** A small fee (e.g., ~10⁻⁸ IPN) per shard/pin broadcast deters spam.
- **Serving rewards:** Nodes that respond to shard audits receive micro-rewards.
- **Archival contracts:** Market for long-term retention where projects pay archival nodes to pin historical data.

---

## 7. Networking / RPC Interfaces (Initial)

### 7.1 DHT APIs

- `PUT /dht/shard`
- `GET /dht/shard?block=HASH&index=i`
- `HEAD /dht/block/availability?block=HASH`

Example `PUT /dht/shard` body:
```json
{
  "block_hash": "0x...",
  "shard_index": 3,
  "size": 4096,
  "checksum": "0x...",
  "provider_peers": ["peer1", "peer2"]
}
```

### 7.2 Fast Sync APIs

- `GET /sync/checkpoint/latest`
- `GET /sync/headers?from=HEIGHT`
- `GET /sync/receipt?tx=TxID`
- `GET /sync/state/proof?key=STATE_KEY`

---

## 8. Security, Privacy & Compliance

- **Weak subjectivity:** Clients trust a recent checkpoint signed by a supermajority of validators.
- **Privacy:** Prunable payloads ensure only cryptographic commitments remain globally replicated.
- **Auditability:** Any body (public or confidential) can be reconstructed from shards if keys are available.
- **Regulatory alignment:** Receipts provide durable evidence while enabling data minimization (e.g., GDPR patterns).

---

## 9. Parameters (Initial Defaults)

| Parameter | Value |
|-----------|-------|
| Block size | 4–32 KB (max 128 KB) |
| Hot window | 48 h |
| Snapshot interval | 10 min |
| Erasure coding | RS n=16, k=10 |
| Shard availability threshold | ≥ 95% |
| Receipt retention | ≥ 90 d |
| Archival replication target | 5× effective |

---

## 10. Rationale

- Achieves scalability (≥10 M TPS) with bounded per-node storage.
- Maintains universal verifiability through headers/receipts and DA sampling.
- Supports low-latency onboarding for new nodes via fast sync.
- Enables economic sustainability: long-term storage is opt-in/paid; short-term retention is universal.

---

## 11. Confidential Transactions and Mixed Visibility

### 11.1 Motivation

The network must support permanent public data (e.g., DNS handles) and confidential payloads readable only by entitled parties. Confidentiality must coexist with the DA and pruning model.

### 11.2 Transaction Visibility Flag

Each transaction includes a `visibility` field:

| Visibility | Meaning |
|------------|---------|
| `public` | Payload is plaintext and globally readable. |
| `confidential` | Payload is encrypted; only commitments and access metadata are public. |

Blocks may freely mix public and confidential transactions.

### 11.3 Confidential Payload Envelope

For `visibility = confidential`, the transaction body is replaced with an encryption envelope:

```json
{
  "enc_algo": "AES-256-GCM",
  "iv": "base64...",
  "ciphertext": "base64...",
  "access_keys": [
    { "recipient_pub": "ed25519:abc...", "enc_key": "base64..." },
    { "recipient_pub": "ed25519:def...", "enc_key": "base64..." }
  ]
}
```

- Generate a random symmetric key `K` per transaction; encrypt the payload with `K`.
- For each entitled recipient, include `{recipient_pub, enc_key = Enc_{recipient_pub}(K)}`.
- `tx_root` and `erasure_root` commit to the ciphertext (not plaintext).

### 11.4 DA & Storage Impact

- IPNDHT stores ciphertext shards for confidential transactions; DA sampling remains unchanged.
- Validators verify availability and coding without decrypting payloads.
- Pruning and retention rules are identical for public and confidential payloads.

### 11.5 Fast Sync with Confidentiality

- Nodes sync headers and state roots identically.
- Nodes possessing decryption keys can fetch shards and decrypt; others cannot.
- All nodes can verify inclusion and state transitions via receipts and roots.

### 11.6 Audit & Compliance

- Recipients (or access contracts) can reveal keys under lawful process.
- Optional view keys or multisig/threshold access policies are supported.
- Future work: zero-knowledge proofs to attest validity (balance/range/signature) without revealing plaintext.

### 11.7 Mixed Public/Confidential State

`state_root` may include:

- Cleartext state (e.g., DNS handles, payment addresses).
- Encrypted state commitments referencing ciphertext plus access policies.

### 11.8 Security Guarantees

- **Confidentiality:** Only holders of private keys in `access_keys[]` can read payloads.
- **Integrity:** Ciphertexts are immutably committed by `tx_root` and `erasure_root`.
- **Availability:** Ciphertext shards must satisfy DA sampling.

---

## 12. Rust Type Definitions (Reference)

Place these (or adapt them) under `crates/types/src/` (for example, `transaction.rs`, `block.rs`, `receipt.rs`, `snapshot.rs`). These definitions use `serde` for (de)serialization; cryptographic primitives are placeholders to be wired into the actual crypto modules.

```rust
// crates/types/src/transaction.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionVisibility {
    Public,
    Confidential,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessKey {
    /// Recipient public key (e.g., ed25519, encoded as multibase/multicodec string)
    pub recipient_pub: String,
    /// Symmetric key K encrypted to recipient_pub (base64 or hex)
    pub enc_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfidentialEnvelope {
    /// e.g., "AES-256-GCM"
    pub enc_algo: String,
    /// IV / nonce (base64 or hex)
    pub iv: String,
    /// Ciphertext of the raw transaction payload (base64 or hex)
    pub ciphertext: String,
    /// One entry per entitled reader
    pub access_keys: Vec<AccessKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
pub enum TransactionBody {
    /// Plaintext payload for public transactions
    Public(Vec<u8>),
    /// Encrypted payload envelope for confidential transactions
    Confidential(ConfidentialEnvelope),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique tx identifier (hash of canonical encoding)
    pub tx_id: String,
    /// Visibility flag aligning with the body type
    pub visibility: TransactionVisibility,
    /// Account/contract that originates the tx (could be hidden in ciphertext for confidential)
    pub from_hint: Option<String>,
    /// Optional cleartext topic/tags for routing or indexing
    pub topics: Vec<String>,
    /// Body holds either plaintext bytes or the encryption envelope
    pub body: TransactionBody,
    /// Signature over canonical transaction (for public txs); for confidential,
    /// you may sign the commitments or use ZK proofs in future.
    pub signature: Option<String>,
}

// crates/types/src/receipt.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// ID of the transaction
    pub tx_id: String,
    /// Outcome code (e.g., Success/Failure codes)
    pub status: u16,
    /// Gas/fee used or metered resource summary (if applicable)
    pub resource_used: u64,
    /// Root/commitment of touched state keys for this tx
    pub touched_keys_root: String,
    /// Optional opaque proof bytes (e.g., Verkle/Merkle proofs)
    pub proof_blob: Option<Vec<u8>>,
}

// crates/types/src/block.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub prev_hashes: Vec<String>,   // Hashes of parent tips (DAG)
    pub hash_timer: String,         // Encoded IPPAN Time + entropy
    pub tx_root: String,            // Root over txs (plaintext or ciphertext)
    pub erasure_root: String,       // Root over erasure-coded shards
    pub receipt_root: String,       // Root over receipts
    pub state_root: String,         // Verkle root after executing this block
    pub validator_sigs: Vec<String> // Aggregate/multisig; encoding TBD
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockBody {
    /// Transactions can be public (plaintext) or confidential (ciphertext envelope)
    pub transactions: Vec<crate::transaction::Transaction>,
    /// Full receipts (may be pruned after retention window)
    pub receipts: Vec<crate::receipt::Receipt>,
    /// Optional intermediate state diffs (implementation-defined)
    pub state_diffs: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    /// Body is optional for nodes that prune beyond the hot window.
    pub body: Option<BlockBody>,
}

// crates/types/src/snapshot.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotCheckpoint {
    pub state_root: String,
    pub block_height: u64,
    pub round_id: String,
    pub validator_set: Vec<String>, // e.g., ed25519 pubkeys
    pub signature: String,          // aggregate signature over the checkpoint
}
```

---

## 13. Implementation Notes & Guidance

- **Encoding choices:** Prefer SSZ/CBORg or canonical bincode with length-prefixing for stable hashes; document endianness explicitly.
- **Roots/tries:** Use Verkle for compact state proofs; MPT or Merkle trees for transaction and receipt trees are acceptable initially.
- **Cipher suites:** Start with AES-256-GCM for envelopes; XChaCha20-Poly1305 is a viable alternative. Key wraps can use recipient X25519/Ed25519-DH or HPKE (recommended long term).
- **Access policies:** For group/threshold access, store a pointer to a policy contract that manages rotating view keys; commit to the policy’s hash/ID on-chain.
- **DA sampling:** Parameterize sample counts and timeouts by observed network latency; expose Prometheus metrics for failure rates.
- **Pruning tooling:** Provide CLI/RPC to (a) prune bodies beyond the hot window, (b) compact receipts, and (c) export/import snapshots.
- **Interest-based pinning:** Allow nodes to specify topics/accounts/handles to keep; translate preferences into DHT pin rules.
- **Audits:** Perform periodic shard-serving audits. Request a random shard; the prover includes a small STARK/Merkle proof-of-possession (or signed hash with nonce) to claim rewards.

---

## 14. Compatibility with Public Data (e.g., DNS / Handle Registry)

- Public registry updates are plaintext transactions; their values live in cleartext state.
- Confidential application messages live as ciphertext with commitments anchored in state.
- Both transaction types share the same blocks, rounds, DA procedures, and pruning logic.

---

## 15. Example: Minimal Confidential Transaction Flow

1. Client builds plaintext payload `P`.
2. Generate random key `K`; compute ciphertext `C = Enc(K, P)`.
3. For each recipient `Rᵢ`, compute `enc_keyᵢ = Enc_{Rᵢ.pub}(K)`.
4. Construct `ConfidentialEnvelope { enc_algo, iv, ciphertext = C, access_keys = [...] }`.
5. Create `Transaction { visibility = Confidential, body = Confidential(envelope), ... }`.
6. Include in a block, compute `tx_root` and `erasure_root` over ciphertexts, erasure-code the body, publish shards to IPNDHT.
7. Validators perform DA sampling; the round finalizes.
8. Entitled recipients fetch shards, reconstruct the body, and decrypt using their `enc_keyᵢ`.

---

## 16. Open Extensions (Roadmap)

- **HPKE-based envelopes** (RFC 9180) for standardized hybrid encryption.
- **Zero-knowledge validity proofs** (balance/range/signature hiding) for confidential value transfers.
- **KMS/HSM integration** for enterprise key custody and regulated access.
- **Data retention markets** with slashing for unserved pins.

---

## 17. ZK-STARK Integration Roadmap

### 17.1 Purpose

Zero-Knowledge Scalable Transparent Arguments of Knowledge (ZK-STARKs) allow a node to prove the correctness of a transaction or block’s state transition **without revealing sensitive data**. IPPAN’s confidentiality model (Section 8) already hides payloads by encryption, but does not by itself prove their validity. Adding STARKs enables:

- **Private but valid transfers** — e.g., balances and spend rules can be enforced while amounts remain hidden.
- **Regulatory selective disclosure** — auditors can verify compliance conditions without full plaintext.
- **Post-quantum security** — STARK proofs are based on hash primitives and remain secure against quantum adversaries.

### 17.2 Integration Stages

| Stage | Objective | Deliverables |
|-------|-----------|--------------|
| **Phase 0 (current)** | Confidential payloads only (no validity proof). | Encryption envelope + DA. |
| **Phase 1** | Add STARK proof per confidential transaction. | Circuits: signature validity, balance conservation, non-negative amounts. Transaction carries: `proof_bytes`, `public_inputs`. |
| **Phase 2** | Batch validity proofs per block/round. | Block receipts include aggregate STARK verifying all confidential transactions. |
| **Phase 3** | Optional application-specific proofs. | Applications submit domain-specific STARKs (e.g., KYC policy, AML checks). |

### 17.3 Transaction Format Extension

For confidential transactions, add:

```json
{
  "proof_type": "stark",
  "proof": "base64...",
  "public_inputs": {
    "tx_id": "0x...",
    "sender_commit": "0x...",
    "receiver_commit": "0x...",
    "sequence_length": "32",
    "result": "5702887"
  }
}
```

- `proof` carries the base64-encoded Winterfell proof bytes.
- `tx_id` must match the transaction's canonical hash.
- `sender_commit` and `receiver_commit` are Blake3 commitments to sender/nonce and receiver/amount respectively.
- `sequence_length` and `result` bind the STARK to the confidential transfer arithmetic (Fibonacci reference circuit in the reference implementation).

---

> _If you need companion materials (commit message templates, PR description templates, or stub Axum RPC handlers), reach out to the IPPAN docs maintainers or generate them alongside this specification._
