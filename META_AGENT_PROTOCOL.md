# 🧠 META-AGENT PROTOCOL

### Version 2.0 · Last Updated 2025-10-25

> Comprehensive governance layer for AI agent orchestration, task distribution, conflict arbitration, and deterministic development in the IPPAN project.

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

### 🎯 Core Principles

1. **Deterministic Task Assignment** — Each task maps to exactly one primary agent based on scope ownership
2. **Conflict Prevention** — Proactive coordination prevents overlapping edits
3. **Graceful Degradation** — System continues operating even when individual agents fail
4. **Audit Trail** — All decisions and overrides are logged and traceable

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

## 4. Task Distribution Algorithm

### Phase 1: Task Analysis
```yaml
Input: Pull Request or Issue
Process:
  1. Parse changed files and affected crates
  2. Identify scope owners from AGENTS.md
  3. Determine task complexity and dependencies
  4. Check for cross-crate impacts
```

### Phase 2: Agent Assignment

1. Each agent may hold **max 3 active tasks**.
2. MetaAgent uses a **fair round-robin queue** for issue distribution.
3. Tasks labeled `priority:high` bypass queue for human review first.
4. Each task must include:

   * Scope (crate or folder)
   * Estimated complexity (`low`, `medium`, `high`)
   * Expected output file(s)
   * Reviewer (human or agent)

**Assignment Strategy:**
```yaml
Primary Assignment:
  - Single scope owner → Direct assignment
  - Multiple scopes → MetaAgent coordinates
  - Cross-crate changes → Lead maintainer review required

Fallback Chain:
  1. Primary agent (from AGENTS.md)
  2. Backup agent (same maintainer)
  3. MetaAgent (escalation)
  4. Human maintainer (manual override)
```

Example assignment comment:

```text
@agent-beta assigned to issue #412
Scope: crates/crypto
Complexity: medium
Reviewer: @ugo-giuliani
ETA: 6h
```

### Phase 3: Lock Management

```yaml
Lock Types:
  - READ_LOCK: Agent can read but not modify
  - WRITE_LOCK: Exclusive write access
  - REVIEW_LOCK: Pending human review

Lock Resolution:
  - Timeout: 24h for WRITE_LOCK, 72h for REVIEW_LOCK
  - Priority: P0 > P1 > P2 (from issue labels)
  - Escalation: MetaAgent can force-release after timeout
```

---

## 5. Locking & Conflict Prevention

* When MetaAgent detects two PRs touching the same crate:

  * It issues a **temporary lock** (label: `locked:<crate>`).
  * Agents attempting edits in a locked scope must queue.
* Locks expire automatically after 6 hours on merge, 24h for write lock, 72h for review lock.
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

## 8. Conflict Resolution

### Conflict Types

| Type | Description | Resolution Strategy |
|------|-------------|-------------------|
| **File Overlap** | Multiple agents modify same files | Lock-based serialization |
| **API Breaking** | Changes break other crates | Cross-agent coordination required |
| **Dependency Cycle** | Circular dependencies introduced | MetaAgent breaks cycle |
| **Resource Contention** | Ports, database locks, etc. | Queue-based allocation |

### Resolution Process

When two PRs overlap:

1. **Detection** — MetaAgent labels both PRs as:
   * `conflict:pending`
   * `agent-a` vs `agent-b`

2. **Notification** — Affected agents receive conflict alerts

3. **Negotiation** — Agents attempt automatic resolution (5min timeout):
   * if compatible → auto-merge using 3-way merge + rebase
   * if incompatible → open mediation issue:

     ```
     Conflict detected between #412 and #417
     Shared scope: crates/economics/src/reward.rs
     Decision pending: @ugo-giuliani
     ```

4. **Arbitration** — MetaAgent applies resolution rules

5. **Escalation** — Upon resolution, MetaAgent rebases and unfreezes both agents

---

## 9. Workflow States

### Agent States
- **IDLE** — Available for new tasks
- **ASSIGNED** — Task received, preparing to work
- **ACTIVE** — Currently executing task
- **BLOCKED** — Waiting for dependencies or locks
- **REVIEW** — Changes submitted, awaiting review
- **COMPLETE** — Task finished successfully
- **FAILED** — Task failed, needs retry or escalation

### Task States
- **PENDING** — Queued for assignment
- **IN_PROGRESS** — Assigned to agent
- **REVIEW** — Human review required
- **MERGED** — Successfully integrated
- **REJECTED** — Failed review or conflicts
- **CANCELLED** — Task no longer needed

---

## 10. Performance & Quota Management

* Each agent's commit rate and CI success rate are tracked.
* MetaAgent downgrades agents exceeding:

  * > 5 failed builds in 24h, or
  * > 3 stale PRs.
* Quota cooldown resets nightly (UTC 00:00).

### Key Performance Indicators
- **Task Completion Rate** — % of tasks completed without escalation
- **Conflict Rate** — % of tasks requiring conflict resolution
- **Agent Utilization** — % of time agents spend in ACTIVE state
- **Review Cycle Time** — Average time from submission to merge
- **Escalation Rate** — % of tasks requiring human intervention

---

## 11. Monitoring & Health Checks

```yaml
Every 5 minutes:
  - Agent heartbeat validation
  - Lock timeout detection
  - Resource availability check
  - CI pipeline status

Every hour:
  - Performance metrics calculation
  - Conflict pattern analysis
  - Agent load balancing
  - Queue depth monitoring
```

---

## 12. Escalation Procedures

### Level 1: Agent Self-Resolution
- Retry failed operations (max 3 attempts)
- Request additional locks if needed
- Coordinate with other agents via comments

### Level 2: MetaAgent Intervention
- Force-release stale locks
- Reassign tasks to backup agents
- Apply conflict resolution rules
- Queue rebalancing

### Level 3: Human Maintainer
- Complex architectural decisions
- Security-related conflicts
- Performance degradation
- Agent malfunction

### Level 4: Emergency Override
- System-wide agent shutdown
- Manual task reassignment
- Emergency hotfix deployment
- Complete system reset

---

## 13. MetaAgent Self-Update Protocol

To evolve safely:

* MetaAgent PRs are tagged `meta:update`.
* Require 2 human approvals.
* Must not modify protected folders (`/crypto`, `/core`) without explicit review.

---

## 14. Human Oversight Rules

| Maintainer        | Scope                             | Authority |
| ----------------- | --------------------------------- | --------- |
| **Ugo Giuliani**  | Core logic, final merge to `main` | Full      |
| **Desirée Verga** | Governance, economics, docs       | Full      |
| **Kambei Sapote** | Network, infra                    | Full      |
| **MetaAgent**     | Delegated merges to `dev`         | Partial   |
| **CursorAgent**   | Conflict resolution only          | Limited   |

---

## 15. Configuration

### Agent Capabilities
```yaml
Agent-Alpha:
  max_concurrent_tasks: 2
  timeout_minutes: 60
  retry_attempts: 3
  escalation_threshold: 2

MetaAgent:
  max_concurrent_tasks: 10
  timeout_minutes: 120
  retry_attempts: 5
  escalation_threshold: 1
```

### Lock Policies
```yaml
WRITE_LOCK:
  default_timeout: 24h
  extension_allowed: true
  max_extensions: 2

REVIEW_LOCK:
  default_timeout: 72h
  extension_allowed: false
  auto_escalate: true
```

---

## 16. Logging and Transparency

MetaAgent logs all:

* Assignments (`.meta/logs/assignments.log`)
* Locks (`.meta/logs/locks.log`)
* Merge approvals (`.meta/logs/approvals.log`)

These logs are pushed automatically to the repo under `/meta_logs` via a nightly action.

---

## 17. Failure Recovery

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

## 18. Deterministic Governance Philosophy

IPPAN's development governance mirrors its **HashTimer deterministic model**:
every contribution must be *time-ordered, verifiable, and reproducible*.
No merge is random; every event in development is timestamped and cryptographically logged.

---

## 19. Decision Log

### Recent Decisions
- **2025-10-25**: Merged operational and governance protocol versions
- **2024-12-19**: Implemented lock-based conflict resolution
- **2024-12-19**: Added automatic branch cleanup workflow
- **2024-12-19**: Established agent scope ownership matrix

### Pending Decisions
- Agent performance scoring algorithm
- Cross-repository coordination protocol
- Emergency override procedures

---

## 20. Continuous Improvement

### Weekly Reviews
- Analyze conflict patterns and resolution effectiveness
- Review agent performance metrics
- Update assignment algorithms based on success rates
- Refine escalation thresholds

### Monthly Updates
- Update agent capabilities and timeouts
- Revise conflict resolution strategies
- Optimize task distribution algorithms
- Update documentation and protocols

---

## 21. Future Extensions

* **AI Arbitration Layer:** GBDT-based conflict prediction and auto-assignment.
* **Cross-Repo MetaGraph:** Synchronize FinDAG and IPPAN parallel updates.
* **On-Chain Audit:** Hash of merged commits stored on IPPAN testnet for proof-of-development.
* **Automated Performance Scoring:** Machine learning-based agent capability assessment.
* **Predictive Conflict Detection:** Proactive identification of potential merge conflicts.

---

### End of Protocol

_Protocol Version: 2.0_  
_Last Updated: 2025-10-25_  
_Next Review: 2025-11-01_
