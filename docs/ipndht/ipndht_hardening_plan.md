# IPNDHT Hardening Plan

**Version:** 1.0  
**Date:** 2025-11-15  
**Status:** Planning Phase  
**Owner:** Agent-Gamma (Network Engineering)

---

## Executive Summary

This document provides a comprehensive analysis of the current IPNDHT (IPPAN DHT) implementation and proposes a phased hardening plan to achieve the network resilience, @handle lookup, and file/hash listing capabilities required by the IPPAN whitepaper.

**Current State:** Basic libp2p Kademlia DHT infrastructure is present but largely **unused for IPNDHT-specific features**. Node discovery works via mDNS and bootstrap peers. @handle resolution exists but operates entirely in L2 (in-memory registry) without DHT integration. **No file/hash listing via DHT is implemented**.

**Goal:** Transform the existing P2P infrastructure into a robust IPNDHT system supporting:
- Resilient node discovery (cold-start, extreme conditions)
- Distributed @handle lookup via DHT
- File/hash advertisement and listing (HashTimer-anchored content)
- Minimum 2 IPNWorker operational requirement

---

## 1. Current State Analysis

### 1.1 Crates Involved

| Crate | Purpose | Key Modules | Status |
|-------|---------|-------------|--------|
| **`ippan-p2p`** | P2P networking stack | `libp2p_network.rs`, `lib.rs` (HTTP fallback) | ✅ Working |
| **`ippan-network`** | Higher-level network primitives | `discovery.rs`, `peers.rs`, `reputation.rs` | ✅ Working |
| **`ippan-l2-handle-registry`** | L2 @handle storage | `registry.rs`, `resolution.rs` | ✅ Working (L2 only) |
| **`ippan-validator-resolution`** | ValidatorId resolver | `resolver.rs` | ✅ Working (L2 only) |
| **`ippan-types`** | Core data types | `Block`, `Transaction`, `HashTimer` | ✅ Working |

### 1.2 Current Flows

#### 1.2.1 Node Discovery
**How it works:**
- **libp2p Kademlia DHT** is initialized in `libp2p_network.rs:183-186`
- **mDNS** discovers local peers automatically (`enable_mdns: true`)
- **Bootstrap peers** dialed on startup from config
- **Identify protocol** exchanges peer information automatically
- **Relay + DCUtR** handle NAT traversal for connectivity

**Implementation:**

```rust:145:186:crates/p2p/src/libp2p_network.rs
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
```

**Status:** ✅ **Implemented and wired** – Kademlia DHT is active, routing table is populated

**Gaps:**
- ❌ No DHT record storage/retrieval APIs exposed
- ❌ No provider records for content announcement
- ❌ No explicit bootstrap validation strategy
- ❌ No cold-start recovery beyond bootstrap list

#### 1.2.2 @Handle Resolution
**How it works:**
- `ValidatorId` can be a public key, @handle, or alias
- L2HandleRegistry stores `Handle → PublicKey` mappings **in-memory only**
- Signature-verified registration, updates, and transfers
- No DHT integration – all lookups are local or via API calls

**Implementation:**

```rust:168:188:crates/l2_handle_registry/src/registry.rs
    pub fn resolve(&self, handle: &Handle) -> Result<PublicKey> {
        let handles = self.handles.read();
        if let Some(meta) = handles.get(handle) {
            if meta.expires_at > 0
                && meta.expires_at
                    < SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
            {
                return Err(HandleRegistryError::HandleExpired {
                    handle: handle.as_str().to_string(),
                });
            }
            Ok(meta.owner.clone())
        } else {
            Err(HandleRegistryError::HandleNotFound {
                handle: handle.as_str().to_string(),
            })
        }
    }
```

**Status:** ⚠️ **Partially implemented** – Works in L2 (local memory), not distributed

**Gaps:**
- ❌ No DHT `PUT_VALUE` for handle registration
- ❌ No DHT `GET_VALUE` for remote handle lookup
- ❌ No peer synchronization mechanism for handle registry
- ❌ No distributed ownership verification beyond local signatures

#### 1.2.3 File/Hash Listing
**How it works:**
- **Currently NOT implemented at all**
- Whitepaper mentions: "Distributed Hash Table (DHT) for file storage + HashTimer metadata"
- No APIs to:
  - Publish a file hash to DHT
  - Query DHT for available hashes
  - List files by HashTimer ID
  - Advertise provider records

**Implementation:**
- ❌ **Missing entirely**

**Status:** 🚫 **Not implemented**

**Gaps:**
- ❌ No provider record API (Kademlia `start_providing`)
- ❌ No content hash storage in DHT
- ❌ No HashTimer-to-content mapping
- ❌ No file availability query mechanism

### 1.3 Kademlia DHT Current Usage

The libp2p Kademlia DHT is **initialized but minimally used**:

```rust:183:186:crates/p2p/src/libp2p_network.rs
        let store = kad::store::MemoryStore::new(peer_id);
        let mut kad_cfg = kad::Config::default();
        kad_cfg.set_query_timeout(Duration::from_secs(5));
        let kademlia = kad::Behaviour::with_config(peer_id, store, kad_cfg);
```

**What it does:**
- ✅ Maintains routing table for peer discovery
- ✅ Responds to peer lookups
- ✅ Integrated with mDNS for local discovery

**What it doesn't do:**
- ❌ Store/retrieve key-value records
- ❌ Publish provider records
- ❌ DHT-based content routing
- ❌ Custom IPNDHT record types

### 1.4 Technical Debt

**No TODO/FIXME comments found** in `crates/p2p` or `crates/network` – code is production-ready but feature-incomplete.

**Key Issues:**
1. **DHT as passive infrastructure** – Kademlia exists but isn't used for IPNDHT goals
2. **L2 registry isolation** – @handle registry has no network distribution
3. **No content addressing** – Missing file/hash storage and retrieval
4. **Bootstrap fragility** – No fallback if bootstrap peers are unreachable
5. **No minimum node enforcement** – No check for "minimum 2 IPNWorker" requirement

---

## 2. Whitepaper Goals vs. Implementation

| Goal | Whitepaper Requirement | Current Status | Gap |
|------|----------------------|----------------|-----|
| **Node Discovery** | Survive extreme conditions, minimum 2 IPNWorkers | ⚠️ Partial – works but no fallback | Cold-start recovery, health validation |
| **@handle Lookup** | DHT-based `@user.ipn` resolution | ❌ Missing – L2 in-memory only | DHT PUT/GET for handles |
| **File/Hash Listing** | Advertise hashes via DHT, HashTimer-anchored | 🚫 Not implemented | Provider records, content queries |
| **Distributed Registry** | L2 handles stored/replicated across DHT | ❌ Missing – local-only | DHT record replication |
| **Resilience** | 2+ node minimum, bootstrap fallback | ⚠️ Partial – no enforcement | Peer count validation, DNS seeds |

**Key Findings:**
- ✅ **libp2p foundation is solid** – Kademlia, mDNS, Relay all working
- ⚠️ **IPNDHT features unimplemented** – DHT used for routing only, not data
- 🚫 **File/hash listing absent** – No provider records or content addressing
- ⚠️ **@handle DHT integration missing** – Works locally, not distributed

---

## 3. Phased Hardening Roadmap

### Phase D2: Foundation Hardening (Bootstrap & Discovery)
**Goal:** Ensure nodes can reliably discover each other in all conditions

**Update (2025-11-15):** `ippan-p2p` now exposes a dedicated `DhtConfig` that decouples bootstrap peers, NAT traversal hints,
and HTTP announcement intervals from the general `P2PConfig`. Node startup feeds this struct from environment variables, and
`HttpP2PNetwork` automatically derives the announce address via explicit overrides, UPnP external IP detection, or fallbacks to
configured IP discovery services. This gives operators a single place to tweak DHT-facing behavior without mutating unrelated
network settings and provides a deterministic source of truth for future libp2p upgrades.

**Tasks:**
1. **Bootstrap configuration improvements**
   - Add DNS seed resolution (e.g., `seed.ippan.network`)
   - Support bootstrap peer lists from config/environment
   - Implement bootstrap health checks and rotation
   - Add fallback to well-known HTTP discovery endpoints

2. **Cold-start recovery**
   - Store known-good peers to disk (peer cache)
   - Load cached peers on startup if bootstrap fails
   - Implement exponential backoff for reconnection attempts

3. **Minimum node validation**
   - Add `min_peers` check before declaring "network ready"
   - Emit health warnings if peer count < 2 (IPNWorker minimum)
   - Block consensus participation until `min_peers` reached

4. **DHT bootstrap improvements**
   - Expose `bootstrap_dht()` API in `Libp2pNetwork`
   - Trigger periodic DHT refresh (every 5 minutes)
   - Log routing table statistics for observability

**Crates to modify:**
- `ippan-p2p` (`libp2p_network.rs`)
- `ippan-network` (`discovery.rs`)

**Public APIs to add:**
```rust
// In Libp2pNetwork
pub fn bootstrap_dht(&mut self) -> Result<()>;
pub fn get_routing_table_stats(&self) -> DhtStats;
pub fn get_peer_count(&self) -> usize;

// In DiscoveryService
pub fn add_dns_seed(&mut self, seed: &str) -> Result<()>;
pub fn load_peer_cache(&mut self, path: &Path) -> Result<()>;
pub fn save_peer_cache(&self, path: &Path) -> Result<()>;
```

**Success Criteria:**
- ✅ Node can start with 0 bootstrap peers (uses DNS seeds)
- ✅ Node recovers from offline state using peer cache
- ✅ Minimum 2-peer requirement enforced and logged
- ✅ DHT routing table reaches 20+ peers within 60 seconds

---

### Phase D3: @Handle DHT Integration
**Goal:** Distribute @handle registry across IPNDHT for global lookup

**Tasks:**
1. **DHT record types for handles**
   - Define IPNDHT-specific DHT record format:
     ```rust
     pub struct HandleDhtRecord {
         handle: String,          // e.g., "@alice.ipn"
         owner: [u8; 32],         // Public key
         expires_at: u64,         // Timestamp
         l1_anchor: [u8; 32],     // L1 ownership proof
         signature: Vec<u8>,      // Ed25519 signature
     }
     ```
   - Serialize as canonical JSON or bincode for determinism

2. **DHT PUT API for handle registration**
   - When handle is registered in L2, also publish to DHT:
     ```rust
     pub async fn publish_handle_to_dht(
         &self,
         handle: &Handle,
         record: HandleDhtRecord,
     ) -> Result<()>;
     ```
   - Use `kademlia.put_record()` with key = `sha256(handle_string)`

3. **DHT GET API for handle lookup**
   - Add remote lookup fallback in `ValidatorResolver`:
     ```rust
     pub async fn resolve_handle_via_dht(
         &self,
         handle: &str,
     ) -> Result<PublicKey>;
     ```
   - Query DHT if local L2 registry misses
   - Cache result in L2 with TTL (5 minutes)

4. **Handle replication & expiration**
   - Implement DHT record republishing (every 12 hours)
   - Prune expired handles from DHT
   - Add handle synchronization on peer connect

5. **Transaction integration**
   - Add `HandleRegistrationTx` type to blockchain
   - Include L1 anchor hash in transaction for auditability
   - Emit `HandleRegistered` event for DHT indexing

**Crates to modify:**
- `ippan-p2p` (add DHT record APIs)
- `ippan-l2-handle-registry` (integrate DHT calls)
- `ippan-validator-resolution` (fallback to DHT)
- `ippan-types` (add `HandleRegistrationTx`)

**Public APIs to add:**
```rust
// In Libp2pNetwork
pub async fn put_dht_record(&mut self, key: &[u8], value: Vec<u8>) -> Result<()>;
pub async fn get_dht_record(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>>;

// In L2HandleRegistry
pub async fn publish_to_dht(&self, p2p: &Libp2pNetwork) -> Result<()>;

// In ValidatorResolver
pub async fn resolve_with_dht_fallback(&self, id: &ValidatorId) -> Result<ResolvedValidator>;
```

**Success Criteria:**
- ✅ Handle registered on node A is resolvable from node B via DHT
- ✅ DHT lookup completes in < 500ms (99th percentile)
- ✅ Expired handles automatically removed from DHT
- ✅ @handle resolution works with 2+ nodes (no central registry)

---

### Phase D4: File/Hash Advertisement & Listing
**Goal:** Enable content addressing via DHT (HashTimer-anchored file storage)

**Tasks:**
1. **Define content record types**
   ```rust
   pub struct ContentDhtRecord {
       content_hash: [u8; 32],      // SHA-256 of file content
       hashtimer_id: [u8; 32],      // HashTimer anchor
       size_bytes: u64,             // File size
       providers: Vec<PeerId>,      // Nodes hosting this file
       metadata: HashMap<String, String>, // MIME type, name, etc.
       timestamp: u64,              // Upload time
   }
   ```

2. **Provider record APIs**
   - Expose `start_providing()` wrapper:
     ```rust
     pub async fn advertise_content(
         &mut self,
         content_hash: [u8; 32],
         hashtimer: HashTimer,
     ) -> Result<()>;
     ```
   - Use Kademlia provider records for peer-to-peer file routing

3. **Content query API**
   - Find providers for a given hash:
     ```rust
     pub async fn find_content_providers(
         &mut self,
         content_hash: [u8; 32],
     ) -> Result<Vec<PeerId>>;
     ```
   - Return list of peers advertising the file

4. **HashTimer-based content indexing**
   - Add `list_content_by_hashtimer()` API
   - Store HashTimer → content mappings in DHT
   - Enable time-based content discovery (e.g., "files from Round 12345")

5. **Content availability tracking**
   - Periodic provider record refresh (every 6 hours)
   - Prune stale provider records (> 24 hours old)
   - Add content replication logic (min 3 providers per file)

6. **RPC endpoints for file operations**
   ```rust
   POST /api/v1/content/publish    // Advertise content to DHT
   GET  /api/v1/content/{hash}     // Lookup providers
   GET  /api/v1/content?hashtimer={id} // List by HashTimer
   ```

**Crates to modify:**
- `ippan-p2p` (provider record APIs)
- `ippan-rpc` (HTTP endpoints for content ops)
- `ippan-types` (content record types)

**Public APIs to add:**
```rust
// In Libp2pNetwork
pub async fn start_providing(&mut self, key: &[u8]) -> Result<()>;
pub async fn find_providers(&mut self, key: &[u8]) -> Result<HashSet<PeerId>>;
pub async fn stop_providing(&mut self, key: &[u8]) -> Result<()>;

// In ippan-rpc AppState
pub async fn publish_content(&self, hash: [u8; 32], hashtimer: HashTimer) -> Result<()>;
pub async fn list_content(&self, hashtimer_id: Option<[u8; 32]>) -> Result<Vec<ContentRecord>>;
```

**Success Criteria:**
- ✅ File hash published on node A is discoverable from node B
- ✅ Provider records include 3+ peers for redundancy
- ✅ Content listing by HashTimer returns correct files
- ✅ Stale providers pruned after 24 hours

---

### Phase D5: Resilience & Testing
**Goal:** Validate IPNDHT under extreme conditions and multi-node scenarios

**Tasks:**
1. **Multi-node integration tests**
   - Spawn 5-node local network with mDNS disabled (force DHT)
   - Test @handle registration + lookup across all nodes
   - Test file advertisement + provider discovery
   - Verify cold-start recovery with peer cache

2. **Extreme condition scenarios**
   - **Test: All bootstrap peers offline**
     - Expected: Nodes use DNS seeds or peer cache
   - **Test: 1 of 2 nodes crashes**
     - Expected: Remaining node stays operational
   - **Test: Network partition (3 nodes split from 2)**
     - Expected: Both partitions continue independently
   - **Test: High churn (50% nodes restart every 30s)**
     - Expected: DHT remains coherent, handle lookups succeed

3. **Performance benchmarks**
   - @handle lookup latency (target: < 500ms p99)
   - Content provider query latency (target: < 1s p99)
   - DHT record replication time (target: < 10s)
   - Bootstrap time (target: < 30s from cold start)

4. **Observability & metrics**
   - Export Prometheus metrics:
     - `ipndht_peer_count`
     - `ipndht_routing_table_size`
     - `ipndht_handle_lookups_total`
     - `ipndht_content_queries_total`
     - `ipndht_bootstrap_failures_total`
   - Add structured logging for DHT events

5. **Fallback mechanisms**
   - HTTP-based handle registry API as fallback
   - Centralized handle index for cold-start bootstrap
   - Peer exchange protocol for rapid topology recovery

**Crates to modify:**
- `ippan-p2p` (metrics, logging)
- `ippan-network` (health checks)
- New: `ippan-p2p/tests/integration/` (multi-node tests)

**Public APIs to add:**
```rust
// In Libp2pNetwork
pub fn get_metrics(&self) -> DhtMetrics;
pub fn is_dht_ready(&self) -> bool;
```

**Success Criteria:**
- ✅ All multi-node tests pass with 5 nodes
- ✅ Network survives 1 node crash without degradation
- ✅ DHT lookups succeed 99.9% of the time
- ✅ Bootstrap completes in < 30s even with cold start
- ✅ Metrics accurately reflect DHT health

---

## 4. Implementation Timeline

| Phase | Duration | Dependencies | Deliverables |
|-------|----------|--------------|--------------|
| **D2: Foundation** | 1 week | None | DNS seeds, peer cache, min peer check |
| **D3: @Handle DHT** | 2 weeks | D2 complete | DHT PUT/GET for handles, L1 anchors |
| **D4: File/Hash** | 2 weeks | D3 complete | Provider records, content queries |
| **D5: Testing** | 1 week | D2-D4 complete | Integration tests, metrics, benchmarks |

**Total:** ~6 weeks

---

## 5. Technical Specifications

### 5.1 DHT Record Key Format

| Record Type | Key Format | Example |
|-------------|------------|---------|
| **@handle** | `sha256("ipndht:handle:" + handle_string)` | `sha256("ipndht:handle:@alice.ipn")` |
| **Content** | `sha256("ipndht:content:" + hex(content_hash))` | `sha256("ipndht:content:abc123...")` |
| **HashTimer Index** | `sha256("ipndht:hashtimer:" + hex(hashtimer_id))` | `sha256("ipndht:hashtimer:deadbeef...")` |

### 5.2 DHT Record Value Format

**Handle Record (JSON):**
```json
{
  "version": 1,
  "handle": "@alice.ipn",
  "owner": "0123456789abcdef...",
  "expires_at": 1735689600,
  "l1_anchor": "fedcba9876543210...",
  "signature": "base64-encoded-sig",
  "updated_at": 1704067200
}
```

**Content Record (JSON):**
```json
{
  "version": 1,
  "content_hash": "sha256-hash-hex",
  "hashtimer_id": "hashtimer-hex",
  "size_bytes": 1048576,
  "mime_type": "application/octet-stream",
  "filename": "example.bin",
  "timestamp": 1704067200
}
```

### 5.3 Configuration Parameters

```toml
[ipndht]
enabled = true
min_peers = 2                    # Minimum peers before declaring "ready"
bootstrap_peers = [
    "/ip4/bootstrap1.ippan.network/tcp/9000/p2p/12D3KooW...",
    "/ip4/bootstrap2.ippan.network/tcp/9000/p2p/12D3KooX...",
]
dns_seeds = [
    "seed1.ippan.network",
    "seed2.ippan.network",
]
peer_cache_path = "./data/peer_cache.json"
dht_refresh_interval_secs = 300  # DHT bootstrap every 5 min
handle_ttl_secs = 300            # L2 cache TTL for remote handles
content_provider_ttl_secs = 21600 # Provider record TTL (6 hours)
max_concurrent_dht_queries = 10
query_timeout_ms = 5000
```

---

## 6. Open Questions & Decisions Needed

1. **@handle ownership on L1 vs L2:**
   - Should L1 store full handle mappings or just anchors?
   - **Recommendation:** Keep L1 minimal (anchors only), DHT for distribution

2. **DHT record signing:**
   - Should all DHT records be signed by the publisher?
   - **Recommendation:** Yes, use Ed25519 signatures for tamper-proof records

3. **File storage vs. DHT metadata:**
   - Does IPNDHT store actual file content or just metadata/pointers?
   - **Recommendation:** DHT stores metadata only; files stored in separate DA layer or local storage

4. **DNS seed authority:**
   - Who controls `seed.ippan.network` DNS entries?
   - **Recommendation:** Foundation-operated with community fallbacks

5. **Kademlia vs. custom DHT:**
   - Should we extend Kademlia or implement custom IPNDHT protocol?
   - **Recommendation:** Extend Kademlia (proven, battle-tested)

---

## 7. Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **DHT spam/pollution** | High | Medium | Rate-limit PUT operations, require small fee |
| **Stale handle records** | Medium | High | Implement TTL + periodic republishing |
| **Bootstrap centralization** | High | Medium | DNS seeds + peer cache + HTTP fallback |
| **DHT partitioning** | High | Low | Monitor partition events, add mesh healing |
| **Provider record churn** | Medium | Medium | Increase TTL, add replication redundancy |

---

## 8. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Min operational nodes** | 2+ | Enforced by code |
| **@handle lookup success rate** | > 99.9% | Prometheus counter |
| **DHT lookup latency (p99)** | < 500ms | Histogram |
| **Bootstrap success rate** | > 99% | Failure counter |
| **Provider discovery time** | < 1s | Timer histogram |
| **Cold-start recovery** | < 30s | Integration test |

---

## 9. References

- **Kademlia Paper:** Maymounkov & Mazières, 2002
- **libp2p Specs:** https://github.com/libp2p/specs
- **IPPAN Whitepaper:** `docs/prd/ippan-prd-2025.md`
- **L2 Handle System:** `docs/L2_HANDLE_SYSTEM.md`
- **Architecture Doc:** `docs/IPPAN_Architecture_Update_v1.0.md`

---

## 10. Conclusion

The IPPAN network currently has **solid P2P foundations** (libp2p, Kademlia, mDNS) but **lacks IPNDHT-specific features** for distributed @handle lookup and file/hash listing. 

This plan proposes a **pragmatic 4-phase approach** to:
1. **Harden bootstrap** and discovery resilience (Phase D2)
2. **Distribute @handle registry** via DHT (Phase D3)
3. **Enable file/hash advertisement** with HashTimer anchoring (Phase D4)
4. **Validate under extreme conditions** with comprehensive testing (Phase D5)

**Key Principle:** Leverage existing libp2p infrastructure rather than reinventing DHT logic. Extend Kademlia with IPNDHT-specific record types while maintaining compatibility with broader libp2p ecosystem.

**Next Steps:**
1. Review and approve this plan with maintainers
2. Create Phase D2 implementation tasks
3. Assign to Agent-Gamma (P2P/Network owner)
4. Target completion: Q1 2026

---

**Prepared by:** Cursor Agent (Background Agent Mode)  
**Reviewed by:** (Pending)  
**Approved by:** (Pending)
