# External Audit & Security Playbook

This document ties together the artifacts auditors and security reviewers need for IPPAN RC and gives operators a single place to start during incidents.

## External audit expectations
- **Scope**: protocol/consensus (DLC, HashTimer), cryptography primitives, RPC surface, and deterministic AI model handling.
- **Inputs**:
  - [`AUDIT_PACKAGE_V1_RC1_2025_11_24.md`](./AUDIT_PACKAGE_V1_RC1_2025_11_24.md) and `AUDIT_READY.md` for version/scope.
  - Deterministic model artifacts and hash script (`crates/ai_core/src/bin/verify_model_hash.rs`).
  - DLC simulation + fuzz/property tests referenced in `PHASE_E_STEP3_FUZZ_RESULTS.md`.
  - Location of fuzz/property tests and chaos plans noted in `CHECKLIST_AUDIT_MAIN.md`.
  - Network/ops posture references in `DEPLOYMENT_READY.md` and monitoring hooks in `docs/operators/monitoring-and-alerts.md`.
- **Expected outputs**: written findings with severity ratings, reproduction steps, and a remediation window per category (critical/high/medium/low). Patch validation runs in CI where applicable.

### Roles and responsibilities
- **Audit coordinator**: owns schedule, provides artifacts, tracks SLA adherence.
- **Domain leads (consensus, network, security, AI)**: supply design clarifications and mitigation owners.
- **Observers**: product/ops leads who receive read-only updates and support postmortems.

### Pre-audit readiness checklist
- `/version` response captured for the snapshot under review (protocol version + commit).
- Determinism gates (AI model hash verifier + DLC simulation) run and logs archived.
- Threat model and RPC hardening docs shared (`SECURITY_THREAT_MODEL.md`, `AUDIT_BUG_TRIAGE_WORKFLOW.md`).
- Communication channel confirmed (security advisory, encrypted email), with backups documented.

## Bug bounty / responsible disclosure
- Preferred channel: encrypted email to the security contacts listed in `AUDIT_READY.md`, or via the repository security advisory form.
- Acknowledge reports within **3 business days**, provide remediation ETA within **7 days** for high/critical issues.
- Maintain a private advisory thread until a fix is merged and `/version`-identifiable binaries are published.
- Researchers should avoid public disclosure until a coordinated fix and patch release is available; credits are published in release notes when desired.
- Out-of-scope examples: abandoned forks, unsupported third-party packages, or exploits requiring root on hardened hosts.

### Triage workflow (summary)
1. Confirm receipt and reproduce the issue; assign a severity (critical/high/medium/low/info) and DRI.
2. If consensus/runtime impacting, freeze releases and notify operators with the workaround and affected `/version` string.
3. Land minimal fixes with regression tests, re-run determinism + fuzz gates, and request reporter validation before disclosure.
4. Publish an advisory/changelog entry once patched binaries or configs are ready for operators.

## OS and network hardening (Phase 1 baseline)
- Follow `DEPLOYMENT_READY.md` and `PRODUCTION_DEPLOYMENT_GUIDE.md` for firewall defaults, reverse-proxy/TLS termination, and binding RPC to loopback where possible.
- Run nodes under dedicated users with least-privilege file permissions; avoid co-locating validators with internet-facing services.
- Terminate TLS at a hardened reverse proxy (e.g., Nginx) and prefer mTLS for validator RPC where available.
- Keep SSH locked down (key-only auth, fail2ban) and restrict RPC ingress via security groups or host firewalls.
- Monitor scrape gaps, forks, and RPC error rates via `grafana_dashboards/` to catch regressions quickly.
- Deeper OS sandboxing/hardening (SELinux/AppArmor, kernel tuning) is targeted for Phase 2; track follow-ups in `CHECKLIST_AUDIT_MAIN.md`.

## Incident coordination
- When a critical issue is reported, assign an owner, capture evidence (logs, metrics snapshots), and gate releases until mitigation is validated.
- Notify validators/operators with impact assessment and mitigations; include `/version` output so operators can verify patched binaries.
- After remediation, schedule a short postmortem and feed improvements back into runbooks and monitoring rules.
- Archive observability snapshots (Prometheus scrapes, health checks, AI hash verifier logs) alongside the advisory for reproducibility.
