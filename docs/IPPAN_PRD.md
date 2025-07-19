# 📜 IPPAN — Product Requirements Document (PRD)

---

## 1️⃣ Overview

**Product Name:** IPPAN (Immutable Proof & Availability Network)  
**Type:** Global Layer-1 blockchain with 1-10 million TPS capacity and built-in global DHT storage.  
**Core Idea:**  
- **Global L1 blockchain** designed for planetary-scale adoption
- **1-10 million TPS** throughput target for mass adoption
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

### ✅ 3.2 BlockDAG & ZK-STARK Consensus
- **BlockDAG Structure:** Blocks connected in Directed Acyclic Graph for parallel processing
- **ZK-STARK Rounds:** Sub-second deterministic finality despite intercontinental latency
- **Round Structure:** 
  - Round duration: 1-5 seconds
  - ZK-STARK proof size: 50-100 KB
  - Verification time: 10-50ms
- **Block Dimensions:**
  - Max block size: 10 MB
  - Max transactions per block: 100,000
  - Block header: 256 bytes
  - Transaction size: 100-500 bytes average
- **Transaction Structure:**
  - Ed25519 signature (64 bytes)
  - Public key (32 bytes)
  - Transaction type (1 byte)
  - Amount (8 bytes)
  - Timestamp (8 bytes)
  - HashTimer (32 bytes)
  - Total: ~145 bytes base + variable data
- **Deterministic Ordering:** Via HashTimers with 0.1μs precision
- **Validator Selection:** Verifiable randomness prevents manipulation
- **Optimized for 1-10 million TPS** through parallel processing and ZK-STARK consensus

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
2- Annual fees fund the Global Fund and incentivize long-tail storage

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

---

## 4️⃣ Technical Specifications & Performance

### 📊 Block & Transaction Specifications

#### **Block Structure**
```
Block Header (256 bytes):
├── Block Hash (32 bytes)
├── Parent Hash (32 bytes)
├── Round Number (8 bytes)
├── Timestamp (8 bytes)
├── Validator ID (32 bytes)
├── Block Size (4 bytes)
├── Transaction Count (4 bytes)
├── Merkle Root (32 bytes)
├── ZK-STARK Proof Reference (32 bytes)
├── HashTimer (32 bytes)
└── Padding (32 bytes)

Block Body:
├── Transaction List (variable)
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
Round (1-5 seconds):
├── Round Header (256 bytes)
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
+----------------------------------------------------------+
```

---

## 6️⃣ Implementation Status (2024)

All major systems are **implemented and integrated**:
- BlockDAG consensus, HashTimers, IPPAN Time
- Staking, validator selection, slashing
- AES-256 encrypted, sharded storage
- Global DHT, proof-of-storage, traffic tracking
- Human-readable domains, premium TLDs, renewals
- Keyless global fund, weekly autonomous distribution
- Local wallet, Ed25519 keys, staking, rewards
- M2M payment channels for IoT/AI, micro-fees
- Full RESTful API, CLI, explorer endpoints
- **Address format with i-prefix (ed25519-based)**

**IPPAN is now production-ready and ready for deployment, testing, and community development.**

### 🚀 Next Milestones
- **Performance Optimization:** Achieve 1M TPS baseline
- **Global Deployment:** Multi-continent node distribution
- **Security Audits:** Comprehensive security review
- **Community Growth:** Developer ecosystem and partnerships
- **Production Launch:** Mainnet deployment and monitoring

---

## 7️⃣ Competitive Positioning

### 🌟 Unique Advantages
- **1-10M TPS Target:** Unprecedented throughput for L1 blockchain
- **Built-in Storage:** Global DHT with proof-of-storage
- **Precision Timestamping:** 0.1 microsecond accuracy
- **Keyless Fund:** Autonomous, unstoppable incentive system
- **M2M Focus:** Designed for IoT and AI applications
- **Rust Implementation:** Performance, security, and reliability

### 🎯 Target Markets
- **IoT Networks:** Billions of connected devices
- **AI Services:** Autonomous agents and AI applications
- **Global Payments:** Cross-border and micro-payments
- **Data Storage:** Decentralized, encrypted storage
- **Timestamping:** Proof-of-existence services
- **Domain Services:** Human-readable identifiers

---

## 8️⃣ Roadmap

### Q1 2024: Foundation
- ✅ Core protocol implementation
- ✅ Address format standardization
- ✅ Basic testing and validation

### Q2 2024: Performance
- 🎯 Achieve 1M TPS baseline
- 🎯 Performance optimization
- 🎯 Security audits

### Q3 2024: Global Scale
- 🎯 Multi-continent deployment
- 🎯 Community growth
- 🎯 Developer ecosystem

### Q4 2024: Production Launch
- 🎯 Mainnet deployment
- 🎯 Monitoring and optimization
- 🎯 5M TPS target

### 2025: Global Adoption
- 🎯 10M TPS target
- 🎯 Mass adoption
- 🎯 Ecosystem expansion

