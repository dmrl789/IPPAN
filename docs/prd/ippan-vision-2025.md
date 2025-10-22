# IPPAN Product Requirements Document (PRD) — Version 2025
### *Deterministic Intelligence for a Decentralized World*

---

## 1. Overview

IPPAN is a next-generation blockchain network engineered for **determinism, scalability, and verifiable intelligence**.  
It combines *parallel block processing* with *cryptographic time ordering* and *AI-driven optimization* to achieve near-instant finality and institutional-grade reliability.

The protocol is designed to serve as a foundation for **finance, AI computation, IoT, and autonomous digital economies**, providing deterministic guarantees at microsecond precision.

---

## 2. Core Objectives

- Achieve **10 million TPS** without probabilistic finality.  
- Provide **microsecond-accurate** ordering of all events through the **HashTimer™** mechanism.  
- Embed **deterministic AI modules** directly at Layer 1 for reputation, anomaly detection, and timing optimization.  
- Offer a programmable **Layer 2 AI marketplace** for decentralized inference and model sharing.  
- Maintain **fixed monetary supply** (21 million IPN) with **capped transaction fees** and **fee-recycling**.  
- Ensure **zk-STARK verifiability** and **quantum-resistant cryptography** for long-term security.  
- Operate with **energy efficiency**, avoiding mining or probabilistic consensus.

---

## 3. Architecture Summary

### 3.1 Deterministic Core (Layer 1)
- Parallel **BlockDAG (Roundchain)** consensus with 100–250 ms rounds.  
- **HashTimer™** assigns deterministic timestamps to every transaction, guaranteeing global chronological order.  
- **Validator reputation** is computed via lightweight AI (GBDT) models identical across all nodes.  
- Consensus built on **Federated Byzantine Agreement (FBA)** principles with DAG-Fair emission scheduling.  
- zk-STARK proofs certify transaction validity and block integrity.  
- No smart contracts: L1 focuses purely on finality, security, and time.

### 3.2 Programmable Execution (Layer 2)
- Deterministic rollups and zk-VMs for custom logic and decentralized AI workloads.  
- L2 hosts **AI inference services**, **federated learning nodes**, and **application rollups**.  
- Payments and state roots anchored to L1 through HashTimer proofs.  
- Enables sectors such as DeFi, IoT, LegalTech, and research grids.

---

## 4. HashTimer™ and IPPAN Time

- Median-based global clock combining node timestamps into a single verifiable timeline.  
- Embedded in every transaction and block header for **deterministic order**.  
- Precision up to **10⁻⁶ s** (microsecond).  
- Used for:
  - Transaction finality  
  - Round scheduling  
  - zk-proof timestamping  
  - AI inference audit trails  

---

## 5. Consensus and Finality

- **Parallel block creation:** thousands of micro-blocks per second.  
- **Roundchain DAG:** rounds finalized every ~150 ms under normal conditions.  
- **Validator selection:** weighted by reputation (score 0–10 000) from the embedded AI model.  
- **Conflict resolution:** deterministic ordering via HashTimer + signature root.  
- **Fault tolerance:** >⅔ honest quorum; instant rollback for failed proofs.  
- **Emission fairness:** rewards computed per round, not per block.

---

## 6. Embedded AI Architecture

### 6.1 Layer 1 AI Modules
- Small, deterministic models (< 2 MB).  
- Trained offline, version-locked, and hash-verified on-chain.  
- Tasks:
  - Validator reputation and reliability scoring.  
  - Network congestion prediction.  
  - Anomaly detection and defense against Sybil/spam.  
  - Time-drift estimation and clock correction.  
- Deterministic inference only — no stochastic behavior or live learning.  
- Models serialized as canonical JSON and verified via SHA-256 hash.

### 6.2 Layer 2 AI Ecosystem
- Open **AI Marketplace (AIMS)** for decentralized inference services.  
- zk-Proof-of-Inference ensures results are genuine and reproducible.  
- Supports cross-domain models (finance, law, IoT, research).  
- Enables AI-as-a-Service with micro-IPN payments per query.

### 6.3 AI Governance and Registry
- Each model stored in **On-Chain Model Registry**:  
  - `{ model_id, hash, version, activation_round, signature }`.  
- New models introduced through **governance proposals** (JSON/YAML).  
- All nodes auto-update at activation round; old models deprecated deterministically.  
- Determinism and performance verified by reproducibility audits across hardware types.

---

## 7. Tokenomics

### 7.1 Monetary Policy
- **Total supply:** 21 000 000 IPN (hard cap).  
- **Unit:** 1 IPN = 10⁸ µIPN.  
- **Emission:** Round-based DAG-Fair schedule; halvings every fixed epoch (~2 years).  
- **No inflation** beyond cap.

### 7.2 Fee System
- **Capped micro-fees:**
  - Transfers ≤ 0.00001 IPN  
  - AI calls ≤ 0.000001 IPN  
- **Recycling:** all fees returned to validator reward pool weekly.  
- **No gas bidding or congestion pricing.**

### 7.3 Reward Distribution
- 20 % → block proposer (verifier)  
- 80 % → participating validators proportionally  
- Rewards auto-balanced across rounds for fairness and uptime incentives.

---

## 8. Data, Storage, and Availability

- **Distributed Hash Table (DHT)** used for file storage and HashTimer metadata.  
- Nodes optionally store user files, AI models, and zk-proof payloads.  
- Each file or model linked to its **HashTimer ID** for immutable referencing.  
- Erasure-coded redundancy for long-term persistence.

---

## 9. Security and Privacy

- **zk-STARK verification:** zero-knowledge proofs for block validation, AI inference, and data integrity.  
- **Deterministic validation path:** prevents nondeterministic forks.  
- **AI-based intrusion detection:** monitors abnormal traffic or malicious availability announcements.  
- **Quantum-resistant primitives:** SHA-3, Ed25519, STARK-friendly hashes.  
- **On-chain audit logs** ensure full traceability of validator behavior.

---

## 10. The Intelligent Internet Layer (IIL)

IPPAN can act as a **coordination overlay** for the global Internet:

- Deterministic time replaces multi-hop confirmations.  
- AI optimizes routing and message fan-out in real time.  
- Edge verification enables local finality between devices.  
- HashTimer proofs eliminate redundant authentication (HTTPS / CAs / gateways).  
- Effective latency reduction of 5–20× for real-time applications.  
- Future goal: integrate IPN micro-payments for network bandwidth accounting.

> IPPAN doesn't move photons faster — it removes the waiting caused by trust.

---

## 11. Governance Framework

- Multi-tier governance:
  1. **Protocol Council** (core validators)  
  2. **AI Committee** (model evaluation)  
  3. **Community Assembly** (on-chain voters)
- Proposals: code updates, parameter changes, or AI model registration.  
- Quorum ≥ 66 %; activation bound to round number, not wall-clock time.  
- All votes hashed, signed, and archived through HashTimer.

---

## 12. Environmental and Ethical Principles

- Consensus consumes minimal energy — no Proof-of-Work.  
- Validators encouraged to use renewable energy; carbon-offset tracking built-in.  
- Fixed emission + capped fees → predictable, sustainable economy.  
- AI governance ensures transparency, fairness, and explainability.  
- Aligns with UN SDGs 7, 9, 12, 13, 16.

---

## 13. Comparative Landscape

| Network | Consensus | TPS | Finality | AI Integration | Token Supply |
|----------|------------|-----|-----------|----------------|---------------|
| **Bitcoin** | PoW | ~7 | ~60 min | None | 21 M BTC |
| **Ethereum** | PoS | ~15 k | 12 s | External | Inflationary |
| **Solana** | PoH + BFT | ~60 k | ~400 ms | None | Inflationary |
| **Gensyn / Bittensor** | PoS / Work Market | — | variable | AI-training focus | Dynamic |
| **IPPAN** | Deterministic DAG (FBA) | **>10 M** | **100–250 ms** | **L1 + L2 AI integrated** | **21 M IPN (fixed)** |

---

## 14. Implementation Roadmap (2025 → 2026)

| Quarter | Milestone |
|----------|------------|
| Q4 2025 | Integrate `ai_core` & `ai_registry` crates; release reputation_v1 model |
| Q1 2026 | Launch devnet with AI governance voting + DAG-Fair emission |
| Q2 2026 | zk-STARK proof aggregation in consensus path |
| Q3 2026 | Intelligent Internet Layer pilot (edge verification) |
| Q4 2026 | Mainnet launch v1.0 — deterministic AI core active |

---

## 15. Future Outlook

- **Self-optimizing infrastructure**: network learns to adjust its own parameters deterministically.  
- **Federated AI collaboration**: L2 models share updates via proof-of-inference.  
- **Cross-chain bridges** with verifiable time anchors.  
- **Integration with national or institutional digital-asset networks**.  
- **Quantum-secure migration** roadmap finalized by 2028.

---

## 16. Conclusion

IPPAN unites **time, intelligence, and trust** into a single deterministic fabric.  
Its architecture eliminates the guesswork of traditional blockchains, embedding verifiable AI as the new logic of coordination.  
By combining **HashTimer™ determinism**, **AI-guided consensus**, and **zk-verifiable computation**, IPPAN becomes the backbone of the **verifiable AI economy** —  
a network capable of thinking, optimizing, and sustaining itself in harmony with the planet and society.

> *IPPAN is not merely faster blockchain — it is the living proof that intelligence, when deterministic, becomes infrastructure.*

---

**Document version:** 2025-10-22  
**Maintainer:** IPPAN Foundation / dmrl789  
**License:** CC-BY-SA-4.0  
