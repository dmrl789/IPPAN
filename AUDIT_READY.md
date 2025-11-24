# IPPAN Audit-Ready Snapshot

- **Operational note:** Operational readiness items (validators, gateways, DR, security) are tracked in [`PRODUCTION_READINESS_CHECKLIST.md`](./PRODUCTION_READINESS_CHECKLIST.md). Complete that checklist before certifying this snapshot as “audit ready”, and continue to use [`CHECKLIST_AUDIT_MAIN.md`](./CHECKLIST_AUDIT_MAIN.md) for feature-specific verification.

- **Project:** IPPAN distributed ledger with DLC-driven consensus and deterministic AI (D-GBDT).
- **Release Candidate:** v0.9.0-rc1 (Audit Candidate)
- **Target Commit:** `<commit hash to be inserted by maintainer when selecting the audit snapshot>`

## Canonical Protocol Specification

**NEW:** The normative protocol specification is now available at:
- [`docs/spec/IPPAN_PROTOCOL_SPEC.md`](./docs/spec/IPPAN_PROTOCOL_SPEC.md)

This document defines the implementation-agnostic protocol rules using RFC 2119 normative language (MUST/SHOULD/MAY). All auditors and implementers MUST refer to this specification as the canonical source of truth.

## Components In Scope
- **Consensus (DLC + D-GBDT):** fairness model activation, shadow verifiers, and deterministic model hashing.
- **Networking / DHT:** libp2p/IPNDHT publish/find flows, descriptor validation, and peer resilience guards.
- **RPC/API:** authenticated/rate-limited surfaces for payments, handles, files, AI status, health/metrics.
- **Storage/State & HashTimer:** snapshot/replay pipelines, HashTimer ordering and skew controls, state persistence.

## Components Out of Scope
- **UI/Explorer surfaces** (apps/ui, explorer views) beyond verifying RPC contracts.
- **Advanced auth and key management** (API keys/JWT, HSM/remote signing) slated for later phases.
- **Long-run chaos/soak CI** and **extended no-float scans** across all crates (tracked for Phase 2).
- **ZK/STARK prototypes** and related cryptographic extensions that are not part of the current RC runtime.

## Notes for Auditors
- Feature development is paused for the selected commit; only documentation/tagging changes should occur before audit start.
- Please record the final commit hash above when the audit scope is frozen.
- Cross-reference `AUDIT_SUMMARY.md` and `SECURITY_THREAT_MODEL.md` for prior hardening context and Phase 2 deferrals.
- See `docs/SECURITY_AND_AUDIT_PLAYBOOK.md` for external audit inputs, disclosure channel, and baseline hardening guidance.
