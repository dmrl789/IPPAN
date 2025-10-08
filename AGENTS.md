# AGENTS.md

> Operating handbook for the IPPAN project’s automation and helper agents (human + AI + CI).
> This file explains **who does what**, **how to invoke them**, **which labels/commands they react to**, and **the guardrails**.

---

## 0) Scope & Principles

* **Repositories:** `dmrl789/IPPAN` (monorepo: Rust crates, deploy, docs), `ippan.org` / Unified UI, gateway, infra manifests.
* **Main goals:** ship a high-throughput L1 (IPPAN) + production-grade UI/gateway with deterministic HashTimer ordering.
* **Rules of engagement**

  1. **Determinism > speed.** Reproducible builds, locked toolchains, pinned Docker images.
  2. **Automate the boring.** PR hygiene, checks, formatting, security scans, and deploys are agent tasks by default.
  3. **Fail safe.** If in doubt, agents abort with actionable logs and a rollback plan.
  4. **No secrets in code.** Ever. Agents read from CI secrets or server `.env` files only.

---

## 1) Agent Roster

| Agent               | Handle           | Purpose                                                 | Triggers                             | Outputs                                  |
| ------------------- | ---------------- | ------------------------------------------------------- | ------------------------------------ | ---------------------------------------- |
| **PRD Architect**   | `@prd-architect` | Turn ideas/issues into specs & acceptance criteria      | `label:needs-prd` or `/draft-prd`    | `docs/prd/<topic>.md` + issue tasks      |
| **Codex (Dev)**     | `@codex`         | Generate/modify code, resolve conflicts, scaffold files | `label:codex`, `/codex`              | commits/PRs, patches, fix branches       |
| **TestSprite**      | `@testsprite`    | Author tests, raise coverage, create smoke suites       | `label:tests`, `/add-tests`          | test files, coverage report              |
| **SecurityBot**     | `@sec-bot`       | SAST/deps scan, threat notes, quick patches             | `label:security`, `/security-scan`   | alerts, PRs, advisories                  |
| **InfraBot**        | `@infra-bot`     | CI/CD, runners, Docker, ports, systemd, Nginx           | `label:infra`, `/deploy`, `/restart` | workflow runs, deploy logs               |
| **ReleaseBot**      | `@release-bot`   | Versioning, changelogs, tags, GitHub Releases           | `label:release`, `/cut-release`      | tags, release notes, SBOMs               |
| **DocsBot**         | `@docs-bot`      | Sync README/architecture/CLI help                       | `label:docs`, `/sync-docs`           | updated docs, TOC, link checks           |
| **UI/UX Coach**     | `@ui-coach`      | Improve Unified UI layout, mobile flows                 | `label:ui-ux`, `/ux-review`          | diffs, Figma notes, Tailwind suggestions |
| **Gateway SRE**     | `@gw-sre`        | Validate gateway/WS health, CORS, envs                  | `label:gateway`, `/gateway-check`    | health reports, `.env` upserts           |
| **Licensing/Legal** | `@legal`         | License headers, notices, patent refs                   | `label:legal`, `/audit-licenses`     | headers, NOTICE, SPDX fixes              |

> All agents are invoked via labels **or** slash commands in PR/Issue comments. Humans remain DRIs (directly responsible individuals).

---

## 2) Labels & Slash Commands

### Canonical Labels

* **Work type:** `codex`, `tests`, `infra`, `docs`, `security`, `ui-ux`, `gateway`, `legal`
* **State:** `needs-prd`, `needs-review`, `ready-to-merge`, `blocked`, `backport`
* **Risk:** `safe`, `medium-risk`, `high-risk`
* **Priority:** `p0`, `p1`, `p2`

### Slash Commands (comment in PR/Issue)

* `/draft-prd <title>` → PRD Architect creates `docs/prd/<slug>.md` with acceptance criteria.
* `/codex plan` or `/codex apply` → Codex posts plan or opens a “codex/*” branch with changes.
* `/add-tests [path]` → TestSprite adds unit/integration tests.
* `/security-scan` → SecurityBot runs SAST/deps audit, opens remediation PR if trivial.
* `/deploy <env>` (dev|staging|prod) → InfraBot runs the appropriate workflow; posts links/logs.
* `/restart ui|gateway` → InfraBot restarts the relevant service on target env.
* `/cut-release <scope>` → ReleaseBot bumps semver, composes CHANGELOG, creates tag.
* `/sync-docs` → DocsBot syncs CLI help and architecture diagrams.
* `/ux-review` → UI/UX Coach posts a punch list for mobile layout & accessibility.
* `/gateway-check` → Gateway SRE verifies `/health`, WS, CORS, and `.env`.
* `/backport vX.Y` → ReleaseBot/InfraBot open backport PR to maintenance branch.

> Commands require write/admin permissions in the repo. Agents will fail gracefully if permissions/secrets are missing.

---

## 3) Branching & Merge Policy

* **Default branch:** `main`
* **Working branches:** `feature/<topic>`, `fix/<topic>`, `codex/<topic>`, `hotfix/<topic>`
* **Release branches:** `release/vX.Y`
* **Backports:** `maintenance/vX.(Y-1)`

**Merge gates (must pass):**

1. CI green (build, tests, lint, format, security scan)
2. One human reviewer approval
3. If `infra`/`deploy` touched: InfraBot “preflight ok”
4. If `security` label: SecurityBot “ok” or waiver by DRI

**Hotfix:** allowed with `p0` + `hotfix/*`, requires InfraBot auto-deploy + rollback ready.

---

## 4) CI/CD Hand-off (what each agent checks)

* **Codex**

  * Runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test -q`
  * UI: `npm ci && npm run build` (no audit blockers at critical/high)
  * Provides patch if failing.

* **TestSprite**

  * Adds tests; targets ≥ **80%** coverage for touched crates/dirs.

* **SecurityBot**

  * `cargo deny`, dependency audit; NPM `npm audit` (no **critical** allowed)
  * Adds `SECURITY.md` updates when needed.

* **InfraBot**

  * Validates workflow YAML, checks for non-ASCII whitespace.
  * Verifies ports aren’t already bound, ensures `.env` keys present.
  * Deploys with health checks & idempotent restarts.

* **ReleaseBot**

  * Enforces Conventional Commits → semantic version bump.
  * Generates release notes + SBOM (CycloneDX) if configured.

---

## 5) Environments & Secrets

* **Dev:** ephemeral or single host; permissive CORS; can use defaults.
* **Staging:** mirrors prod topology; smoke tests must pass.
* **Prod:** locked versions; only ReleaseBot/InfraBot deploy via approved workflows.

**Secrets policy**

* No secrets in code or PR bodies.
* Required CI secrets (examples):

  * `DEPLOY_HOST`, `DEPLOY_PORT`, `DEPLOY_USER`, `DEPLOY_SSH_KEY`, `DEPLOY_FINGERPRINT`
  * `NEXT_PUBLIC_*` (public build-time) should still come from **repo variables** or **env files**, never hardcoded.
  * Server keeps `.env` managed by InfraBot.

---

## 6) File & Ownership Map

| Path                   | Owner agent(s)                 | Notes                                      |
| ---------------------- | ------------------------------ | ------------------------------------------ |
| `crates/**`            | Codex, TestSprite, SecurityBot | Rust code, HashTimer, consensus, types     |
| `deploy/**`            | InfraBot                       | Compose/K8s, scripts, Nginx, service files |
| `docs/**`              | PRD Architect, DocsBot         | PRDs, ADRs, READMEs                        |
| `unified-ui/**`        | Codex, UI/UX Coach, DocsBot    | Next.js/Tailwind, mobile flows             |
| `gateway/**`           | Gateway SRE, InfraBot          | WS/API health, CORS, `.env`                |
| `.github/workflows/**` | InfraBot, SecurityBot          | CI/CD, policy gates                        |

> Changes to `deploy/**` or workflows always ping **InfraBot** and require its preflight.

---

## 7) Playbooks

### 7.1 Merge Conflict (common in `index.html`, lockfiles)

1. Author comments `/codex plan` → Codex proposes resolution strategy.
2. Codex rebases `codex/<topic>` onto `main`, resolves conflict, force-pushes.
3. TestSprite re-runs tests; DocsBot syncs READMEs if templates changed.

### 7.2 “Short menu” in Unified UI

1. `/gateway-check` → SRE posts `.env` keys detected.
2. If missing, InfraBot updates server `.env` with:

   ```
   NEXT_PUBLIC_ENABLE_FULL_UI=1
   NEXT_PUBLIC_GATEWAY_URL=https://ui.ippan.org/api
   NEXT_PUBLIC_API_BASE_URL=https://ui.ippan.org/api
   NEXT_PUBLIC_WS_URL=wss://ui.ippan.org/ws
   ```
3. `/restart ui` then verify `/api` and `/ws` in browser network tab.

### 7.3 Port conflict on deploy

1. InfraBot runs pre-deploy hook to free port or remap port.
2. If still failing, marks PR `blocked` with the conflicting PID and a suggested patch to compose file.

### 7.4 YAML invalid in workflows

1. InfraBot lints YAML; replaces non-breaking spaces; re-indents.
2. Posts fixed diff; blocks merge until fixed file is committed.

### 7.5 Hotfix & rollback

1. Label `p0` and comment `/deploy prod`.
2. InfraBot snapshots previous images and `.env`.
3. If post-deploy health fails, auto-rollback and attach logs to PR.

---

## 8) PR Ready Checklist (agents enforce)

* [ ] Conventional Commit(s) in PR title/squash (`feat:`, `fix:`, `chore:` …)
* [ ] Code formatted & lint-clean (`cargo fmt`, `clippy`, ESLint if UI)
* [ ] Tests added/updated; coverage ≥ 80% on touched code
* [ ] Security scan shows no **critical** issues (Rust/NPM)
* [ ] Docs updated (`README`, `docs/prd/*`, ADR if architectural)
* [ ] If deploy/infra touched → InfraBot preflight ✅
* [ ] No secrets present in diffs, logs, or comments

---

## 9) How to Ask Agents for Help (Examples)

* Spec drafting

  ```text
  /draft-prd IPPAN Time: client SDK
  ```
* Code change

  ```text
  /codex plan
  Goal: add ws keepalive; ensure reconnection backoff; expose /health in gateway.
  ```
* Tests

  ```text
  /add-tests crates/types/src/hashtimer.rs
  ```
* Security

  ```text
  /security-scan
  ```
* Deploy & health

  ```text
  /deploy staging
  /gateway-check
  /restart gateway
  ```

---

## 10) Documentation Conventions

* **PRDs:** `docs/prd/<slug>.md` → problem, scope, non-goals, acceptance criteria, telemetry.
* **ADRs:** `docs/adr/NNNN-<title>.md` → decision, context, alternatives.
* **Diagrams:** in `docs/diagrams/` (source + exported PNG/SVG).
* **CHANGELOG:** generated by ReleaseBot; manual edits discouraged.

---

## 11) Guardrails & Escalation

* Agents never merge failing CI or bypass reviews.
* If multiple agents disagree (e.g., SecurityBot vs Codex), the **DRI** decides, recorded in the PR discussion.
* **Escalation path:** DRI → repo maintainer(s) → org admin.

---

## 12) Roadmap Hooks (optional)

These labels map to quarterly planning:

* `q4-2025:consensus`, `q4-2025:gateway`, `q4-2025:ui`, `q4-2025:infra`, `q4-2025:security`

Agents will prioritize items carrying the active quarter label.

---

## 13) Appendix — Minimal `.env` for UI/Gateway Hosts

```
NEXT_PUBLIC_ENABLE_FULL_UI=1
NEXT_PUBLIC_GATEWAY_URL=https://ui.ippan.org/api
NEXT_PUBLIC_API_BASE_URL=https://ui.ippan.org/api
NEXT_PUBLIC_WS_URL=wss://ui.ippan.org/ws
GATEWAY_ALLOWED_ORIGINS=https://ui.ippan.org
```

---

**Maintainers:** Update this file whenever labels, commands, or CI gates change so people and agents stay in sync.
