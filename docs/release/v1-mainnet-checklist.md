# IPPAN v1.0 Mainnet Checklist

## 1. Overview

- Target version: v1.0.0
- Current status: Release Candidate (v0.9.x)
- This document defines what is REQUIRED for mainnet launch vs. what can be deferred.

## 2. Categories

Each item has:
- Status: ❌ Not started, 🟡 In progress, ✅ Done
- Priority:
  - BLOCKER (must be done for v1.0)
  - NICE-TO-HAVE (good before v1.0; can slip if needed)
  - POST-1.0 (explicitly after mainnet)

See also:
- Audit checklist: `CHECKLIST_AUDIT_MAIN.md`
- Feature mapping: `docs/feature-mapping.md`

## 3. Consensus & Core Protocol

- ❌ BLOCKER — DLC/D-GBDT consensus spec frozen and documented
- ❌ BLOCKER — Genesis configuration format & default profiles (testnet/mainnet)
- 🟡 NICE-TO-HAVE — Shadow verifier design/implementation (if not fully wired)
- ❌ POST-1.0 — ZK-STARK implementation (design exists; not required for v1.0)

See also:
- Consensus notes and mappings: `docs/feature-mapping.md`
- Audit checklist: `CHECKLIST_AUDIT_MAIN.md`

## 4. Economics & Emission

- ❌ BLOCKER — Capped supply + emission curve encoded and tested
- ❌ BLOCKER — Fee handling consistent with whitepaper (no burns if capped)
- 🟡 NICE-TO-HAVE — On-chain metrics/telemetry for emission + rewards
- ❌ POST-1.0 — Advanced dynamic economics / AI-tuned parameters

See also:
- Emission analysis: `docs/EMISSION_CURVE_ANALYSIS.md`
- Fees and emission overview: `docs/FEES_AND_EMISSION.md`
- Audit checklist: `CHECKLIST_AUDIT_MAIN.md`

## 5. Governance

- ❌ BLOCKER — Minimal governance mechanism or upgrade path (e.g. config-gated)
- 🟡 NICE-TO-HAVE — Documented governance roadmap (how IPPAN evolves)
- ❌ POST-1.0 — Full on-chain governance & voting

See also:
- Governance models: `GOVERNANCE_MODELS.md`, `docs/GOVERNANCE_MODELS.md`
- Feature mapping: `docs/feature-mapping.md`

## 6. Security

- ✅ BLOCKER — Threat model (RC-level) documented
- ✅ BLOCKER — Security Hardening Phase 1 tests (rate limit, whitelist, abuse)
- ❌ BLOCKER — External third-party security review (scope + plan agreed)
- 🟡 NICE-TO-HAVE — Recommended OS/network hardening guide (firewall, ulimits)

See also:
- Threat model: `docs/security/threat-model-rc.md`
- Hardening Phase 1: `docs/security/hardening-phase1.md`
- Audit checklist: `CHECKLIST_AUDIT_MAIN.md`

## 7. Testing & Observability

- ✅ BLOCKER — Comprehensive Testing Phases 1–2 (time, DLC, storage, RPC, P2P)
- ❌ BLOCKER — Long-duration testnet run (continuous for N days with logs)
- 🟡 NICE-TO-HAVE — Basic performance dashboards / metrics examples
- ❌ POST-1.0 — Fuzzing at scale / chaos testing

See also:
- Comprehensive testing Phase 1: `docs/testing/comprehensive-testing-phase1.md`
- Comprehensive testing Phase 2: `docs/testing/comprehensive-testing-phase2.md`
- Observability guide: `docs/OBSERVABILITY_GUIDE.md`
- Audit checklist: `CHECKLIST_AUDIT_MAIN.md`

## 8. Operations & Docs

- ❌ BLOCKER — “Run a mainnet-style node” guide (Prod-grade)
- ❌ BLOCKER — “Upgrade and rollback” operational doc
- 🟡 NICE-TO-HAVE — Example infra templates (systemd, docker-compose)
- ❌ POST-1.0 — Full SRE playbook

See also:
- Deployment guide: `docs/DEPLOYMENT_GUIDE.md`
- Release engineering: `docs/RELEASE_ENGINEERING.md`
- Operators docs: `docs/operators/`
- Audit checklist: `CHECKLIST_AUDIT_MAIN.md`

## 9. Launch Decision Criteria

- All BLOCKER items marked ✅
- No critical or high-severity bugs open
- External audit plan agreed and scheduled
- Long-duration soak test completed with acceptable metrics
- Release notes + upgrade guidance published
