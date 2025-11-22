# IPPAN Threat Model and Security Hardening (Phase 1)

## High-Value Assets
- **IPN balances and payment ledger**: atomic-unit accounting, anti-double-spend enforcement, and fee distribution.
- **HashTimer ordering and consensus integrity**: deterministic ordering, DLC fairness model, and round finalization.
- **Validator set health**: validator reputation, staking/selection data, and shadow verifier configuration.
- **D-GBDT models and registry state**: deterministic model artifacts, canonical JSON/BLAKE3 hashes, and active model pointers.
- **Network overlays**: P2P/libp2p mesh, IPNDHT descriptors for files/handles, and peer metadata.
- **RPC/API control plane**: rate-limited endpoints, admin/dev-only toggles, and configuration values that gate unsafe helpers.

## Adversary Model
- **Network attacker**: floods RPC/P2P with malformed or replayed traffic, attempts DoS, or scrapes DHT state.
- **Byzantine validator/operator**: equivocates, spams announcements, or attempts to skew DLC fairness/HashTimer ordering.
- **Spammy or abusive clients**: replay/duplicate transactions, brute-force auth-gated endpoints, or misuse dev helpers outside dev mode.
- **Economically rational adversary**: seeks profit via fee evasion, unfair ordering, or exploiting deterministic AI path divergence.

## Threats Mitigated in Phase 1
- **RPC guardrails**: SecurityManager-backed per-IP/per-endpoint rate limits, IP allow/deny lists, and dev-mode gating for helpers.
- **P2P/IPNDHT hygiene**: peer caps, churn throttling, malformed message rejection, and DHT descriptor conflict checks.
- **Deterministic execution**: no floating-point arithmetic in runtime paths; canonical JSON + BLAKE3 hashing for D-GBDT artifacts; determinism tests to prevent model drift.
- **Consensus integrity**: HashTimer/DLC ordering invariants exercised in long-run simulations; monotonic time clamping and skew rejection.
- **Input validation**: strict payload shape checks across payment/handle/file endpoints with bounded pagination and integer-only amounts.

## Deferred / Phase 2 (Explicitly Out of Scope Now)
- **External reviews**: third-party security/cryptography audit, coordinated bug bounty, and red-team exercises.
- **Advanced DoS & sandboxing**: OS-level firewalls, kernel hardening, container isolation, and volumetric attack handling.
- **Side channels & supply chain**: timing/power side-channel defenses, compiler/toolchain compromise, and dependency typosquat detection beyond existing audits.
- **Novel cryptographic breaks**: new attacks against hash/signature primitives or deterministic hashing; mitigations to follow upstream guidance.
- **ZK/STARK enforcement**: proof system integration is still in design; not enforced in Phase 1.

## Readiness Path
- **Phase 1**: Application-layer guards, deterministic AI enforcement, and P2P/DHT checks are in place and tested; remaining work is primarily external validation and operational hardening.
- **Phase 2**: Schedule external audits, launch bug bounty/red-team program, and add OS/network hardening playbooks; expand CI to cover cross-crate no-float scanning and long-run chaos tests.
- **Phase 3**: Integrate audit findings, enable optional ZK/STARK proofs, and roll out sandboxed deployments with defense-in-depth defaults.
