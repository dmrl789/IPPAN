# IPPAN Product Requirements Document (PRD) — Version 2025

### Deterministic Intelligence for a Decentralized World

---

## 1. Overview

IPPAN is a next-generation blockchain network engineered for determinism, scalability, and verifiable intelligence.
It combines parallel block processing with cryptographic time ordering and AI-driven optimization to achieve near-instant finality and institutional-grade reliability.

The protocol is designed to serve as a foundation for finance, AI computation, IoT, and autonomous digital economies, providing deterministic guarantees at microsecond precision.

---

## 2. Core Objectives

- Achieve 10 million TPS without probabilistic finality.
- Provide microsecond-accurate ordering of all events through the HashTimer™ mechanism.
- Embed deterministic AI modules directly at Layer 1 for reputation, anomaly detection, and timing optimization.
- Offer a programmable Layer 2 AI marketplace for decentralized inference and model sharing.
- Maintain fixed monetary supply (21 million IPN) with capped transaction fees and fee-recycling.
- Ensure zk-STARK verifiability and quantum-resistant cryptography for long-term security.
- Operate with energy efficiency, avoiding mining or probabilistic consensus.

---

## 3. Architecture Summary

### 3.1 Deterministic Core (Layer 1)
- Parallel BlockDAG (Roundchain) consensus with 100–250 ms rounds.
- HashTimer™ assigns deterministic timestamps to every transaction, guaranteeing global chronological order.
- Validator reputation is computed via lightweight AI (GBDT) models identical across all nodes.
- Consensus built on Federated Byzantine Agreement (FBA) principles with DAG-Fair emission scheduling.
- zk-STARK proofs certify transaction validity and block integrity.
- No smart contracts: L1 focuses purely on finality, security, and time.

### 3.2 Programmable Execution (Layer 2)
- Deterministic rollups and zk-VMs for custom logic and decentralized AI workloads.
- L2 hosts AI inference services, federated learning nodes, and application rollups.
- Payments and state roots anchored to L1 through HashTimer proofs.
- Enables sectors such as DeFi, IoT, LegalTech, and research grids.

---

## 4. HashTimer™ and IPPAN Time

- Median-based global clock combining node timestamps into a single verifiable timeline.
- Embedded in every transaction and block header for deterministic order.
- Precision up to 10⁻⁶ s (microsecond).
- Used for:
  - Transaction finality
  - Round scheduling
  - zk-proof timestamping
  - AI inference audit trails

---

## 5. Consensus and Finality

- Parallel block creation: thousands of micro-blocks per second.
- Roundchain DAG: rounds finalized every ~150 ms under normal conditions.
- Validator selection: weighted by reputation (score 0–10 000) from the embedded AI model.
- Conflict resolution: deterministic ordering via HashTimer + signature root.
- Fault tolerance: >⅔ honest quorum; instant rollback for failed proofs.
- Emission fairness: rewards computed per round, not per block.

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
- Open AI Marketplace (AIMS) for decentralized inference services.
- zk-Proof-of-Inference ensures results are genuine and reproducible.
- Supports cross-domain models (finance, law, IoT, research).
- Enables AI-as-a-Service with micro-IPN payments per query.

### 6.3 AI Governance and Registry
- Each model stored in On-Chain Model Registry:
  - `{ model_id, hash, version, activation_round, signature }`.
- New models introduced through governance proposals (JSON/YAML).
- All nodes auto-update at activation round; old models deprecated deterministically.
- Determinism and performance verified by reproducibility audits across hardware types.

---

## 7. Tokenomics

### 7.1 Monetary Policy
- Total supply: 21 000 000 IPN (hard cap).
- Unit: 1 IPN = 10⁸ µIPN.
- Emission: Round-based DAG-Fair schedule; halvings every fixed epoch (~2 years).
- No inflation beyond cap.

### 7.2 Fee System
- Capped micro-fees:
  - Transfers ≤ 0.00001 IPN
  - AI calls ≤ 0.000001 IPN
- Recycling: all fees returned to validator reward pool weekly.
- No gas bidding or congestion pricing.

### 7.3 Reward Distribution
- 20 % → block proposer (verifier)
- 80 % → participating validators proportionally
- Rewards auto-balanced across rounds for fairness and uptime incentives.

---

## 8. DAG-Fair Emission Framework

### 8.1 Rationale

In IPPAN, **blocks are micro-events**, not emission units.
Unlike linear blockchains (e.g., Bitcoin), where block intervals define issuance (1 block = 1 reward), IPPAN's **BlockDAG** creates thousands of micro-blocks per second within overlapping rounds.

To maintain **scarcity, determinism, and fairness**, emission must therefore be **round-based**, not block-based.
Each round (≈100–250 ms) aggregates many blocks from multiple validators.
Rewards are computed **per round** and distributed proportionally to the nodes that participated in that round's validation and verification.

---

### 8.2 Core Parameters

| Parameter                   | Description                      | Example                              |
| --------------------------- | -------------------------------- | ------------------------------------ |
| **Total Supply**            | Hard-capped monetary base        | 21 000 000 IPN                       |
| **Round Duration**          | Average consensus interval       | 100 ms                               |
| **Rounds per Second**       | Network heartbeat frequency      | ≈ 10                                 |
| **Annual Rounds**           | 31.5 × 10⁶ s × 10 = 315 M rounds | deterministic                        |
| **Halving Interval**        | Reward halving schedule          | every ≈ 6.3 × 10⁸ rounds (≈ 2 years) |
| **Initial Round Reward R₀** | Base emission at genesis         | 0.0001 IPN per round                 |

---

### 8.3 Emission Function

Each round *t* issues a deterministic amount R(t) according to:

```
R(t) = R₀ / 2^⌊t / Tₕ⌋
```

where

* R₀ = 0.0001 IPN (initial reward per round)
* Tₕ = halving interval (~2 years)
* t = round index (HashTimer-based)

The **total network reward per round** R(t) is then split among all participating validators.

---

### 8.4 Distribution Within a Round

Let

* Bᵣ = number of micro-blocks finalized in round r
* Vᵣ = validators active in the quorum
* R(t) = total reward pool for that round

Then:

```
Rewardᵦ = R(t) / Bᵣ

Rewardᵥ = Σ(b ∈ Bᵣ(v)) [R(t) / Bᵣ × f(v)]
```

where *f(v)* is a weighting factor based on node role and uptime:

* Proposer = 1.2×
* Verifier = 1.0×
* Observer = 0×

All micro-rewards accumulate into each validator's payout ledger and are periodically settled to the wallet.

---

### 8.5 Fairness Properties

| Property                  | Description                                                          |
| ------------------------- | -------------------------------------------------------------------- |
| **Proportional fairness** | Rewards scale with actual participation — idle nodes earn nothing.   |
| **Temporal determinism**  | Emission tied to HashTimer rounds, not unpredictable block creation. |
| **Hardware neutrality**   | Each round carries the same reward pool regardless of local speed.   |
| **Supply integrity**      | 21 M IPN hard cap, enforced mathematically.                          |

---

### 8.6 Emission Curve (Illustrative)

| Period    | Reward per round (IPN) | Annual issuance (IPN) | Cumulative supply (IPN) |
| --------- | ---------------------: | --------------------: | ----------------------: |
| Years 1–2 |                 0.0001 |                3.15 M |                  3.15 M |
| Years 3–4 |                0.00005 |                1.58 M |                  4.73 M |
| Years 5–6 |               0.000025 |                0.79 M |                  5.52 M |
| …         |                      … |                     … |   asymptotically → 21 M |

(Parameters tuned to achieve ~10-year full emission.)

---

### 8.7 Validator Reward Composition

| Component                   | Share | Description                                  |
| --------------------------- | ----- | -------------------------------------------- |
| **Round Emission R(t)**     | 60 %  | Base reward distributed per round            |
| **Transaction Fees**        | 25 %  | Deterministic micro-fees per tx              |
| **AI Service Commissions**  | 10 %  | From inference and compute tasks             |
| **Network Reward Dividend** | 5 %   | Weekly redistribution by uptime × reputation |

All components are logged and verifiable through HashTimer records and zk-STARK proofs.

---

### 8.8 Fee-Cap Integration

Each transaction carries a micro-fee (≈ 1 µIPN), but the total fees per round are capped to prevent economic centralization:

```
Fᵣ ≤ 0.1 × R(t)
```

→ ensuring fees never dominate validator income and network participation remains open and balanced.

---

### 8.9 Governance Controls

* All emission parameters (`R₀`, `Tₕ`, `f(v)`, fee caps) reside in **on-chain configuration**, modifiable only by super-majority validator vote.
* Every epoch, nodes verify that the **total minted IPN** equals the deterministic emission schedule.
* Any rounding excess is **auto-burned** at epoch closure, guaranteeing supply integrity.

---

### 8.10 DAG-Fair Summary

| Goal                            | Mechanism                   | Outcome                              |
| ------------------------------- | --------------------------- | ------------------------------------ |
| Fair emission under parallelism | Round-based reward function | Equal opportunity for all validators |
| Predictable monetary curve      | Deterministic halving       | Bitcoin-grade credibility            |
| Inflation control               | Hard-cap + auto-burn        | Immutable supply                     |
| Long-term sustainability        | Fee + AI revenue            | Continuous validator incentive       |
| Transparency                    | HashTimer + zk-STARK proofs | Fully auditable economics            |

---

> **IPPAN's DAG-Fair Emission** transforms block mining into **time-anchored micro-rewards**.
> Each HashTimer round defines a precise emission slice shared fairly among validators — enabling millions of blocks per second without inflation drift, ensuring a stable, transparent, and verifiable monetary policy.

---

## 9. Data, Storage, and Availability

- Distributed Hash Table (DHT) used for file storage and HashTimer metadata.
- Nodes optionally store user files, AI models, and zk-proof payloads.
- Each file or model linked to its HashTimer ID for immutable referencing.
- Erasure-coded redundancy for long-term persistence.

---

## 10. Security and Privacy

- zk-STARK verification: zero-knowledge proofs for block validation, AI inference, and data integrity.
- Deterministic validation path: prevents nondeterministic forks.
- AI-based intrusion detection: monitors abnormal traffic or malicious availability announcements.
- Quantum-resistant primitives: SHA-3, Ed25519, STARK-friendly hashes.
- On-chain audit logs ensure full traceability of validator behavior.

---

## 11. The Intelligent Internet Layer (IIL)

IPPAN can act as a coordination overlay for the global Internet:

- Deterministic time replaces multi-hop confirmations.
- AI optimizes routing and message fan-out in real time.
- Edge verification enables local finality between devices.
- HashTimer proofs eliminate redundant authentication (HTTPS / CAs / gateways).
- Effective latency reduction of 5–20× for real-time applications.
- Future goal: integrate IPN micro-payments for network bandwidth accounting.

> IPPAN doesn’t move photons faster — it removes the waiting caused by trust.

---

## 12. Governance Framework

- Multi-tier governance:
  1. Protocol Council (core validators)
  2. AI Committee (model evaluation)
  3. Community Assembly (on-chain voters)
- Proposals: code updates, parameter changes, or AI model registration.
- Quorum ≥ 66 %; activation bound to round number, not wall-clock time.
- All votes hashed, signed, and archived through HashTimer.

---

## 13. Environmental and Ethical Principles

- Consensus consumes minimal energy — no Proof-of-Work.
- Validators encouraged to use renewable energy; carbon-offset tracking built-in.
- Fixed emission + capped fees → predictable, sustainable economy.
- AI governance ensures transparency, fairness, and explainability.
- Aligns with UN SDGs 7, 9, 12, 13, 16.

---

## 14. Comparative Landscape

| Network | Consensus | TPS | Finality | AI Integration | Token Supply |
|----------|------------|-----|-----------|----------------|---------------|
| Bitcoin | PoW | ~7 | ~60 min | None | 21 M BTC |
| Ethereum | PoS | ~15 k | 12 s | External | Inflationary |
| Solana | PoH + BFT | ~60 k | ~400 ms | None | Inflationary |
| Gensyn / Bittensor | PoS / Work Market | — | variable | AI-training focus | Dynamic |
| IPPAN | Deterministic DAG (FBA) | >10 M | 100–250 ms | L1 + L2 AI integrated | 21 M IPN (fixed) |

---

## 15. Implementation Roadmap (2025 → 2026)

| Quarter | Milestone |
|----------|------------|
| Q4 2025 | Integrate `ai_core` & `ai_registry` crates; release `reputation_v1` model |
| Q1 2026 | Launch devnet with AI governance voting + DAG-Fair emission |
| Q2 2026 | zk-STARK proof aggregation in consensus path |
| Q3 2026 | Intelligent Internet Layer pilot (edge verification) |
| Q4 2026 | Mainnet launch v1.0 — deterministic AI core active |

---

## 16. Future Outlook

- Self-optimizing infrastructure: network learns to adjust its own parameters deterministically.
- Federated AI collaboration: L2 models share updates via proof-of-inference.
- Cross-chain bridges with verifiable time anchors.
- Integration with national or institutional digital-asset networks.
- Quantum-secure migration roadmap finalized by 2028.

---

## 17. Conclusion

IPPAN unites time, intelligence, and trust into a single deterministic fabric.
Its architecture eliminates the guesswork of traditional blockchains, embedding verifiable AI as the new logic of coordination.
By combining HashTimer™ determinism, AI-guided consensus, and zk-verifiable computation, IPPAN becomes the backbone of the verifiable AI economy —
a network capable of thinking, optimizing, and sustaining itself in harmony with the planet and society.

> IPPAN is not merely faster blockchain — it is the living proof that intelligence, when deterministic, becomes infrastructure.

---

Document version: 2025-10-22  
Maintainer: IPPAN Foundation / dmrl789  
License: CC-BY-SA-4.0

---

## Appendix A — Acceptance Criteria (initial milestones)

### A.1 Q4 2025: Integrate `ai_core` & `ai_registry` and ship `reputation_v1`

- Deterministic model load:
  - Nodes load `reputation_v1` from a canonical JSON file (< 2 MB).
  - SHA-256 of the JSON matches the on-chain/registry hash.
  - Model version is logged and exposed via RPC.
- Registry mechanics:
  - Registry entry structure is `{ model_id, hash, version, activation_round, signature }`.
  - Governance proposal to add/activate a model is validated and stored.
  - Nodes honor `activation_round` and switch models exactly at that round.
- Deterministic inference:
  - Given a public test vector of validator telemetry, all supported platforms (x86_64, aarch64) produce identical scores.
  - Scores are integers in [0, 10000]; no floating-point nondeterminism in consensus path.
- Consensus coupling (read-only weighting):
  - Reputation scores are consumed deterministically by validator selection logic without altering block validity rules.
  - Disabling AI (via config flag) falls back to neutral weights without fork risk.
- Toolchain and reproducibility:
  - Builds succeed with the pinned toolchain in `rust-toolchain.toml`.
  - `cargo test -q` passes for `ai_core`, `ai_registry`, and any touched crates.
  - `cargo clippy -D warnings` is clean for changed code.
- Observability:
  - `rpc_get_ai_model_info` (or equivalent) returns model id, version, hash, activation round.
  - Node logs include one-line confirmation on activation with the model hash.

### A.2 Governance and Model Lifecycle

- Proposal format:
  - Governance proposal files (JSON/YAML) validate against a published schema.
  - Invalid signatures or mismatched hashes are rejected with explicit error codes.
- Roll-forward only:
  - Model activation never occurs before `activation_round` and is monotonic by version.
- Deprecation:
  - Nodes retain previous model metadata for audit but never re-activate deprecated versions without a new proposal.

### A.3 Determinism & Security Invariants

- No RNG or time-based branching in inference or registry code paths.
- Hashes use SHA-256 or a STARK-friendly equivalent; results match reference vectors.
- All network-facing RPCs that expose AI state are read-only and rate-limited.

### A.4 Documentation & Release Artifacts

- `docs/` updated with:
  - Model registry schema.
  - Reproducibility checklist.
  - Test vectors and expected outputs.
- Release artifacts include:
  - Canonical model JSON, its SHA-256, and signature bundle.
  - Change log entry referencing acceptance tests.
