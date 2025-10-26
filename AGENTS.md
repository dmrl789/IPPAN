# 🤖 IPPAN Active Agents & Scopes

> Registry of all autonomous and human contributors.  
> Each agent or maintainer **owns specific crates or domains** to prevent overlap.

---

## 🧠 Meta & System Agents

| Agent | Role | Scope | Maintainer |
|--------|------|--------|-------------|
| **MetaAgent** | Task orchestration, dependency graph, conflict arbitration | Global | Maintainers |
| **CursorAgent** | Local code edits, PR generation, conflict resolution | Per-task | Maintainers |
| **DocsAgent** | PRD, Whitepaper, and API documentation updates | `/docs` | Desirée Verga |
| **AuditAgent** | Security, reproducibility, cargo-deny, Trivy scans | Global | Ugo Giuliani |
| **CI-Agent** | Manages GitHub Actions & pipeline sync | `.github/`, workflows | Maintainers |

---

## 🧰 Core Development Agents

| Agent | Scope | Description | Maintainer |
|--------|--------|-------------|-------------|
| **Agent-Alpha** | `/crates/consensus`, `/crates/economics` | Round & reward logic | Ugo Giuliani |
| **Agent-Beta** | `/crates/core`, `/crates/crypto` | HashTimer, keys, serialization | Desirée Verga |
| **Agent-Gamma** | `/crates/network`, `/crates/p2p` | libp2p, NAT, relay, DHT | Kambei Sapote |
| **Agent-Delta** | `/crates/wallet`, `/crates/addressing` | Ed25519 wallet & domain registry | Desirée Verga |
| **Agent-Epsilon** | `/crates/governance`, `/crates/metrics` | Voting & validator scoring | Marco F. |
| **Agent-Zeta** | `/crates/ai_core`, `/crates/ai_registry` | GBDT, AI inference | MetaAgent |
| **Agent-Theta** | `/crates/explorer`, `/crates/api_gateway` | Warp API & GraphQL endpoints | Ugo Giuliani |
| **Agent-Lambda** | `/apps/ui`, `/apps/mobile` | Unified UI & Tauri frontend | Desirée Verga |
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
2. **Agent handoffs** — use `@agent-name` in PR comments to transfer ownership.  
3. **Conflict resolution** — MetaAgent arbitrates when multiple agents claim the same scope.  
4. **Scope changes** — require a PR with `@metaagent` approval.  
5. **Maintainer overrides** — can reassign agents or scopes at any time.

---

## 🔄 Workflow Automation Agents

| Agent | Handle | Purpose | Triggers | Outputs |
|--------|--------|---------|----------|---------|
| **PRD Architect** | `@prd-architect` | Turn ideas/issues into specs & acceptance criteria | `label:needs-prd`, `/draft-prd` | `docs/prd/<topic>.md` + issue tasks |
| **Codex (Dev)** | `@codex` | Generate/modify code, resolve conflicts, scaffold files | `label:codex`, `/codex` | commits/PRs, patches, fix branches |
| **TestSprite** | `@testsprite` | Author tests, raise coverage, smoke suites | `label:tests`, `/add-tests` | test files, coverage report |
| **SecurityBot** | `@sec-bot` | SAST/dependency scan, threat notes, patches | `label:security`, `/security-scan` | alerts, PRs, advisories |
| **InfraBot** | `@infra-bot` | CI/CD, runners, Docker, ports, Nginx | `label:infra`, `/deploy`, `/restart` | workflow runs, deploy logs |
| **ReleaseBot** | `@release-bot` | Versioning, changelogs, GitHub releases | `label:release`, `/cut-release` | tags, release notes, SBOMs |
| **DocsBot** | `@docs-bot` | Sync README/architecture/CLI help | `label:docs`, `/sync-docs` | updated docs, TOC, link checks |
| **UI/UX Coach** | `@ui-coach` | Improve Unified UI layout & mobile flows | `label:ui-ux`, `/ux-review` | Figma notes, Tailwind diffs |
| **Gateway SRE** | `@gw-sre` | Validate gateway/WS health, CORS, envs | `label:gateway`, `/gateway-check` | health reports, `.env` upserts |
| **Licensing/Legal** | `@legal` | License headers, notices, patent refs | `label:legal`, `/audit-licenses` | headers, NOTICE, SPDX fixes |

> Agents are invoked via labels or slash commands in PR/Issue comments. Humans remain DRIs (Directly Responsible Individuals).

---

## 🏷️ Labels & Slash Commands

### Canonical Labels

* **Work type:** `codex`, `tests`, `infra`, `docs`, `security`, `ui-ux`, `gateway`, `legal`, `metaagent`
* **State:** `needs-prd`, `needs-review`, `ready-to-merge`, `blocked`, `backport`
* **Risk:** `safe`, `medium-risk`, `high-risk`
* **Priority:** `p0`, `p1`, `p2`
* **Agent assignments:** `agent-alpha`, `agent-beta`, `agent-gamma`, `agent-delta`, `agent-epsilon`, `agent-zeta`, `agent-theta`, `agent-lambda`
* **MetaAgent system:** `metaagent:approved`, `locked`, `conflict:pending`

### Slash Commands

* `/draft-prd <title>` — PRD Architect creates a spec in `docs/prd/`
* `/codex plan` or `/codex apply` — Codex plans or implements code changes
* `/add-tests [path]` — TestSprite adds unit/integration tests
* `/security-scan` — SecurityBot runs dependency audit
* `/deploy <env>` — InfraBot deploys (`dev|staging|prod`)
* `/restart <service>` — InfraBot restarts specific container/service
* `/cut-release <scope>` — ReleaseBot bumps version, creates release notes
* `/sync-docs` — DocsBot updates documentation
* `/ux-review` — UI/UX Coach performs design audit
* `/gateway-check` — Gateway SRE verifies endpoints & env vars
* `/metaagent` — MetaAgent runs governance & assignment logic

---

## 🌿 Branching & Merge Policy

* **Default branch:** `main`
* **Working branches:** `feature/*`, `fix/*`, `codex/*`, `hotfix/*`
* **Release branches:** `release/vX.Y`
* **Backports:** `maintenance/vX.(Y-1)`

**Merge gates:**
1. CI green (build, tests, lint, format)
2. ≥1 human approval
3. InfraBot preflight ok if infra changed
4. SecurityBot ok if security label present

**Hotfix:** allowed with `p0` + `hotfix/*`, requires InfraBot auto-deploy + rollback.

---

## 🔧 CI/CD Hand-off Summary

* **Codex:** code format, lint, test pass  
* **TestSprite:** adds ≥80% test coverage for touched crates  
* **SecurityBot:** `cargo deny`, `npm audit` (no critical issues)  
* **InfraBot:** YAML lint, ports, `.env` sanity, deploy checks  
* **ReleaseBot:** semantic version bump, changelog, SBOM  

---

## 🌍 Environments & Secrets

* **Dev:** permissive; local envs allowed  
* **Staging:** mirrors prod topology  
* **Prod:** restricted; only bots with approved workflows deploy  

**Secrets policy:**  
No secrets in code or comments. Use GitHub secrets or environment variables.

---

## 📁 File Ownership Map

| Path | Owner | Description |
|------|--------|-------------|
| `crates/**` | Codex, TestSprite, SecurityBot | Rust logic, HashTimer, consensus |
| `deploy/**` | InfraBot | Compose, systemd, Nginx |
| `docs/**` | PRD Architect, DocsBot | PRDs, ADRs, READMEs |
| `unified-ui/**` | Codex, UI/UX Coach | Next.js UI, mobile flows |
| `gateway/**` | Gateway SRE, InfraBot | API/WS endpoints |
| `.github/workflows/**` | InfraBot, SecurityBot | CI/CD, policies |

---

## 🧠 MetaAgent Governance System

MetaAgent automates governance:  
- Assigns agents  
- Manages locks  
- Detects conflicts  
- Validates merges  
- Commits logs to `.meta/logs/`

### Key Files
* `.github/workflows/metaagent-governance.yml`
* `.meta/logs/assignments.log`, `locks.log`, `approvals.log`

### Triggers
* Issues: `opened`, `labeled`
* PRs: `opened`, `synchronize`, `closed`
* Scheduled: hourly
* Manual: `workflow_dispatch`

---

_Last synchronized: 2025-10-26 · Maintainers: Ugo Giuliani, Desirée Verga_
