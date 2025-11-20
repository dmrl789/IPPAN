# ZK-IPPAN: ZK-STARK Design for IPPAN

## 1. Objectives

- Establish how ZK-STARK proofs can attest to correctness of deterministic learning consensus (D-GBDT/DLC), HashTimer ordering, and emission rules without altering RC-stable core logic.
- Provide a phased roadmap for introducing proofs as an optional audit and trust layer before any code-level integration.
- Keep arithmetic and data representations consistent with fixed-point, float-free math used by IPPAN AI and economics crates.
- Clarify actors, proof cadence, and storage so proofs can be added incrementally (off-chain first, optional on-chain hooks later).

### Non-goals / Out of scope
- No new Rust code, dependencies, or circuit implementations in this phase.
- No changes to consensus safety or liveness assumptions; STARKs are additive verification, not a prerequisite for protocol correctness.
- No commitments on specific proving stacks or libraries; selection remains open for prototyping.

## 2. Where ZK proofs add value in IPPAN

- **Deterministic AI / D-GBDT (DLC) scoring:** Prove that validators compute round scores using the approved deterministic gradient-boosted decision trees and fixed-point feature pipelines, preventing adversarial model tweaks or floating-point drift.
- **Emission & fairness rules:** Attest that reward allocation and emission follow DAG Fair Emission and DLC weighting rules based on previously committed scores and balances.
- **HashTimer ordering guarantees:** Provide evidence that transaction and round ordering respect HashTimer commitments (hash preimage + timer), reducing disputes about replay or reorder advantages.
- **Auditability and reproducibility:** Offer verifiable artifacts for off-chain or on-chain auditors without exposing private validator data; ZK increases confidence without changing existing replay or RPC safety guarantees.

## 3. Proof targets (v1 scope)

### 3.1 Validator scoring correctness
- **What is proven:** Given a committed feature vector per validator per round and the published D-GBDT model parameters, the computed score matches the protocol-defined fixed-point inference rules.
- **Public inputs:**
  - Round identifier and HashTimer commitment for the round.
  - Merkle/commitment root of the D-GBDT model parameters (publicly pinned hash).
  - Commitment to per-validator feature vectors (root over feature set for the round).
  - Expected validator scores or their commitments.
- **Private inputs:**
  - Individual validator feature vectors (per validator or batched subset) used for scoring.
  - Optional salts for feature commitments if privacy of raw features is desired.
- **Outputs:**
  - STARK proof that the score derivation follows deterministic model evaluation and fixed-point arithmetic, producing committed scores.
  - Optionally, per-validator score commitments for subsequent reward calculation.

### 3.2 Emission & distribution correctness
- **What is proven:** Reward and emission distribution for a round/epoch respects DAG Fair Emission curves and DLC weighting using previously committed scores and balances.
- **Public inputs:**
  - Round/epoch identifier.
  - Commitment to validator scores for the period.
  - Previous emission pool size and validator stake/balance commitments.
  - Emission policy parameters (curve constants, caps, decay factors) already part of protocol config.
- **Private inputs:**
  - Validator balances if not already public; optional anonymization via commitments.
- **Outputs:**
  - STARK proof that computed per-validator reward deltas and total emission match protocol rules and conserve totals (no overflow/underflow in fixed-point math).
  - Commitment to updated balances for the next period.

### 3.3 Optional targets (future phases)
- Full block/round validity STARK tying together mempool admission, HashTimer ordering, scoring, and emission in a single proof.
- Historical state commitments (per-epoch Merkle roots for balances and scores) with STARK proofs of correct roll-forward.
- Privacy-preserving variants that hide validator-specific features or balances while proving aggregate correctness.

## 4. Integration model

- **Proof generators:**
  - Primary: validators or designated provers generate scoring proofs alongside round participation.
  - Secondary: auditors or community provers generate emission proofs periodically (e.g., per epoch) to validate aggregate fairness.
- **Cadence:**
  - Scoring proofs: per round or sampled rounds (configurable) to balance cost vs coverage.
  - Emission proofs: per epoch or at checkpoints aligned with reward distribution events.
- **Verification modes:**
  - **Off-chain (v1 default):** Validators and auditors verify proofs locally; failures can be escalated via governance or slashing proposals.
  - **On-chain/hybrid (future):** Lightweight verifier contract or module consumes proofs/commitments, gating reward disbursement. Integration deferred until performance and cost validated.
- **Failure handling:**
  - Off-chain detection triggers dispute or challenge windows without halting consensus.
  - On-chain (future) would reject or pause reward actions if proofs missing/invalid.
- **Storage:**
  - Proofs stored off-chain (e.g., blob storage or IPFS) with references/commitments in consensus metadata or governance records.
  - Only commitments or short references flow through consensus to avoid payload bloat.

## 5. Data model & constraints for circuits

- **Arithmetic:** Fixed-point representation mirroring IPPAN AI crates (no floating point); deterministic rounding and saturation rules documented alongside model parameters.
- **Hash functions:** Favor protocol-approved hashes (e.g., BLAKE3) for commitments; circuit-friendly variants must preserve collision resistance and align with HashTimer commitments when reused.
- **State commitments:** Merkle (or similar) roots over validator features, scores, and balances; roots are public inputs to proofs and can be referenced in consensus messages.
- **Model representation:** D-GBDT trees encoded as fixed-depth decision nodes with integer thresholds and weights; feature vectors encoded as fixed-length arrays of fixed-point integers.
- **Circuit constraints:** Avoid dynamic branching; normalize tree evaluation order to match existing deterministic inference implementation; ensure reproducibility across provers.

## 6. Protocol flows

### Round with STARK proof for validator scoring (textual sequence)
1. Validators collect round features and build feature commitments; consensus publishes round ID and HashTimer commitment.
2. Prover (validator or delegate) evaluates D-GBDT model deterministically to compute scores and generates a STARK proof tying commitments, model root, and outputs.
3. Proof (or reference) is gossiped/off-chain shared; validators verify off-chain. Valid scores are committed for reward calculation; failures trigger dispute escalation but do not block consensus progress.

### Emission distribution with STARK proof (textual sequence)
1. At epoch boundary, aggregator collects committed scores, balances, and emission parameters; builds state commitments.
2. Prover computes reward deltas using DAG Fair Emission/DLC weighting and produces a STARK proof that totals and per-validator outputs match the rules.
3. Verification occurs off-chain by validators/auditors; verified outputs inform reward disbursement. Invalid proofs prompt governance review before funds move.

## 7. Performance considerations

- **Proof size & cost:** Expect tens to hundreds of KB per proof depending on batch size; verification should target sub-second on commodity hardware for off-chain modes.
- **Frequency vs cost trade-off:** Sampling (e.g., every N rounds) can provide assurance with bounded overhead; full coverage reserved for audits or high-risk periods.
- **Batching:** Multiple validator scores can be proven in batches to amortize overhead; emission proofs naturally aggregate per epoch.
- **Data availability:** Proofs rely on committed roots; ensure feature and score commitments are retained for the duration of challenge windows.

## 8. Roadmap & phases

- **Phase 0:** This PRD and simulations; finalize inputs/outputs and commitment formats.
- **Phase 1:** Prototype circuits and off-chain verifier (testnet only); select proving library (e.g., Winterfell or similar) after benchmarking.
- **Phase 2:** Optional integration into DLC/reward flows with off-chain verification and governance-enforced challenge periods.
- **Phase 3:** Extended proofs (full block validity or historical rollups) and potential on-chain verification hooks if cost-effective.

## 9. Open questions

- Which STARK library best fits IPPAN’s fixed-point, hash, and performance requirements? (Winterfell vs custom vs other)
- How to encode HashTimer commitments most efficiently as circuit-friendly constraints without compromising security assumptions?
- What minimum sampling rate balances assurance and cost for production deployments?
- Should feature commitments remain public or blinded (with salts) to protect validator strategy while preserving auditability?
- How should governance handle disputes raised from failed off-chain verification (slashing vs temporary quarantine of rewards)?
