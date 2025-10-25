# 🧠 Meta-Agent Protocol

> Governance layer for AI agent orchestration, task distribution, and conflict arbitration in the IPPAN project.

---

## 🎯 Core Principles

1. **Deterministic Task Assignment** — Each task maps to exactly one primary agent based on scope ownership
2. **Conflict Prevention** — Proactive coordination prevents overlapping edits
3. **Graceful Degradation** — System continues operating even when individual agents fail
4. **Audit Trail** — All decisions and overrides are logged and traceable

---

## 🏗️ Task Distribution Algorithm

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

## ⚔️ Conflict Resolution

### Conflict Types

| Type | Description | Resolution Strategy |
|------|-------------|-------------------|
| **File Overlap** | Multiple agents modify same files | Lock-based serialization |
| **API Breaking** | Changes break other crates | Cross-agent coordination required |
| **Dependency Cycle** | Circular dependencies introduced | MetaAgent breaks cycle |
| **Resource Contention** | Ports, database locks, etc. | Queue-based allocation |

### Resolution Process

1. **Detection** — CI detects conflicts during build/test
2. **Notification** — Affected agents receive conflict alerts
3. **Negotiation** — Agents attempt automatic resolution (5min timeout)
4. **Arbitration** — MetaAgent applies resolution rules
5. **Escalation** — Human maintainer for complex conflicts

---

## 🔄 Workflow States

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

## 📊 Monitoring & Metrics

### Key Performance Indicators
- **Task Completion Rate** — % of tasks completed without escalation
- **Conflict Rate** — % of tasks requiring conflict resolution
- **Agent Utilization** — % of time agents spend in ACTIVE state
- **Review Cycle Time** — Average time from submission to merge
- **Escalation Rate** — % of tasks requiring human intervention

### Health Checks
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

## 🚨 Escalation Procedures

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

## 🔧 Configuration

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

## 📝 Decision Log

### Recent Decisions
- **2024-12-19**: Implemented lock-based conflict resolution
- **2024-12-19**: Added automatic branch cleanup workflow
- **2024-12-19**: Established agent scope ownership matrix

### Pending Decisions
- Agent performance scoring algorithm
- Cross-repository coordination protocol
- Emergency override procedures

---

## 🔄 Continuous Improvement

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

_Protocol Version: 1.0_  
_Last Updated: 2024-12-19_  
_Next Review: 2024-12-26_