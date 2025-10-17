IPPAN Architecture Update — Deterministic Time & Global BlockDAG Synchronization
Version: v1.0
Date: October 2025
Scope: crates/time + crates/core modules
Status: Implemented in codebase

1 Overview
This update establishes the deterministic temporal and network backbone of the IPPAN blockchain.
It introduces:
✅ Corrected IPPAN Time service (median-based, monotonic)


✅ Signed HashTimer primitive (cryptographic timestamp proofs)


✅ Block & BlockHeader structures with integrated HashTimer anchors


✅ Persistent BlockDAG storage (Sled-based)


✅ Deterministic ordering via HashTimer-based comparator


✅ DAG synchronization using libp2p (RequestResponse → GossipSub)


✅ Automatic peer discovery (mDNS + Kademlia global DHT)


Together these form a fully decentralized, globally ordered, and self-synchronizing BlockDAG network suitable for high-throughput financial and IoT-grade applications.

2 IPPAN Time Service (crates/time)
2.1 Problem Resolved
The original implementation incorrectly computed peer drift using a stale last_time_us, causing double counting and “future jumps.”
2.2 Fix
now_us() and ingest_sample() now always refresh last_time_us before drift calculation.
Median-filter smoothing with bounded correction (±5 ms) prevents time leaps.
2.3 Features
Feature
Description
Microsecond precision
Uses SystemTime → UNIX_EPOCH
Median drift correction
Maintains sliding window of peer offsets
Bounded Δt
± 5 ms per update
Thread-safe
Static Lazy<Mutex<...>> state
Monotonic
Never decreases, even under clock skew

ippan_time::now_us();          // deterministic timestamp
ippan_time::ingest_sample(...); // median correction


3 HashTimer Module
3.1 Purpose
Creates verifiable timestamps that serve as atomic “moments of existence” across the network.
3.2 Structure
struct HashTimer {
    timestamp_us: i64,
    entropy: [u8; 32],
    signature: Option<Signature>,
    public_key: Option<PublicKey>,
}

3.3 Functions
sign_hashtimer(&Keypair) → signed HashTimer


verify_hashtimer(&HashTimer) → bool


generate_entropy() → random [32]


hash() → SHA-256 identifier


3.4 Use Cases
Embedded in every block header


Deterministic ordering across concurrent blocks


Basis for audit and compliance timestamping



4 Block & Header (crates/core/block.rs)
4.1 New Data Structures
pub struct BlockHeader {
    version: u8,
    creator: PublicKey,
    parent_hashes: Vec<[u8; 32]>,
    hash_timer: HashTimer,
    median_time_us: i64,
    merkle_root: [u8; 32],
    signature: Signature,
}

pub struct Block {
    header: BlockHeader,
    transactions: Vec<Vec<u8>>,
}

4.2 Capabilities
Feature
Description
Multiple parents
DAG topology support
Signed HashTimer
Deterministic time anchor
Median Time Snapshot
Embeds current IPPAN Time
Self-verification
Checks HashTimer + block signature
Merkle root
Ensures TX integrity

4.3 Example
let block = Block::new(&keypair, parent_hashes, txs);
assert!(block.verify());


5 Deterministic Ordering (order.rs)
Defines a total order over DAG blocks:
fn order_blocks(a, b) =
    a.hash_timer.timestamp_us.cmp(&b.hash_timer.timestamp_us)
      .then_with(|| a.hash_timer.hash().cmp(&b.hash_timer.hash()));

Ensures all nodes reach identical ordering independent of reception sequence.

6 Persistent BlockDAG (dag.rs)
6.1 Storage
Uses Sled embedded DB for durable local state.
6.2 Core Functions
Method
Purpose
open(path)
Initialize DB and rebuild tips
insert_block(block)
Verify → store → update tips
get_block(hash)
Retrieve by key
get_tips()
Return unreferenced heads
topological_order()
BFS + deterministic sort
prune(retain)
Compact old generations

6.3 Behaviour
Automatically rebuilds tips after crash


Deterministic traversal for audits and consensus



7 Network Synchronization
7.1 Phase 1 – Request/Response Sync
Nodes exchanged blocks via libp2p RequestResponse.


Periodic tip advertisement every 6 s.


7.2 Phase 2 – Gossip Fan-out
Introduced Gossipsub for scalable one-to-many propagation.


Added mDNS for automatic LAN discovery.


Each node publishes:


GossipMsg::Tip(hash)


GossipMsg::Block(block)


7.3 Phase 3 – Global Kademlia Discovery
Integrated libp2p Kademlia for cross-network peer routing.


Each node:


Publishes its presence record.


Learns and connects to new peers automatically.


Combined behaviour:
 DagBehaviour = { gossip + mdns + kademlia }.



8 System Architecture (Current Stack)
┌───────────────────────────────┐
│  Application Layer            │
│  • Transaction Pool           │
│  • Consensus (FBA/FastBFT)    │
├───────────────────────────────┤
│  DAG Layer (crates/core)      │
│  • Block / Header             │
│  • BlockDAG (Sled)            │
│  • Deterministic Ordering     │
│  • DAG Sync (Gossip + KAD)    │
├───────────────────────────────┤
│  Time Layer (crates/time)     │
│  • IPPAN Time Service         │
│  • Peer Median Drift Filter   │
│  • Signed HashTimer           │
│  • HashTimer Emission Loop    │
├───────────────────────────────┤
│  Networking (libp2p)          │
│  • TCP / Noise / Yamux        │
│  • Gossipsub Broadcast        │
│  • mDNS + Kademlia Discovery  │
└───────────────────────────────┘


9 Security & Determinism Guarantees
Layer
Guarantee
IPPAN Time
Median-bounded, monotonic, drift ≤ ±5 ms
HashTimer
Ed25519-signed, entropy-unique, tamper-proof
Block
Dual signature (timer + header)
DAG Order
Deterministic across all nodes
Network
Authenticated (Noise), Sybil-resistant once key registry enforced


10 Next Milestones (v1.1+)
Area
Planned Enhancement
Bootstrap Peers
Static seed list for global Kademlia entrypoints
Round Topics
Per-round Gossipsub channels for scalability
Block Compression
HashTimer + diff broadcast instead of full payloads
Security
Signed gossip envelopes, node reputation
Consensus Layer
Integrate FBA / Roundchain validator scheduling


11 Conclusion
The current stack provides:
Deterministic, cryptographically verifiable time across nodes


Fully decentralized BlockDAG with no central coordinator


Automatic local + global peer discovery


Scalable, self-healing data propagation


This transforms IPPAN from a high-speed prototype into a production-grade, globally coherent, and self-organizing Layer-1 BlockDAG network — ready for the upcoming FBA consensus and smart-contract layer integration.

Maintainers: IPPAN Core Team
Authors: Hugh Vega / Désirée Verga / Contributors
Repository: dmrl789/IPPAN


