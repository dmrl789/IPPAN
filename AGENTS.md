# 🤖 IPPAN Active Agents & Scopes

> Registry of all autonomous and human contributors.  
> Each agent or maintainer **owns specific crates or domains** to prevent overlap.

---

## 🧠 Meta & System Agents

| Agent | Role | Scope | Maintainer |
|--------|------|--------|-------------|
| **MetaAgent** | Task orchestration, dependency graph, conflict arbitration | Global | Maintainers |
| **CursorAgent** | Local code edits, PR generation, conflict resolution | Per-task | Maintainers |
| **DocsAgent** | PRD, Whitepaper, and API documentation updates | `/docs` | Desiree Verga |
| **AuditAgent** | Security, reproducibility, cargo-deny, Trivy scans | Global | Ugo Giuliani |
| **CI-Agent** | Manages GitHub Actions & pipeline sync | `.github/`, workflows | Maintainers |

---

## 🧰 Core Development Agents

| Agent | Scope | Description | Maintainer |
|--------|--------|-------------|-------------|
| **Agent-Alpha** | `/crates/consensus`, `/crates/economics` | Round & reward logic | Ugo Giuliani |
| **Agent-Beta** | `/crates/core`, `/crates/crypto` | HashTimer, keys, serialization | Desiree Verga |
| **Agent-Gamma** | `/crates/network`, `/crates/p2p` | libp2p, NAT, relay, DHT | Kambei Sapote |
| **Agent-Delta** | `/crates/wallet`, `/crates/addressing` | Ed25519 wallet & domain registry | Desiree Verga |
| **Agent-Epsilon** | `/crates/governance`, `/crates/metrics` | Voting & validator scoring | Marco F. |
| **Agent-Zeta** | `/crates/ai_core`, `/crates/ai_registry` | GBDT, AI inference | MetaAgent |
| **Agent-Theta** | `/crates/explorer`, `/crates/api_gateway` | Warp API & GraphQL endpoints | Ugo Giuliani |
| **Agent-Lambda** | `/apps/ui`, `/apps/mobile` | Unified UI & Tauri frontend | Desiree Verga |
| **Agent-Sigma** | `/infra/docker`, `/infra/deploy` | Dockerfiles, GitHub Actions, CI | MetaAgent |
| **Agent-Omega** | `/tests`, `/benchmark` | Integration tests, TPS validation | Kambei Sapote |

---

## 🧑‍💻 Human Maintainers

| Name | Role | Permissions |
|------|------|-------------|
| **Ugo Giuliani** | Lead Architect | Merge to `main`, release management |
| **Desirée Verga** | Strategic Product Lead | Docs, roadmap, governance |
| **Kambei Sapote** | Network Engineer | P2P topology, infra |
| **Cursor Agent (autonomous)** | Automated PRs & merges | CI + Dev branches |

---

## 🧱 Rules of Engagement

1. **One crate per agent** — unless explicitly coordinated by MetaAgent.  
2. Agents must commit under their assigned names (`Co-authored-by` supported).  
3. Human maintainers can override agent assignments only via PR discussion.  
4. Agents with overlapping scopes must request lock from MetaAgent before commit.  
5. Nightly build validates ownership consistency.

---

_Last synchronized: {{DATE_AUTO_UPDATE}}_
