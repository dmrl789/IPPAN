# Security Hardening Phase 1 (RC)

> Mainnet gating checklist: `docs/release/v1-mainnet-checklist.md` tracks remaining security launch blockers.

## Scope completed

- RC threat model documented at `docs/security/threat-model-rc.md`.
- Security crate exercised with rate limits (per-IP, per-endpoint, global), IP whitelist enforcement, and lockout recovery paths.
- RPC guard coverage for abusive clients (rate-limit bursts and repeated failures) with integration tests tied to `SecurityManager`.
- P2P abuse scenarios covered in unit tests (malformed peer spam, rapid connect/disconnect) to ensure state remains healthy.

## How to run the checks

```bash
cargo test -p ippan-security -- --nocapture
cargo test -p ippan-rpc -- --nocapture
cargo test -p ippan-p2p -- --nocapture
```

These focus on rate limiting, whitelist enforcement, lockouts, RPC guards, and peer churn handling.

## Deferred / Phase 2 items

- External third-party security audit and threat review.
- OS/network hardening guidance (firewall recipes, sandboxing, kernel tuning).
- Extended DoS handling (sustained volumetric attacks, advanced circuit breaking across services).
- Formal proofs for DLC economics and ZK-STARK enforcement once the proof system moves beyond design.
