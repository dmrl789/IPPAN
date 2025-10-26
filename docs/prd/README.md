# IPPAN Product Requirements Documents (PRDs)

This directory contains the complete set of Product Requirements Documents for the IPPAN blockchain project.

---

## Document Index

### 1. [IPPAN Vision 2025](./ippan-vision-2025.md) — **Master PRD**
*Comprehensive product vision and strategic objectives*

**Scope:** High-level product vision, business objectives, competitive positioning, and roadmap

**Key Topics:**
- Core objectives (10M TPS, deterministic AI, microsecond timing)
- HashTimer™ and IPPAN Time architecture
- Embedded AI architecture (L1 + L2)
- Tokenomics and monetary policy
- Governance framework
- Environmental and ethical principles
- Implementation roadmap (Q4 2025 → Q4 2026)
- Comparative landscape vs. Bitcoin, Ethereum, Solana, etc.

**Audience:** Executives, investors, partners, community

---

### 2. [IPPAN L1 Architecture](./ippan-l1-architecture.md) — **Technical Specification**
*Layer-1 blockchain architecture, consensus, and data allocation*

**Scope:** Technical design of the Layer-1 blockchain, parallel consensus, and L1/L2 separation

**Key Topics:**
- Global Layer-1 design principles  
- BlockDAG + parallel block creation  
- Round-based finalization (200–250 ms)  
- L1 vs L2 data allocation strategy  
- Confidentiality and compliance model  
- DNS and human-readable identity (`@user.ipn`)  
- Scalability targets (1–10 M TPS)  
- Developer reference types (Rust)

**Audience:** Core developers, protocol engineers, validator operators

---

### 3. [IPPAN Storage & Data Availability](./ippan-storage-da.md) — **Storage PRD**
*Data availability, pruning, fast sync, and confidential transactions*

**Scope:** Block storage model, erasure coding, retention policies, and privacy

**Key Topics:**
- Block data layout (headers vs. bodies)
- Data availability via Reed–Solomon erasure coding
- Retention model (validator, full node, archival node)
- Fast sync procedure with snapshot checkpoints
- Confidential transactions and mixed visibility
- ZK-STARK integration roadmap (Phases 0–3)
- Economic incentives (announcement fees, serving rewards, archival contracts)
- Networking / RPC interfaces

**Audience:** Storage engineers, infrastructure teams, privacy specialists

---

### 4. [Beyond BFT: Deterministic Learning Consensus](../BEYOND_BFT_DETERMINISTIC_LEARNING_CONSENSUS.md) — **Consensus Theory**
*Revolutionary departure from traditional BFT consensus mechanisms*

**Scope:** Theoretical foundation and mathematical proofs for IPPAN’s consensus model

**Key Topics:**
- Deterministic Learning Consensus (DLC) model  
- HashTimer™ temporal determinism  
- BlockDAG structure and parallel processing  
- AI-driven optimization (D-GBDT)  
- Mathematical foundations and security proofs  
- Performance analysis and scalability metrics  
- Comparison with traditional BFT and Nakamoto consensus  
- Implementation architecture and economic integration

**Audience:** Cryptographers, consensus researchers, protocol engineers, academics

---

## Document Relationships

