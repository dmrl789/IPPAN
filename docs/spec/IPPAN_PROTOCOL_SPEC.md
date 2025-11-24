# IPPAN Protocol Specification v1.0

**Status:** Release Candidate 1  
**Version:** 1.0.0-rc1  
**Date:** 2025-11-24  
**Authors:** IPPAN Development Team

---

## Document Purpose

This document serves as the **canonical, implementation-agnostic protocol specification** for IPPAN (Internet Protocol Protocol Autonomous Network). It defines the normative behavior of IPPAN nodes, consensus mechanisms, economic rules, and network protocols using precise, unambiguous language.

**Intended Audience:**
- Protocol implementers (in Rust or other languages)
- External auditors
- Academic researchers
- Standards bodies

**Normative Language:** This specification uses RFC 2119 keywords:
- **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**: Absolute requirements
- **SHOULD**, **SHOULD NOT**, **RECOMMENDED**: Strong recommendations (may deviate with justification)
- **MAY**, **OPTIONAL**: Truly optional features

---

## Table of Contents

1. [Overview](#1-overview)
2. [IPPAN Time & HashTimer](#2-ippan-time--hashtimer)
3. [Block Structure](#3-block-structure)
4. [BlockDAG & Roundchain](#4-blockdag--roundchain)
5. [Deterministic Fork Choice](#5-deterministic-fork-choice)
6. [DLC: Deterministic Learning Consensus](#6-dlc-deterministic-learning-consensus)
7. [D-GBDT: Verifier Selection](#7-d-gbdt-verifier-selection)
8. [Emission & DAG-Fair Rewards](#8-emission--dag-fair-rewards)
9. [Transaction Lifecycle](#9-transaction-lifecycle)
10. [Network Protocol](#10-network-protocol)
11. [Storage & State Model](#11-storage--state-model)
12. [Security Model](#12-security-model)
13. [Implementation Notes](#13-implementation-notes)

---

## 1. Overview

### 1.1 System Architecture

IPPAN is a **distributed ledger** that uses:
- **HashTimer™** for deterministic temporal ordering
- **BlockDAG** for parallel block production
- **Deterministic Learning Consensus (DLC)** for validator selection and finalization
- **D-GBDT (Deterministic Gradient-Boosted Decision Trees)** for fairness
- **DAG-Fair Emission** for round-based token distribution

### 1.2 Design Principles

**MUST** adhere to:
1. **Determinism:** All consensus-critical operations MUST be deterministic across implementations
2. **Safety:** Finalized state MUST never contradict
3. **Liveness:** Network MUST make progress under ≤33% Byzantine faults
4. **Fairness:** Validator selection MUST be proportional to reputation scores
5. **Auditability:** All state transitions MUST be verifiable

### 1.3 Key Properties

| Property | Value |
|----------|-------|
| **Target Round Duration** | 200ms |
| **Finality Depth** | 2 rounds (~400ms) |
| **Max Reorg Depth** | 2 blocks |
| **Supply Cap** | 21,000,000 IPN (hard cap) |
| **Minimum Validator Bond** | 10 IPN |
| **Byzantine Tolerance** | <33% of bonded stake |

---

## 2. IPPAN Time & HashTimer

### 2.1 IPPAN Time Definition

**IPPAN Time** is a deterministic temporal reference measured in **microseconds** since an epoch.

#### 2.1.1 Requirements

- Nodes MUST use UTC as the base clock
- Nodes MUST use NTP or equivalent for clock synchronization
- Nodes MUST reject timestamps that deviate >500ms from their local clock (after accounting for network delay)
- Nodes MUST use **microsecond precision** (64-bit unsigned integer)

#### 2.1.2 Representation

```rust
type IppanTimeMicros = u64;  // Microseconds since epoch (2020-01-01 00:00:00 UTC)
```

### 2.2 HashTimer Construction

**HashTimer** is a cryptographic time anchor that binds an event to deterministic IPPAN Time.

#### 2.2.1 Structure

```rust
struct HashTimer {
    time: IppanTimeMicros,      // Microsecond timestamp
    hash: [u8; 32],              // BLAKE3 hash
}
```

#### 2.2.2 Hash Derivation

The `hash` field MUST be computed as:

```
hash = BLAKE3(
    domain || 
    time_bytes || 
    domain_data || 
    payload || 
    creator_id
)
```

Where:
- `domain`: UTF-8 string (e.g., `"ippan_block_v1"`, `"ippan_round_v1"`)
- `time_bytes`: 8-byte little-endian encoding of `time`
- `domain_data`: Domain-specific context bytes
- `payload`: Event-specific data
- `creator_id`: 32-byte validator ID

#### 2.2.3 Ordering Rules

HashTimer instances MUST be ordered by:

1. **Primary:** `time` (ascending)
2. **Secondary:** `hash` (lexicographic, ascending)

If `A.time < B.time`, then `A < B`.  
If `A.time == B.time`, then `A < B` iff `A.hash < B.hash`.

#### 2.2.4 Validation

Nodes MUST verify:
- `time` is within acceptable drift (±500ms from local clock)
- `hash` is correctly computed from inputs
- `hash` matches expected domain

---

## 3. Block Structure

### 3.1 Block Definition

A **block** is the fundamental unit of data propagation in IPPAN.

#### 3.1.1 Block Schema

```rust
struct Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
    signature: Signature,  // Ed25519 signature over header hash
}

struct BlockHeader {
    id: BlockId,                // BLAKE3(canonical_header_bytes)
    creator: ValidatorId,       // 32-byte Ed25519 public key
    round: u64,                 // Round number
    hashtimer: HashTimer,       // Temporal anchor
    parent_ids: Vec<BlockId>,   // Parent blocks (1-16 parents)
    state_root: [u8; 32],       // Merkle root of state
    tx_merkle_root: [u8; 32],   // Merkle root of transactions
}

type BlockId = [u8; 32];      // BLAKE3 hash
type ValidatorId = [u8; 32];  // Ed25519 public key
```

### 3.2 Block Creation Rules

#### 3.2.1 Parent Selection

- A block MUST reference 1 to 16 parent blocks
- Parents MUST be from the current or previous round
- Parents MUST NOT form cycles (acyclicity requirement)
- A genesis block MAY have zero parents

#### 3.2.2 Round Assignment

- `header.round` MUST be the current round number
- Round number MUST be monotonically increasing
- Round number MUST NOT skip values (no gaps)

#### 3.2.3 HashTimer Generation

- `header.hashtimer` MUST be computed using domain `"ippan_block_v1"`
- `domain_data` MUST include parent IDs
- `payload` MUST include transaction merkle root
- `time` MUST be ≤ current IPPAN Time

#### 3.2.4 State Root

- `state_root` MUST be the Merkle root of all account states after applying transactions
- State computation MUST be deterministic

#### 3.2.5 Signature

- `signature` MUST be an Ed25519 signature over `BLAKE3(canonical_header_bytes)`
- Signing key MUST match `header.creator`
- Signature MUST be verifiable by all nodes

### 3.3 Block Validation

Upon receiving a block, nodes MUST verify:

1. **Signature validity:** Ed25519 signature matches `creator` and `header`
2. **HashTimer validity:** `hashtimer` is correctly computed and within drift tolerance
3. **Parent existence:** All `parent_ids` are known and valid
4. **Round consistency:** `round` matches consensus round
5. **Transaction validity:** All transactions are valid (see §9)
6. **State root correctness:** `state_root` matches computed state after applying txs
7. **No double-spending:** No transaction conflicts with finalized state

---

## 4. BlockDAG & Roundchain

### 4.1 BlockDAG Definition

The **BlockDAG** is a directed acyclic graph (DAG) of blocks where edges represent parent-child relationships.

#### 4.1.1 DAG Properties

- **Acyclicity:** MUST NOT contain cycles
- **Parallelism:** Multiple blocks MAY exist at the same height/round
- **Convergence:** All honest nodes MUST converge to the same canonical ordering

### 4.2 Roundchain Definition

The **Roundchain** is a linearization of the BlockDAG organized into **rounds**.

#### 4.2.1 Round Structure

- A **round** is a fixed time window (default: 200ms)
- All blocks with `round == N` belong to round N
- Rounds MUST be sequential (no gaps)

#### 4.2.2 Round Closure

Round N MUST close when:
- HashTimer indicates end of round window, OR
- Next round (N+1) has begun

### 4.3 DAG Invariants

Implementations MUST enforce:

1. **No conflicting finalized blocks:** Once finalized, blocks MUST NOT be reverted
2. **Deterministic ordering:** All nodes MUST agree on canonical tip (see §5)
3. **Bounded reorgs:** Maximum reorganization depth is 2 rounds
4. **Finality depth:** Blocks are finalized after 2 rounds (≥2 rounds deep)

---

## 5. Deterministic Fork Choice

### 5.1 Purpose

When multiple blocks exist at the same height, nodes MUST select a **canonical tip** deterministically.

### 5.2 Fork Choice Algorithm

Given a set of competing tips `{B₁, B₂, ..., Bₙ}`, the canonical tip MUST be selected by:

```
canonical_tip = argmax(B) by:
  1. round (highest)
  2. HashTimer.time (earliest)
  3. D-GBDT weight (highest, if applicable)
  4. BlockId (lexicographically smallest, tie-breaker)
```

#### 5.2.1 Comparison Rules

**Step 1:** Compare `B.round` (higher is preferred)

```
if A.round > B.round: return A
if B.round > A.round: return B
```

**Step 2:** Compare `B.hashtimer.time` (earlier is preferred)

```
if A.hashtimer.time < B.hashtimer.time: return A
if B.hashtimer.time > A.hashtimer.time: return B
```

**Step 3:** Compare D-GBDT weight (higher is preferred, see §7)

```
if weight(A.creator) > weight(B.creator): return A
if weight(B.creator) > weight(A.creator): return B
```

**Step 4:** Compare `B.id` (lexicographically smallest)

```
if A.id < B.id: return A
else: return B
```

### 5.3 Finalization Rules

A block MUST be considered **finalized** when:
- It is ≥2 rounds deep from the canonical tip
- It has been confirmed by ≥1 round of subsequent blocks

Once finalized, a block MUST NOT be reverted under any circumstances (safety guarantee).

---

## 6. DLC: Deterministic Learning Consensus

### 6.1 Overview

**Deterministic Learning Consensus (DLC)** replaces traditional voting-based consensus with:
- **Temporal finality:** Rounds close deterministically via HashTimer
- **Shadow verifiers:** Redundant validation (3-5 validators)
- **AI-driven fairness:** D-GBDT selects validators based on reputation

### 6.2 Round Execution

#### 6.2.1 Round Start

At the start of round N:

1. HashTimer MUST generate round anchor: `round_timer = HashTimer::for_round(N)`
2. D-GBDT MUST select verifiers (see §7)
3. Primary verifier MUST propose blocks from mempool
4. Shadow verifiers MUST validate proposed blocks in parallel

#### 6.2.2 Block Proposal

- **Primary verifier:** Proposes ≥1 block per round
- **Shadow verifiers:** Validate all proposed blocks
- **Other validators:** MAY propose blocks (permissionless)

#### 6.2.3 Validation Phase

Shadow verifiers MUST:
- Validate all transactions in proposed blocks
- Check state root correctness
- Verify signatures and HashTimers
- Flag inconsistencies (if primary misbehaves)

#### 6.2.4 Round Closure

Round N MUST close when:
- `current_time ≥ round_start_time + round_duration`, OR
- Round N+1 has started

### 6.3 Consensus Properties

#### 6.3.1 Safety

**Invariant:** Finalized state MUST NEVER contradict.

**Guarantee:** Under ≤33% Byzantine validators, all honest nodes MUST agree on finalized state.

#### 6.3.2 Liveness

**Invariant:** Network MUST make progress (finalize blocks) within bounded time.

**Guarantee:** If ≥67% validators are honest and network is synchronous, rounds MUST finalize within 2×round_duration.

### 6.4 Slashing Rules

Validators MUST be slashed for:

| Offense | Penalty | Detection |
|---------|---------|-----------|
| **Double-signing** | 50% of bond | Shadow verifiers detect conflicting signatures |
| **Invalid block proposal** | 10% of bond | Shadow verifiers reject invalid state |
| **Extended downtime** | 1% per missed round | Consensus tracks participation |
| **Equivocation** | 50% of bond | Multiple conflicting votes |

### 6.5 Validator Bonding

- Validators MUST bond ≥10 IPN to participate
- Bonds MUST be locked for ≥7 days after unbonding request
- Slashed funds MUST be burned or redistributed to treasury

---

## 7. D-GBDT: Verifier Selection

### 7.1 Purpose

**D-GBDT (Deterministic Gradient-Boosted Decision Trees)** selects validators for each round based on reputation scores.

### 7.2 Feature Vector

Each validator MUST be scored using the following features (scaled to integers):

| Feature | Weight | Scale | Description |
|---------|--------|-------|-------------|
| `uptime` | 0.25 | % × 1,000,000 | Fraction of rounds participated |
| `blocks_proposed` | 0.20 | count | Total blocks proposed |
| `blocks_verified` | 0.15 | count | Total blocks verified |
| `latency` | 0.15 | µs | Median block latency |
| `slash_penalty` | 0.10 | penalty count | Number of slashing events |
| `performance` | 0.10 | score × 1,000,000 | Recent performance metric |
| `stake` | 0.05 | µIPN | Bonded stake amount |

### 7.3 Scoring Algorithm

#### 7.3.1 Integer-Only Inference

All arithmetic MUST use **integer-only operations** (no floating-point).

```
SCALE = 1_000_000

score = 0
for tree in model.trees:
    node = tree.root
    while not node.is_leaf:
        feature_value = features[node.feature_index]
        if feature_value <= node.threshold:
            node = node.left
        else:
            node = node.right
    score += node.leaf_value

final_score = (score + model.bias) * model.post_scale / SCALE
```

#### 7.3.2 Deterministic Guarantees

- Same features MUST produce same score across all implementations
- Model parameters MUST be integer-scaled (thresholds, leaf values)
- Canonical JSON serialization MUST be used for model hashing

### 7.4 Verifier Selection

#### 7.4.1 Selection Seed

```
seed = BLAKE3("DLC_VERIFIER_SELECTION" || round_number)
```

#### 7.4.2 Weighted Random Selection

```
1. Compute scores for all validators: {v₁: s₁, v₂: s₂, ..., vₙ: sₙ}
2. Initialize PRNG with seed
3. Select primary validator using weighted random draw (weight = score)
4. Select K shadow validators (K = 3-5) using remaining weights
```

#### 7.4.3 Selection Fairness

Over a large number of rounds, selection frequency MUST be proportional to average scores (within statistical bounds).

---

## 8. Emission & DAG-Fair Rewards

### 8.1 Supply Cap

- **Total supply cap:** 21,000,000 IPN (MUST NOT exceed)
- **Precision:** Amounts stored as **micro-IPN** (µIPN): 1 IPN = 1,000,000 µIPN

### 8.2 Emission Formula

Emission is **per-round** (not per-block).

```
R(t) = R₀ / 2^(⌊t / Tₕ⌋)
```

Where:
- `R(t)` = Reward per round at round `t`
- `R₀` = Initial reward per round (10,000 µIPN = 0.0001 IPN)
- `Tₕ` = Halving interval (315,000,000 rounds ≈ 2 years at 200ms/round)
- `t` = Current round number

#### 8.2.1 Halving Schedule

| Halving | Rounds | Years | Annual Emission |
|---------|--------|-------|-----------------|
| 0 | 0 - 315M | 0-2 | 3.15M IPN |
| 1 | 315M - 630M | 2-4 | 1.58M IPN |
| 2 | 630M - 945M | 4-6 | 0.79M IPN |
| 3 | 945M - 1.26B | 6-8 | 0.40M IPN |

### 8.3 Reward Distribution (DAG-Fair)

Rewards for round N MUST be distributed as:

| Component | Percentage | Basis |
|-----------|------------|-------|
| **Base Emission** | 60% | Proportional to participation |
| **Transaction Fees** | 25% | Proportional to blocks processed |
| **AI Commissions** | 10% | Proportional to AI service usage |
| **Network Pool** | 5% | Distributed to all validators |

### 8.4 Participation Scoring

Validators MUST be rewarded proportionally to:

```
participation_score = 
    block_count × 0.40 +
    uptime × 0.30 +
    reputation × 0.20 +
    stake × 0.10
```

### 8.5 Role Multipliers

| Role | Multiplier |
|------|------------|
| **Primary Verifier** | 1.2× (20% bonus) |
| **Shadow Verifier** | 1.0× (standard) |
| **AI Service Provider** | 1.1× (10% bonus) |

### 8.6 Emission Invariants

Implementations MUST enforce:

1. **Supply cap:** Total emitted supply ≤ 21M IPN
2. **Reward accounting:** Sum of distributed rewards ≤ emitted supply
3. **No negative rewards:** All reward amounts ≥ 0
4. **Deterministic distribution:** Same participation → same rewards

---

## 9. Transaction Lifecycle

### 9.1 Transaction Types

IPPAN supports the following transaction types:

| Type | Description |
|------|-------------|
| **Payment** | Transfer IPN between accounts |
| **HandleRegistration** | Register a human-readable handle (`@username`) |
| **ValidatorBond** | Bond IPN to become a validator |
| **ValidatorUnbond** | Unbond validator stake |
| **GovernanceVote** | Vote on governance proposals |

### 9.2 Transaction Structure

```rust
struct Transaction {
    tx_type: TxType,
    sender: Address,        // 32-byte Ed25519 public key
    nonce: u64,             // Sender's transaction count
    payload: Vec<u8>,       // Type-specific data
    fee: u64,               // Fee in µIPN
    signature: Signature,   // Ed25519 signature
}
```

### 9.3 Transaction Validation

Nodes MUST verify:

1. **Signature:** Ed25519 signature is valid for `sender`
2. **Nonce:** `nonce == sender.current_nonce + 1` (prevents replay)
3. **Balance:** `sender.balance ≥ amount + fee`
4. **Fee:** Fee is within acceptable range (0.001 - 1 IPN)
5. **Payload:** Type-specific validation (e.g., handle format)

### 9.4 Transaction Processing

#### 9.4.1 Mempool Admission

- Transactions MUST be validated before entering mempool
- Mempool MUST prioritize by fee (highest first)
- Mempool MUST evict low-fee txs when full

#### 9.4.2 Block Inclusion

- Primary verifier MUST select transactions from mempool
- Transactions MUST be ordered by nonce (per sender)
- Conflicting transactions MUST NOT be included in the same block

#### 9.4.3 State Updates

Upon finalization of a block containing transaction `tx`:

```
sender.balance -= tx.amount + tx.fee
sender.nonce += 1
recipient.balance += tx.amount
treasury.balance += tx.fee
```

### 9.5 Finality

Transactions are **finalized** when their containing block is finalized (≥2 rounds deep).

---

## 10. Network Protocol

### 10.1 Protocol Stack

IPPAN uses **libp2p** for peer-to-peer networking.

#### 10.1.1 Transport

- **MUST support:** TCP, QUIC
- **SHOULD support:** WebSocket (for browser clients)
- **SHOULD support:** WebRTC (for NAT traversal)

#### 10.1.2 Encryption

- All connections MUST use TLS or Noise protocol
- Peer identities MUST be verified via public keys

### 10.2 Network Protocols

| Protocol | Purpose |
|----------|---------|
| **Gossipsub** | Broadcast blocks, transactions |
| **Kad DHT** | Peer discovery, content routing |
| **Request-Response** | Sync missing blocks, query state |

### 10.3 Gossipsub Topics

- `/ippan/blocks/v1` - Block propagation
- `/ippan/txs/v1` - Transaction propagation
- `/ippan/announcements/v1` - Network announcements

### 10.4 Message Format

All network messages MUST use **Protobuf** or **CBOR** encoding.

### 10.5 Security Measures

#### 10.5.1 Message Size Limits

- Gossipsub messages MUST be ≤1 MB
- Request-response payloads MUST be ≤10 MB
- Nodes MUST drop oversized messages

#### 10.5.2 Rate Limiting

- Nodes MUST limit incoming messages to 1000/sec per peer
- Nodes MUST ban peers exceeding rate limits (1 hour cooldown)

#### 10.5.3 Peer Scoring

- Nodes SHOULD score peers based on behavior (valid msgs, latency)
- Nodes SHOULD prune low-scoring peers

---

## 11. Storage & State Model

### 11.1 State Structure

IPPAN state consists of:

```rust
struct State {
    accounts: HashMap<Address, Account>,
    supply: u64,              // Total circulating supply (µIPN)
    round: u64,               // Current round number
    state_root: [u8; 32],     // Merkle root of accounts
}

struct Account {
    balance: u64,             // Balance in µIPN
    nonce: u64,               // Transaction count
    handles: Vec<String>,     // Registered handles
}
```

### 11.2 State Transitions

State updates MUST be deterministic:

```
new_state = apply(old_state, transaction)
```

All nodes MUST compute identical state roots after applying the same transactions.

### 11.3 Persistence

#### 11.3.1 Storage Backends

- **Production:** Sled (embedded key-value store)
- **Testing:** In-memory HashMap

#### 11.3.2 Data Model

| Key | Value |
|-----|-------|
| `account:<addr>` | Serialized `Account` |
| `block:<id>` | Serialized `Block` |
| `tx:<id>` | Serialized `Transaction` |
| `state:current` | Current `State` |

### 11.4 Snapshots

Nodes MUST support exporting and importing state snapshots:

```
snapshot = export_state(round_number)
import_state(snapshot) -> State
```

Snapshots MUST include:
- All account states
- Current round number
- State root
- Total supply

---

## 12. Security Model

### 12.1 Threat Model

#### 12.1.1 Assumptions

- ≤33% of bonded stake is Byzantine (adversarial)
- ≤50% of primary verifiers are Byzantine (shadow verifiers compensate)
- Network partitions heal within finite time

#### 12.1.2 Threat Actors

| Actor | Capability | Mitigation |
|-------|------------|------------|
| **Malicious Validator** | Double-sign, equivocate | Slashing (50% bond) |
| **Network Adversary** | Partition, delay, drop messages | Timeout + resync |
| **RPC Abuser** | Spam endpoints | Rate limiting |
| **P2P Attacker** | Flood peers | Message size limits, banning |

### 12.2 Security Guarantees

#### 12.2.1 Safety

**Guarantee:** Finalized state MUST NEVER contradict.

**Proof Sketch:** Shadow verifiers detect equivocation; slashing removes Byzantine validators.

#### 12.2.2 Liveness

**Guarantee:** Network MUST finalize blocks within bounded time (2×round_duration).

**Proof Sketch:** ≥67% honest validators ensure progress via temporal finality.

#### 12.2.3 Fairness

**Guarantee:** Validator selection frequency MUST be proportional to reputation scores.

**Proof Sketch:** D-GBDT uses deterministic weighted random selection with verifiable seeds.

---

## 13. Implementation Notes

This section provides non-normative guidance for implementers.

### 13.1 Reference Implementation

The canonical Rust implementation is located at:
- **Repository:** https://github.com/dmrl789/IPPAN
- **Crates:**
  - `crates/consensus` - Consensus engine, emission tracker
  - `crates/consensus_dlc` - DLC & DAG logic
  - `crates/ai_core` - D-GBDT inference engine
  - `crates/time` - HashTimer implementation
  - `crates/storage` - Persistence layer
  - `crates/rpc` - HTTP/WebSocket API
  - `crates/p2p` - libp2p networking

### 13.2 Key Implementation Details

#### 13.2.1 Fork Choice

See: `crates/consensus_dlc/src/dag.rs::select_canonical_tip()`

#### 13.2.2 Emission Calculation

See: `crates/consensus/src/emission_tracker.rs::calculate_round_reward()`

#### 13.2.3 D-GBDT Inference

See: `crates/ai_core/src/gbdt/inference.rs::score()`

#### 13.2.4 Slashing Logic

See: `crates/consensus_dlc/src/verifier.rs::apply_slashing()`

### 13.3 Deviations & Extensions

Implementations MAY deviate from this spec if:
- Behavior is strictly more restrictive (e.g., lower message size limits)
- Extensions are clearly documented and opt-in
- Consensus rules remain unchanged

---

## Appendix A: Glossary

| Term | Definition |
|------|------------|
| **BlockDAG** | Directed acyclic graph of blocks |
| **DLC** | Deterministic Learning Consensus |
| **D-GBDT** | Deterministic Gradient-Boosted Decision Trees |
| **HashTimer** | Cryptographic temporal anchor |
| **IPN** | IPPAN native token |
| **µIPN** | Micro-IPN (1 IPN = 1,000,000 µIPN) |
| **Primary Verifier** | Validator proposing blocks for a round |
| **Shadow Verifier** | Validator validating primary's work |
| **Roundchain** | Linearized sequence of rounds |

---

## Appendix B: References

1. Castro, M., & Liskov, B. (1999). Practical Byzantine fault tolerance. *OSDI*.
2. Yin, M., et al. (2019). HotStuff: BFT consensus in the lens of blockchain. *PODC*.
3. Kwon, J., & Buchman, E. (2016). Tendermint: Consensus without mining.
4. Friedman, J. H. (2001). Greedy function approximation: A gradient boosting machine. *Annals of Statistics*.
5. Sompolinsky, Y., & Zohar, A. (2015). Secure high-rate transaction processing in Bitcoin. *FC*.

---

## Appendix C: Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0-rc1 | 2025-11-24 | Initial release candidate |

---

**Document Status:** RELEASE CANDIDATE  
**Next Review:** Post-external-audit

**Maintainers:**  
- Ugo Giuliani (Lead Architect)
- Desirée Verga (Strategic Product Lead)
- Kambei Sapote (Network Engineer)
