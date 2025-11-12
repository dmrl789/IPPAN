# D-GBDT Rollout - Agent Assignment Summary

**Feature Branch:** `feat/d-gbdt-rollout` (created)  
**Status:** Ready for Phase 1  
**Total Phases:** 7

---

## 📋 Quick Reference

| Phase | Agent | Branch | Effort | Status |
|-------|--------|--------|--------|--------|
| 1 | Agent-Alpha | `phase1/deterministic-math` | 2-3 days | Ready |
| 2 | Agent-Alpha | `phase2/inference-engine` | 3-4 days | After Phase 1 |
| 3 | Agent-Theta | `phase3/model-registry` | 2-3 days | After Phase 2 |
| 4 | Agent-Alpha | `phase4/consensus-integration` | 3-4 days | After Phase 3 |
| 5 | Agent-Sigma | `phase5/ci-determinism` | 1-2 days | After Phase 4 |
| 6 | Agent-Zeta | `phase6/trainer-cli` | 2-3 days | After Phase 5 |
| 7 | DocsAgent | `phase7/docs` | 1-2 days | After Phase 6 |

**Total Timeline:** 14-21 days sequential, 7-10 days with safe parallelization

---

## 🤖 Agent Instructions

Each agent has a detailed instruction document in `.meta/agents/`:

1. **`AGENT_1_DETERMINISTIC_MATH.md`** - Fixed-point math foundation
2. **`AGENT_2_INFERENCE_ENGINE.md`** - GBDT inference refactor
3. **`AGENT_3_MODEL_REGISTRY.md`** - Canonical serialization & hashing
4. **`AGENT_4_CONSENSUS_INTEGRATION.md`** - DLC consensus integration
5. **`AGENT_5_CI_DETERMINISM.md`** - Cross-arch CI enforcement
6. **`AGENT_6_TRAINER_CLI.md`** - Training CLI & quantization
7. **`AGENT_7_DOCUMENTATION.md`** - Docs & migration guide

---

## 🚀 How to Assign Agents

### Option 1: Sequential (Recommended)

Start with Phase 1 and proceed sequentially:

```bash
# Human orchestrator or MetaAgent:
# 1. Assign Agent-Alpha to Phase 1
gh issue create \
  --title "D-GBDT Phase 1: Deterministic Math Foundation" \
  --body "$(cat .meta/agents/AGENT_1_DETERMINISTIC_MATH.md)" \
  --label "agent-alpha,p0,d-gbdt-rollout" \
  --assignee <agent-alpha-handle>

# 2. Wait for Phase 1 PR to merge
# 3. Assign Agent-Alpha to Phase 2
# ... and so on
```

### Option 2: Parallel (Advanced)

Phases 2-4 can be partially parallelized if needed:

**Parallel Group A (after Phase 1):**
- Phase 2: Inference Engine (Agent-Alpha)
- Phase 3: Model Registry (Agent-Theta) - can start early with stubs

**Parallel Group B (after Phase 4):**
- Phase 5: CI (Agent-Sigma)
- Phase 6: Training CLI (Agent-Zeta) - can start with quantization module

**Sequential:**
- Phase 7: Documentation (DocsAgent) - must be last

---

## 📌 Assignment Template

Use this template for each GitHub Issue:

```markdown
## Phase X: [Title]

**Agent:** [Agent Name]  
**Branch:** `phaseX/[branch-name]`  
**Depends On:** Phase [X-1]  
**Estimated Effort:** [X-Y] days

### Objective
[Brief description]

### Tasks
- [ ] Task 1
- [ ] Task 2
- [ ] ...

### Acceptance Gates
- [ ] Float check passes
- [ ] All tests pass
- [ ] CI green
- [ ] [Phase-specific gates]

### Full Instructions
See: `.meta/agents/AGENT_X_[name].md`

### Command to Start
\`\`\`bash
cd /workspace
git checkout feat/d-gbdt-rollout
git pull origin feat/d-gbdt-rollout
git checkout -b phaseX/[branch-name]
\`\`\`

---

**When complete:** Create PR to `feat/d-gbdt-rollout` using instructions in agent document.
```

---

## 🔐 Access Control

**Who can assign agents?**
- Maintainers (Ugo Giuliani, Desirée Verga)
- MetaAgent (automated governance)
- Lead Architect approval required for Phase 4+ (consensus-critical)

**Who can merge phase PRs?**
- Require 1 human approval
- CI must be green
- Float detection must pass (Phase 5+)

**Who can merge feat/d-gbdt-rollout to main?**
- Require 2 maintainer approvals
- All 7 phases complete
- Full test suite green
- Performance benchmarks acceptable (<10% regression)

---

## 📊 Progress Tracking

Use GitHub Projects or this checklist:

### Phase 1: Deterministic Math
- [ ] Issue created & assigned
- [ ] Branch created
- [ ] Development in progress
- [ ] PR created
- [ ] Review approved
- [ ] CI green
- [ ] Merged to feat/d-gbdt-rollout

### Phase 2: Inference Engine
- [ ] Issue created & assigned
- [ ] Branch created
- [ ] Development in progress
- [ ] PR created
- [ ] Review approved
- [ ] CI green
- [ ] Merged to feat/d-gbdt-rollout

### Phase 3: Model Registry
- [ ] Issue created & assigned
- [ ] Branch created
- [ ] Development in progress
- [ ] PR created
- [ ] Review approved
- [ ] CI green
- [ ] Merged to feat/d-gbdt-rollout

### Phase 4: Consensus Integration
- [ ] Issue created & assigned
- [ ] Branch created
- [ ] Development in progress
- [ ] PR created
- [ ] Review approved
- [ ] CI green
- [ ] Merged to feat/d-gbdt-rollout

### Phase 5: CI Determinism
- [ ] Issue created & assigned
- [ ] Branch created
- [ ] Development in progress
- [ ] PR created
- [ ] Review approved
- [ ] CI green
- [ ] Merged to feat/d-gbdt-rollout

### Phase 6: Training CLI
- [ ] Issue created & assigned
- [ ] Branch created
- [ ] Development in progress
- [ ] PR created
- [ ] Review approved
- [ ] CI green
- [ ] Merged to feat/d-gbdt-rollout

### Phase 7: Documentation
- [ ] Issue created & assigned
- [ ] Branch created
- [ ] Development in progress
- [ ] PR created
- [ ] Review approved
- [ ] CI green
- [ ] Merged to feat/d-gbdt-rollout

### Final Merge
- [ ] All 7 phases complete
- [ ] Integration tests pass
- [ ] Performance benchmarks acceptable
- [ ] Documentation complete
- [ ] 2 maintainer approvals
- [ ] Merged to main

---

## 🆘 Escalation

**If a phase is blocked:**
1. Comment in phase PR with `@metaagent` tag
2. MetaAgent will analyze and suggest resolution
3. If needed, escalate to maintainers

**If acceptance gates fail:**
1. Agent must fix issues before requesting review
2. Maximum 3 iterations per phase
3. If still failing, escalate to lead agent for that domain

**If consensus disagreement:**
1. MetaAgent arbitrates technical decisions
2. Maintainers arbitrate scope/priority changes
3. Security/consensus changes require 2 maintainer approvals

---

## 📞 Contact

- **Orchestrator:** MetaAgent (this document)
- **Technical Lead:** Agent-Alpha (Ugo Giuliani)
- **Strategic Oversight:** Desirée Verga
- **Network/Infra:** Agent-Sigma (automated)
- **Documentation:** DocsAgent (automated)

---

**Created:** 2025-11-12  
**Last Updated:** 2025-11-12  
**Next Review:** After Phase 1 completion
