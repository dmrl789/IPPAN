# 📜 IPPAN — Product Requirements Document (PRD)

---

## 1️⃣ Overview

**Product Name:** IPPAN (Immutable Proof & Availability Network)  
**Type:** Global Layer-1 blockchain with 1-10 million TPS capacity and built-in global DHT storage.  
**Core Idea:**  
- **Global L1 blockchain** designed for planetary-scale adoption
- **1-10 million TPS** throughput target for mass adoption
- **Distributed Ledger:** Every node maintains a complete copy of the blockchain
- **Node Communication:** Nodes exchange transaction details and block information
- Trustless timestamping (HashTimers + IPPAN Time)  
- Deterministic ordering of transactions  
- Immutable proof-of-existence for any data  
- Encrypted, sharded storage  
- Permissionless nodes with staking  
- M2M payments in tiny units  
- Fully automated, keyless Global Fund for incentives

**Implementation:**  
One unstoppable node software: **IPPANCore**, written in Rust. No external pinning or third-party services needed.

---

## 2️⃣ Vision & Purpose

IPPAN is a **global Layer-1 blockchain** designed for planetary-scale adoption with **1-10 million TPS** capacity:

### 🚀 Global L1 Positioning
- **Primary Goal:** Become the world's fastest and most scalable L1 blockchain
- **Target:** 1-10 million transactions per second for mass global adoption
- **Use Cases:** IoT networks, AI agents, global payments, data storage, timestamping
- **Vision:** Power the next generation of decentralized applications and services

### 🎯 Production-Ready Protocol
IPPAN is now a fully functional, production-ready protocol for:
- Proving when any data existed, with **tenth-of-a-microsecond precision**
- Keeping data available through trustless, incentivized storage
- Enabling direct, unstoppable M2M payments and AI services
- Running independently, even in catastrophic scenarios — only **IPPANCore** is needed to restart

---

## 3️⃣ Core Principles & Design

### ✅ 3.1 HashTimers & IPPAN Time
- Every block, transaction, and file anchor includes a **HashTimer**
- HashTimers embed **IPPAN Time**, calculated as the median time of node clocks (atomic clocks, GPS recommended)
- Precision: **0.1 microsecond**

### ✅ 3.2 Distributed Ledger & BlockDAG Consensus

#### 🌐 Distributed Ledger Architecture

* **Complete Blockchain Copy**: Every node maintains a full copy of the entire blockchain
* **Node Communication**: Nodes actively exchange transaction details and block information
* **Consensus Propagation**: All nodes participate in consensus and validate transactions
* **Data Synchronization**: Continuous synchronization ensures all nodes have the latest state
* **Fault Tolerance**: Network remains operational even if some nodes go offline

#### 📐 BlockDAG Structure

* **Model**: Directed Acyclic Graph (DAG)
* **Function**: Enables concurrent block creation across nodes (parallelism)
* **Local Chains**: Each node maintains its own chain of blocks
* **Global Agreement**: Achieved via cryptographically anchored Rounds

#### 🔐 zk-STARK-Backed Rounds

* **Purpose**: Finalize multiple DAG branches with deterministic proof
* **Round Duration**: 100 ms to 500 ms (adaptive, depending on load)
* **Proof Type**: zk-STARK
* **Proof Size**: ~50–100 KB per round
* **Proof Generation Time**: ~0.5–2 seconds (parallelizable)
* **Verification Time**: 10–50 ms
* **Latency Tolerance**: Finality sustained under NY ↔ Tokyo RTT (~180 ms)

#### 📦 Block Dimensions (IPPAN Specification)

* **Max Block Size**: 32 KB (lightweight for rapid propagation)
* **Typical Block Size**: 4–32 KB
* **Max Transactions per Block**: ~500–2,000 (depends on tx size)
* **Block Header Size**: ~128 bytes
* **Block Content**: References transaction hashes only (no inlined payload)

#### 🧾 Transaction Structure

* **Signature**: Ed25519 (64 bytes)
* **Public Key**: 32 bytes
* **Transaction Type**: 1 byte (e.g., payment, file, TXT, etc.)
* **Amount/Data Length**: 8 bytes
* **Timestamp**: 8 bytes (microsecond precision)
* **HashTimer**: 32 bytes (ordering anchor)
* **Optional Payload**: e.g., file ref, TXT string, domain, etc.

> **Total Base Size**: ~145 bytes
> **With Payload**: Typically ~200–500 bytes depending on type

#### ⏱️ Deterministic Transaction Ordering

* **Mechanism**: `HashTimer` (cryptographic time anchor)
* **Precision**: ≤ 0.1 μs
* **Usage**: Ensures transactions and blocks are globally sortable without relying on system clocks

#### 🎯 Optimized for 1-10 Million TPS

* **Parallel Processing**: BlockDAG enables concurrent transaction handling
* **Validator Selection**: Verifiable randomness prevents manipulation
* **Sub-Second Finality**: Despite intercontinental latency
* **Adaptive Rounds**: Duration adjusts based on network load

### ✅ 3.3 Staking & Node Rules
- Nodes are permissionless for the first month
- After 1 month, each node must stake **10–100 IPN**  
- Stake can be slashed for downtime, fake proofs, or malicious behavior

### ✅ 3.4 Native Token (IPN)
- **Ticker:** IPN  
- **Max Supply:** 21,000,000 IPN (Bitcoin-style, 0 at genesis, halving schedule)  
- **Subdivision:** 1 IPN = 100,000,000 satoshi-like units

### ✅ 3.5 Fees: Transactions & Domains
- **1% fee** on every transaction → goes to the Global Fund
- `@handle.ipn` domain names have annual fees for registration & renewal → also to the Global Fund

### ✅ 3.6 Keyless Global Fund
- Collects all transaction fees & domain fees
- Autonomous: **no private keys**, cannot be seized or misused
- Every week, funds are auto-distributed to nodes that:
  - Maintained uptime
  - Validated correctly (blocks & HashTimers)
  - Provided IPPAN Time with high precision (atomic/GPS)
  - Proved storage availability
  - Served real file traffic
- Leftover funds roll over — no waste, no leakage.

### ✅ 3.7 ZK-STARK Proof System
- **Round-Level Proofs:** ZK-STARK proofs generated for entire rounds
- **Proof Generation:** Proves round validity without revealing internal state
- **Fast Verification:** 10-50ms verification time for 50-100 KB proofs
- **Sub-Second Finality:** Deterministic finality despite global latency
- **Proof Binding:** ZK-STARK binds to round state, block list, and HashTimers

### ✅ 3.8 Verifiable Randomness for Validator Selection
- Validators randomly selected for block production & validation
- Selection is transparent and verifiable on-chain
- Ensures fairness while preventing centralization

### ✅ 3.9 Encrypted, Sharded Storage
- Files are AES-256 encrypted, sharded, auto-balanced across nodes
- Built-in global DHT maps which nodes hold shards
- Proof-of-Storage via Merkle trees & spot checks
- Clickable content hashes show proof-of-existence + live storage status

### ✅ 3.10 Human-Readable Domains
- Users, devices, and AI agents can register handles like `@alice.ipn` or `@bot.iot`
- Premium TLDs possible (`.m`, `.cyborg`, `.humanoid`)
- Annual fees fund the Global Fund and incentivize long-tail storage

### ✅ 3.11 Machine-to-Machine (M2M) Payments
- Micro-payments possible in smallest IPN units
- Perfect for IoT devices and autonomous AI agents
- Every M2M payment pays the 1% micro-fee to the Global Fund

### ✅ 3.12 L2 Blockchain Integration
- **L2 Settlement Layer:** IPPAN serves as the ultimate settlement layer for L2 blockchains
- **Cross-Chain Anchors:** L2s can anchor their state to IPPAN for finality
- **Data Availability:** L2s can use IPPAN's global DHT for data storage
- **Timestamping Service:** L2s can leverage IPPAN's precision timestamping
- **M2M Payments:** L2s can enable micro-payments through IPPAN's M2M channels

### ✅ 3.13 TXT Metadata for Files and Servers
- **Objective:** Enable IPPAN nodes and users to publish signed text entries for files and servers.
- **Use Cases:**
  - **Files:** Add semantic descriptions to uploaded content (e.g., PDFs, media, whitepapers).
  - **Servers:** Announce public services like API endpoints or storage hosts.
- **Features:**
  - Signed by the handle's owner.
  - Timestamped using HashTimer.
  - Discoverable in IPNDHT.
  - Optionally anchored on-chain.
- **TXT Entry Types:**
  - **FileDescription:** Summary of a file or dataset.
  - **ServerInfo:** Service availability and endpoint metadata.
  - **DNSLikeRecord:** Domain and TLS information.
  - **ProofBinding:** Declaration of handle-resource link.

### ✅ 3.14 Archive Mode with Automatic Website Sync
- **Objective:** Enable nodes to run in archive mode, retaining validated transactions and syncing them to external endpoints.
- **Features:**
  - Retain all validated transactions.
  - Periodically push summaries or full transactions to external APIs (e.g., `ippan.net`).
  - Optionally include TXT records, file manifests, and proofs.
  - Enhance transparency and robustness of the network.
- **Configuration:**
  - Archive mode toggle and sync configuration in `node_config.rs`.
  - Local archive store implemented in `tx_archive.rs`.
  - Background uploader for syncing in `sync_uploader.rs`.
  - CLI commands for managing archive mode in `cli.rs`.

### ✅ 3.15 i-Prefix Address Format
- **Ed25519-based addresses** with Base58Check encoding
- **Address format:** `i` + Base58Check(version + pubkey_hash + checksum)
- **Version byte:** 0x01 for mainnet, 0x02 for testnet
- **Security:** Cryptographic address generation and validation
- **Integration:** Full wallet and transaction system support

---

## 4️⃣ Technical Specifications & Performance

### 📊 Block & Transaction Specifications

#### **Block Structure**
```
Block Header (~128 bytes):
├── Block Hash (32 bytes)
├── Parent Hash (32 bytes)
├── Round Number (8 bytes)
├── Timestamp (8 bytes)
├── Validator ID (32 bytes)
├── Block Size (4 bytes)
├── Transaction Count (4 bytes)
├── Merkle Root (32 bytes)
├── HashTimer (32 bytes)
└── Padding (~8 bytes)

Block Body:
├── Transaction Hash References (variable)
└── ZK-STARK Proof (50-100 KB)
```

#### **Transaction Structure**
```
Transaction (145 bytes base + variable data):
├── Ed25519 Signature (64 bytes)
├── Public Key (32 bytes)
├── Transaction Type (1 byte)
├── Amount (8 bytes)
├── Timestamp (8 bytes)
├── HashTimer (32 bytes)
└── Variable Data (0-500 bytes)
```

#### **Transaction Types**
- **Payment (0x01):** Standard IPN transfer
- **Storage (0x02):** File upload/download
- **Domain (0x03):** Domain registration/renewal
- **Staking (0x04):** Stake/unstake operations
- **Anchor (0x05):** Cross-chain anchor for L2 integration
- **M2M (0x06):** Machine-to-machine payment
- **L2 Settlement (0x07):** L2 blockchain settlement transactions
- **L2 Data (0x08):** L2 data availability and storage

#### **ZK-STARK Round Structure**
```
Round (100-500 ms, adaptive):
├── Round Header (~128 bytes)
├── Block List (variable)
├── State Transition (variable)
├── ZK-STARK Proof (50-100 KB)
└── Validator Signatures (variable)
```

### 🎯 1-10 Million TPS Goal
- **Phase 1:** 1 million TPS (current target)
- **Phase 2:** 5 million TPS (optimization phase)
- **Phase 3:** 10 million TPS (global scale)

### 📈 Scaling Strategy
- **Parallel Processing:** BlockDAG enables concurrent transaction processing
- **Sharding:** Horizontal scaling across multiple shards
- **Optimized Consensus:** Minimal consensus overhead for maximum throughput
- **Network Optimization:** Efficient peer-to-peer communication
- **Storage Scaling:** Distributed storage with proof-of-storage

### 🌍 Global Infrastructure
- **Geographic Distribution:** Nodes across all continents
- **Network Optimization:** Low-latency connections between major hubs
- **Redundancy:** Multiple paths for data and transaction propagation
- **Resilience:** Survives regional outages and network partitions

---

## 5️⃣ Node Architecture — IPPANCore

```
+----------------------------------------------------------+
|                    IPPANCore Node                        |
+----------------------------------------------------------+
| 1) BlockDAG Consensus Engine (1-10M TPS optimized)       |
| 2) HashTimer Module with IPPAN Time (0.1μs precision)    |
| 3) Verifiable Randomness Selector                        |
| 4) Validator & Staking Logic (10–100 IPN)                |
| 5) AES-256 Encrypted Storage Orchestrator                |
| 6) Global DHT Router & Lookup                            |
| 7) Proof-of-Storage (Merkle Trees, Spot Checks)          |
| 8) Traffic Tracker & File Serving                        |
| 9) Domain Name System (Handles, Renewals)                |
|10) Keyless Global Reward Fund Logic                      |
|11) Local Wallet (Ed25519 keys, staking, rewards)         |
|12) M2M Payment Support                                   |
|13) Performance Monitoring & Optimization                  |
|14) Cross-Chain Bridge & Anchor System                    |
|15) Archive Mode & External Sync                          |
|16) i-Prefix Address Format Support                       |
+----------------------------------------------------------+
```

---

## 6️⃣ Implementation Status (2024)

### ✅ **COMPLETED SYSTEMS**

All major systems are **implemented and integrated**:

#### **Core Consensus Engine**
- ✅ **BlockDAG consensus** with parallel processing
- ✅ **HashTimers** with 0.1μs precision timestamping
- ✅ **IPPAN Time** median time calculation
- ✅ **ZK-STARK rounds** with sub-second finality
- ✅ **Verifiable randomness** for validator selection

#### **Economic & Staking Systems**
- ✅ **Staking system** with 10-100 IPN requirements
- ✅ **Validator selection** and rotation
- ✅ **Slashing logic** for misbehavior penalties
- ✅ **Keyless global fund** with autonomous distribution
- ✅ **Weekly reward distribution** based on performance

#### **Storage & Data Systems**
- ✅ **AES-256 encrypted storage** with derived keys
- ✅ **Sharded file storage** with auto-balancing
- ✅ **Proof-of-storage** via Merkle trees and spot checks
- ✅ **Global DHT** for key-value storage and discovery
- ✅ **Traffic tracking** and bandwidth monitoring

#### **Network & Communication**
- ✅ **P2P networking** with peer discovery
- ✅ **NAT traversal** and relay support
- ✅ **Block propagation** and transaction broadcasting
- ✅ **Network diagnostics** and topology management

#### **Wallet & Payment Systems**
- ✅ **Ed25519 key management** with secure storage
- ✅ **Transaction processing** and validation
- ✅ **M2M payment channels** for IoT/AI
- ✅ **Micro-payments** with 1% fee collection
- ✅ **i-prefix address format** (ed25519-based)

#### **Domain & Identity Systems**
- ✅ **Human-readable domains** (@handle.ipn)
- ✅ **Premium TLDs** (.m, .cyborg, .humanoid)
- ✅ **Domain renewal** and transfer systems
- ✅ **Fee collection** for registration/renewal

#### **API & Interface Systems**
- ✅ **RESTful API** with comprehensive endpoints
- ✅ **CLI interface** with full command set
- ✅ **Explorer API** for blockchain exploration
- ✅ **Health checks** and monitoring endpoints

#### **Advanced Features**
- ✅ **Cross-chain bridge** and anchor system
- ✅ **L2 blockchain integration** support
- ✅ **Archive mode** with external sync
- ✅ **TXT metadata** for files and servers
- ✅ **Quantum-resistant cryptography** framework
- ✅ **AI system integration** capabilities

### 🎯 **PRODUCTION READY STATUS**

**IPPAN is now production-ready** with:
- ✅ **Complete consensus engine** with ZK-STARK proofs
- ✅ **Full storage system** with encryption and proofs
- ✅ **Comprehensive networking** with P2P discovery
- ✅ **Complete wallet system** with M2M payments
- ✅ **Autonomous economic model** with global fund
- ✅ **Extensive API layer** for all functionality
- ✅ **Security hardening** with cryptographic validation
- ✅ **Performance optimization** for high throughput

### 🚀 **Next Milestones**
- **Performance Optimization:** Achieve 1M TPS baseline
- **Global Deployment:** Multi-continent node distribution
- **Security Audits:** Comprehensive security review
- **Community Growth:** Developer ecosystem and partnerships
- **Production Launch:** Mainnet deployment and monitoring

**IPPAN is now a fully functional, decentralized blockchain with built-in storage, M2M payments, and autonomous governance!** 🚀

