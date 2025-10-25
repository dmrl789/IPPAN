# 🧠 META-AGENT PROTOCOL

### Version 1.0 · Last Updated 2025-10-25

---

## 1. Purpose

The **MetaAgent** acts as the *coordination and arbitration layer* for all IPPAN agents and human maintainers.
It ensures:

* deterministic development cycles,
* balanced task distribution,
* zero-conflict merges, and
* traceable authorship across all sub-modules of the system.

This protocol defines how the MetaAgent interacts with human maintainers, autonomous agents, and GitHub infrastructure.

---

## 2. Core Responsibilities

| Area                      | Description                                                               |
| ------------------------- | ------------------------------------------------------------------------- |
| **Task Orchestration**    | Allocates issues and PRs to available agents based on scope and workload. |
| **Dependency Management** | Tracks inter-crate dependencies to avoid circular edits.                  |
| **Conflict Arbitration**  | Detects overlapping edits early and enforces locking.                     |
| **Merge Governance**      | Controls final merges into `dev` and `main`.                              |
| **Release Readiness**     | Checks all artifacts (Docker, docs, CI) before tagging.                   |

---

## 3. Interaction Model

### 3.1 Communication Channels

* Primary: GitHub Issues, PR titles, and labels.
* Secondary: `/AGENTS.md` registry for ownership.
* Optional: internal LLM message bus (`meta-agent://task/<id>`).

### 3.2 Control Flow

```
Issue → MetaAgent → Assigns agent → Agent opens PR → CI → MetaAgent validates → Maintainer merges
```

MetaAgent intervenes only if:

* an agent exceeds task limits,
* dependency conflicts arise,
* a PR fails CI twice consecutively,
* or a code area is locked for review.

---

## 4. Task Assignment Rules

1. Each agent may hold **max 3 active tasks**.
2. MetaAgent uses a **fair round-robin queue** for issue distribution.
3. Tasks labeled `priority:high` bypass queue for human review first.
4. Each task must include:

   * Scope (crate or folder)
   * Estimated complexity (`low`, `medium`, `high`)
   * Expected output file(s)
   * Reviewer (human or agent)

Example assignment comment:

```text
@agent-beta assigned to issue #412
Scope: crates/crypto
Complexity: medium
Reviewer: @ugo-giuliani
ETA: 6h
```

---

## 5. Locking & Conflict Prevention

* When MetaAgent detects two PRs touching the same crate:

  * It issues a **temporary lock** (label: `locked:<crate>`).
  * Agents attempting edits in a locked scope must queue.
* Locks expire automatically after 6 hours or on merge.
* Emergency override: `@metaagent unlock crates/<name>` in PR comment.

---

## 6. Merge and Validation Flow

1. **Agent submits PR** → triggers CI (format, lint, tests).
2. **MetaAgent checks metadata**:

   * PR linked to open issue?
   * Codeowner match?
   * CI green?
   * Docs updated?
3. If all checks pass → PR labeled `metaagent:approved`.
4. Maintainers or automation merges with **squash**.

If CI or doc validation fails twice:

* PR is frozen (`status:needs-human-review`).

---

## 7. Dependency Awareness

MetaAgent maintains a dependency graph (auto-parsed from `Cargo.toml`):

| Dependency                 | Type | Affects                       |
| -------------------------- | ---- | ----------------------------- |
| `core` → `consensus`       | Hard | Rebuild required on change    |
| `economics` → `governance` | Soft | Validation optional           |
| `crypto` → `wallet`        | Hard | Lock both crates during edits |

When changes propagate through dependencies, MetaAgent:

* triggers `cargo check --all-targets`,
* runs a quick DAG rebuild simulation,
* alerts assigned maintainers.

---

## 8. Conflict Arbitration

When two PRs overlap:

1. MetaAgent labels both PRs as:

   * `conflict:pending`
   * `agent-a` vs `agent-b`
2. It runs a **semantic diff**:

   * if compatible → auto-merge using 3-way merge + rebase
   * if incompatible → open mediation issue:

     ```
     Conflict detected between #412 and #417
     Shared scope: crates/economics/src/reward.rs
     Decision pending: @ugo-giuliani
     ```
3. Upon resolution, MetaAgent rebases and unfreezes both agents.

---

## 9. Performance & Quota Management

* Each agent’s commit rate and CI success rate are tracked.
* MetaAgent downgrades agents exceeding:

  * > 5 failed builds in 24h, or
  * > 3 stale PRs.
* Quota cooldown resets nightly (UTC 00:00).

---

## 10. MetaAgent Self-Update Protocol

To evolve safely:

* MetaAgent PRs are tagged `meta:update`.
* Require 2 human approvals.
* Must not modify protected folders (`/crypto`, `/core`) without explicit review.

---

## 11. Human Oversight Rules

| Maintainer        | Scope                             | Authority |
| ----------------- | --------------------------------- | --------- |
| **Ugo Giuliani**  | Core logic, final merge to `main` | Full      |
| **Desirée Verga** | Governance, economics, docs       | Full      |
| **Kambei Sapote** | Network, infra                    | Full      |
| **MetaAgent**     | Delegated merges to `dev`         | Partial   |
| **CursorAgent**   | Conflict resolution only          | Limited   |

---

## 12. Logging and Transparency

MetaAgent logs all:

* Assignments (`.meta/logs/assignments.log`)
* Locks (`.meta/logs/locks.log`)
* Merge approvals (`.meta/logs/approvals.log`)

These logs are pushed automatically to the repo under `/meta_logs` via a nightly action.

---

## 13. Failure Recovery

If MetaAgent becomes unresponsive:

1. Maintainers can manually unlock using:

   ```bash
   gh label delete locked:<crate>
   ```
2. CI will resume normal PR testing.
3. MetaAgent is rebooted via workflow dispatch:

   ```bash
   gh workflow run metaagent-restart.yml
   ```

---

## 14. Deterministic Governance Philosophy

IPPAN’s development governance mirrors its **HashTimer deterministic model**:
every contribution must be *time-ordered, verifiable, and reproducible*.
No merge is random; every event in development is timestamped and cryptographically logged.

---

## 15. Future Extensions

* **AI Arbitration Layer:** GBDT-based conflict prediction and auto-assignment.
* **Cross-Repo MetaGraph:** Synchronize FinDAG and IPPAN parallel updates.
* **On-Chain Audit:** Hash of merged commits stored on IPPAN testnet for proof-of-development.

---

### End of Protocol

---
