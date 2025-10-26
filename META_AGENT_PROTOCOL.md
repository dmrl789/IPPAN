# 🧠 META-AGENT PROTOCOL  
### Version 2.0 · Last Updated 2025-10-25  

> Comprehensive governance layer for AI-agent orchestration, task distribution, conflict arbitration, and deterministic development in the IPPAN project.

---

## 1 · Purpose  

The **MetaAgent** acts as the *coordination and arbitration layer* for all IPPAN agents and human maintainers.  
It ensures:

* deterministic development cycles  
* balanced task distribution  
* zero-conflict merges  
* traceable authorship across all sub-modules  

This protocol defines how the MetaAgent interacts with human maintainers, autonomous agents, and GitHub infrastructure.

---

## 2 · Core Responsibilities  

| Area | Description |
| -- | -- |
| **Task Orchestration** | Allocates issues and PRs to agents based on scope and workload |
| **Dependency Management** | Tracks inter-crate dependencies to avoid circular edits |
| **Conflict Arbitration** | Detects overlapping edits and enforces locking |
| **Merge Governance** | Controls final merges into `dev` and `main` |
| **Release Readiness** | Checks Docker, docs and CI artifacts before tagging |

### Core Principles
1. Deterministic Task Assignment — one primary agent per scope  
2. Conflict Prevention — proactive coordination prevents overlap  
3. Graceful Degradation — operation continues if agents fail  
4. Audit Trail — all decisions and overrides are logged  

---

## 3 · Interaction Model  

### 3.1 Communication Channels
* Primary → GitHub Issues, PR titles and labels  
* Secondary → `/AGENTS.md` registry for ownership  
* Optional → internal LLM bus `meta-agent://task/<id>`

### 3.2 Control Flow
