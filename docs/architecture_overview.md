# IPPAN Architecture Overview
**System Design and Component Interactions**

**Version:** v1.0.0-rc1  
**Date:** 2025-11-24

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       IPPAN Node                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────┐     ┌──────────┐     ┌──────────────┐       │
│  │  RPC/API │────▶│ Mempool  │────▶│  Consensus   │       │
│  │  (Axum)  │     │          │     │   Engine     │       │
│  └──────────┘     └──────────┘     └──────┬───────┘       │
│       │                                    │               │
│       │                                    ▼               │
│       │                         ┌─────────────────┐       │
│       │                         │  Block DAG      │       │
│       │                         │  + DLC Engine   │       │
│       │                         └────────┬────────┘       │
│       │                                  │                │
│       │                                  ▼                │
│       │                         ┌─────────────────┐       │
│       └────────────────────────▶│   Storage       │       │
│                                 │   (Sled/Mem)    │       │
│                                 └─────────────────┘       │
│                                                             │
│  ┌──────────────┐         ┌─────────────┐                 │
│  │  HashTimer   │◀───────▶│  D-GBDT AI  │                 │
│  │  (Time Sync) │         │  (Fairness) │                 │
│  └──────────────┘         └─────────────┘                 │
│                                                             │
│  ┌──────────────────────────────────────────┐             │
│  │           P2P Network (libp2p)           │             │
│  │  ┌────────┐  ┌──────┐  ┌──────────────┐ │             │
│  │  │ Gossip │  │ DHT  │  │  Discovery   │ │             │
│  │  └────────┘  └──────┘  └──────────────┘ │             │
│  └──────────────────────────────────────────┘             │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. RPC/API Layer (`crates/rpc`)

**Purpose:** HTTP/WebSocket API for external clients

**Technology:** Axum (async Rust web framework)

**Endpoints:**
- `/health` - Node health status
- `/metrics` - Prometheus metrics
- `/tx/payment` - Submit payment transaction
- `/tx/handle` - Register handle
- `/account/:addr/payments` - Query payment history
- `/files/*` - File storage/retrieval
- `/ai/status` - D-GBDT model status

**Security:**
- Rate limiting (per-IP, per-endpoint)
- Request body size limits (default 1MB)
- IP whitelist/blacklist
- Dev-mode gating for sensitive endpoints

**Flow:**
1. Client sends HTTP request
2. RPC handler validates input
3. Transaction added to mempool
4. Response returned (tx ID, status)

---

### 2. Mempool (`crates/mempool` logic in consensus)

**Purpose:** Buffer for pending transactions before consensus

**Operations:**
- `add(tx)` - Add transaction (with validation)
- `remove(tx_id)` - Remove after inclusion in block
- `get_pending()` - Retrieve transactions for block proposal

**Validation:**
- Fee caps enforced (per transaction type)
- Nonce checking (prevent replay)
- Balance verification (sender has funds)
- Signature validation

---

### 3. Consensus Engine (`crates/consensus`)

**Purpose:** Core consensus logic, emission tracking, validator management

**Key Modules:**
- `emission_tracker.rs` - DAG-Fair emission, reward distribution
- `round_executor.rs` - Execute consensus rounds
- `payments.rs` - Payment transaction processing
- `handles.rs` - Handle registration logic
- `bonding.rs` - Validator bonding and slashing

**Consensus Flow:**
1. **Round Start:** HashTimer determines current round
2. **Verifier Selection:** DLC selects primary + shadow verifiers
3. **Block Proposal:** Primary proposes block from mempool
4. **Verification:** Shadow verifiers validate
5. **DAG Insertion:** Block added to DAG
6. **Finalization:** Blocks beyond depth=2 finalized
7. **State Update:** Storage updated with finalized blocks
8. **Emission:** Rewards distributed to validators

---

### 4. Block DAG & DLC (`crates/consensus_dlc`)

**Purpose:** Parallel block production with deterministic ordering

**DAG Properties:**
- Blocks can have multiple parents (parallel proposals)
- Fork-choice: height → HashTimer → D-GBDT weights → ID tie-breaker
- Finalization lag: 2 rounds
- Max reorg depth: 2 blocks

**DLC (Deterministic Learning Consensus):**
- **Primary verifier:** Proposes blocks
- **Shadow verifiers:** Validate primary's work
- **D-GBDT scoring:** AI model assigns fairness scores
- **Selection:** Deterministic based on round seed + scores

**Fork Resolution:**
```rust
fn select_canonical_tip(
    tips: &[Block],
    validator_weights: &HashMap<String, i64>,
    shadow_flags: &HashSet<String>,
) -> String {
    tips.sort_by(|a, b| {
        b.height.cmp(&a.height)
            .then_with(|| a.timestamp.cmp(&b.timestamp))  // HashTimer
            .then_with(|| b.weight.cmp(&a.weight))        // D-GBDT
            .then_with(|| a.id.cmp(&b.id))                // Deterministic tie-break
    });
    tips[0].id.clone()
}
```

---

### 5. D-GBDT AI Engine (`crates/ai_core`, `crates/ai_registry`)

**Purpose:** Deterministic validator scoring for fairness

**Model Properties:**
- **No floating-point:** All arithmetic is integer-based (SCALE = 1,000,000)
- **Fixed-point trees:** Thresholds and leaf values are scaled integers
- **Canonical JSON:** Sorted keys for reproducible hashing
- **BLAKE3 hashing:** Fast, deterministic model verification

**Inference Flow:**
1. **Extract features:** uptime, latency, honesty, stake, etc. (from telemetry)
2. **Traverse trees:** Integer comparisons only (`feature <= threshold`)
3. **Accumulate scores:** Sum leaf values across all trees
4. **Apply bias/scale:** Final score = `(sum + bias) * post_scale / SCALE`
5. **Return score:** Used for verifier selection

**Example:**
```rust
let features = vec![
    uptime * SCALE / 100,      // 95% → 95_000_000
    latency * SCALE,           // 50ms → 50_000_000
];
let score = model.score(&features);  // Returns i64 score
```

---

### 6. HashTimer (`crates/time`)

**Purpose:** Deterministic time synchronization across nodes

**How It Works:**
1. Each round has a target timestamp
2. Nodes hash `(round_index, previous_hash)` to generate round hash
3. HashTimer orders events by:
   - Round number (primary)
   - Hash comparison (secondary)
   - Timestamp (tertiary, with skew tolerance)

**Properties:**
- Deterministic: Same inputs → same ordering
- Fork-choice compatible: Resolves ties without clock dependence
- Skew tolerance: Rejects outliers beyond threshold (±500ms)

**Usage:**
```rust
let round_time = HashTimer::for_round(round);
let hash = round_time.hash;  // Deterministic hash
```

---

### 7. Storage (`crates/storage`)

**Purpose:** Persistent state (accounts, transactions, blocks, chain state)

**Backends:**
- **Sled:** Production (embedded key-value store)
- **Memory:** Testing/simulations (in-memory HashMap)

**Data Model:**
- **Accounts:** `{address → (balance, nonce)}`
- **Transactions:** `{tx_id → Transaction}`
- **Blocks:** `{block_id → Block}`
- **Chain state:** `{height, round, state_root, supply}`

**Operations:**
- `update_account(account)` - Update balance/nonce
- `store_transaction(tx)` - Persist transaction
- `store_block(block)` - Persist block
- `export_snapshot()` - Export full state to JSON
- `import_snapshot()` - Restore from snapshot

---

### 8. P2P Network (`crates/p2p`)

**Purpose:** Node-to-node communication via libp2p

**Protocols:**
- **Gossipsub:** Broadcast blocks, transactions, announcements
- **Kad DHT:** Peer discovery, content routing
- **Request-Response:** Sync missing blocks, query state

**Security:**
- **Message size limits:** Drop oversized messages
- **Per-peer rate limits:** Prevent spam
- **Peer scoring:** Ban misbehaving peers
- **NAT traversal:** Relay, hole-punching

**DHT Operations:**
- `publish(key, value)` - Announce file/handle
- `find(key)` - Query DHT for value
- `find_peers()` - Discover validators

---

## Data Flow: Payment Transaction

```
1. Client → RPC
   POST /tx/payment { from, to, amount, signature }

2. RPC → Mempool
   Validate: signature, balance, nonce
   Add to mempool

3. Mempool → Consensus
   Round starts, primary verifier selected

4. Consensus → DAG
   Primary proposes block with tx
   Shadow verifiers validate

5. DAG → Finalization
   Block reaches depth=2, finalized

6. Finalization → Storage
   Update accounts:
     sender.balance -= amount + fee
     sender.nonce += 1
     recipient.balance += amount
     treasury.balance += fee

7. Storage → Response
   Client queries /account/:addr/payments
   Returns updated balance + history
```

---

## Data Flow: Verifier Selection (DLC)

```
1. HashTimer → Round Start
   Round N begins at target timestamp

2. Consensus → AI Registry
   Load active D-GBDT model + hash

3. AI Registry → D-GBDT
   For each validator:
     Extract features (uptime, latency, stake)
     Run inference (integer-only)
     Return score (i64)

4. D-GBDT → Verifier Selection
   Scores: {val1: 9500, val2: 8200, val3: 7100, ...}
   Weighted random selection (deterministic seed)
   Primary: highest weighted random draw
   Shadows: next K highest draws

5. Verifier Set → Round Execution
   Primary proposes block
   Shadows validate
   Shadow disagreements flagged

6. Round End → Emission
   Distribute rewards based on participation + scores
   Update network dividend pool
```

---

## Key Invariants

### Consensus Invariants
1. **No conflicting finalized blocks:** Once finalized, never changes
2. **Deterministic fork-choice:** All nodes converge to same canonical tip
3. **Supply cap enforcement:** Total emission ≤ 21M IPN
4. **Reward accounting:** Distributed rewards ≤ emitted supply

### DAG Invariants
1. **Acyclicity:** No cycles (blocks reference only older blocks)
2. **Finalization order:** Finalized blocks form a total order
3. **Reorg depth limit:** Max 2-block reorganizations

### AI Invariants
1. **No floats:** All inference uses integer arithmetic
2. **Deterministic scores:** Same features → same score
3. **Model hash stability:** BLAKE3 hash verifies model integrity

### Storage Invariants
1. **Balance conservation:** Total supply = sum of all balances
2. **Nonce monotonicity:** Account nonces never decrease
3. **Snapshot consistency:** Export → import preserves state

---

## Security Model

### Threat Model

**Assumptions:**
- <33% adversarial validators (BFT threshold)
- <50% adversarial primary selections (shadow verifier protection)
- Network partitions heal within finite time

**Protections:**
- **Double-spend:** Prevented by nonce + finality
- **Double-signing:** Detected and slashed (50% bond)
- **Equivocation:** Shadow verifiers flag conflicts
- **Sybil attack:** Bonding requirement (10 IPN min)
- **Eclipse attack:** Multiple bootstrap peers, DHT routing
- **DDoS:** Rate limiting, message size caps, peer banning

---

## Performance Characteristics

| Metric | Value |
|--------|-------|
| **Round duration** | ~200ms (target) |
| **Block finality** | 2 rounds (~400ms) |
| **Max reorg depth** | 2 blocks |
| **TPS (theoretical)** | 1000+ (depends on mempool) |
| **Validator count** | 100+ (DLC scales linearly) |
| **D-GBDT inference** | <1ms per validator |
| **Storage (Sled)** | ~100 MB/day (varies by activity) |

---

## Summary

IPPAN uses a **layered architecture** with:
- **RPC/API** for external interaction
- **Mempool** for transaction buffering
- **Consensus** for round execution and emission
- **Block DAG + DLC** for parallel block production and deterministic ordering
- **D-GBDT AI** for fair validator selection (no floats)
- **HashTimer** for time synchronization
- **Storage (Sled)** for persistence
- **P2P (libp2p)** for networking

**Key Design Principles:**
1. **Determinism:** No floats, canonical serialization, seeded RNG
2. **Security:** Rate limiting, slashing, shadow verifiers
3. **Scalability:** DAG parallelism, efficient storage
4. **Observability:** Prometheus metrics, health endpoints

---

**For Detailed Specs:**
- `docs/FEES_AND_EMISSION.md` - Economics
- `docs/dev_guide.md` - Development workflow
- `CHECKLIST_AUDIT_MAIN.md` - Feature status
- `ACT_DLC_SIMULATION_REPORT_2025_11_24.md` - Simulation results

**Diagrams:** See `docs/diagrams/` for visual architecture (TODO: Phase 9.3)
