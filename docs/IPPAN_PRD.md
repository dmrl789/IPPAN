# 📜 IPPAN — Product Requirements Document (PRD)

---

## 1️⃣ Overview

**Product Name:** IPPAN (Immutable Proof & Availability Network)  
**Type:** Fully decentralized Layer-1 blockchain with built-in global DHT storage.  
**Core Idea:**  
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

### ✅ 3.2 BlockDAG & Simple Round Consensus
- Blocks are connected in a Directed Acyclic Graph (DAG)
- Rounds have a simple linear structure for consensus coordination
- Deterministic ordering via HashTimers
- Validators selected by **verifiable randomness** so no single party can manipulate selection

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

### ✅ 3.7 Verifiable Randomness for Validator Selection
- Validators randomly selected for block production & validation
- Selection is transparent and verifiable on-chain
- Ensures fairness while preventing centralization

### ✅ 3.8 Encrypted, Sharded Storage
- Files are AES-256 encrypted, sharded, auto-balanced across nodes
- Built-in global DHT maps which nodes hold shards
- Proof-of-Storage via Merkle trees & spot checks
- Clickable content hashes show proof-of-existence + live storage status

### ✅ 3.9 Human-Readable Domains
- Users, devices, and AI agents can register handles like `@alice.ipn` or `@bot.iot`
- Premium TLDs possible (`.m`, `.cyborg`, `.humanoid`)
- Annual fees fund the Global Fund and incentivize long-tail storage

### ✅ 3.10 Machine-to-Machine (M2M) Payments
- Micro-payments possible in smallest IPN units
- Perfect for IoT devices and autonomous AI agents
- Every M2M payment pays the 1% micro-fee to the Global Fund

---

## 4️⃣ Node Architecture — IPPANCore

```
+----------------------------------------------------------+
|                    IPPANCore Node                        |
+----------------------------------------------------------+
| 1) BlockDAG Consensus Engine                             |
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
+----------------------------------------------------------+
```

---

## 5️⃣ Implementation Status (2024)

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

**IPPAN is now production-ready and ready for deployment, testing, and community development.**

