# AGENTS.md

> Operating handbook for the IPPAN project’s automation and helper agents (human + AI + CI).
> This file explains **who does what**, **how to invoke them**, **which labels/commands they react to**, and the **guardrails**.

---

## 0) Scope & Principles

* **Repositories:** `dmrl789/IPPAN` (monorepo: Rust crates, deploy, docs), Unified UI, gateway, infra manifests.
* **Main goals:** ship a high-throughput L1 (IPPAN) + production-grade UI/gateway with deterministic HashTimer ordering.
* **Rules of engagement**

  1. **Determinism > speed.** Reproducible builds, locked toolchains, pinned Docker images.
  2. **Automate the boring.** PR hygiene, checks, formatting, security scans, and deploys are agent tasks by default.
  3. **Fail safe.** If in doubt, agents abort with actionable logs and a rollback plan.
  4. **No secrets in code.** Agents read from CI secrets or server `.env` files only.

---

## 1) Agent Roster

| Agent               | Handle           | Purpose                                                 | Triggers                             | Outputs                                  |
| ------------------- | ---------------- | ------------------------------------------------------- | ------------------------------------ | ---------------------------------------- |
| **PRD Architect**   | `@prd-architect` | Turn ideas/issues into specs & acceptance criteria      | `label:needs-prd` or `/draft-prd`    | `docs/prd/<topic>.md` + issue tasks      |
| **Codex (Dev)**     | `@codex`         | Generate/modify code, resolve conflicts, scaffold files | `label:codex`, `/codex`              | commits/PRs, patches, fix branches       |
| **TestSprite**      | `@testsprite`    | Author tests, raise coverage, smoke suites              | `label:tests`, `/add-tests`          | test files, coverage report              |
| **SecurityBot**     | `@sec-bot`       | SAST/deps scan, threat notes, quick patches             | `label:security`, `/security-scan`   | alerts, PRs, advisories                  |
| **InfraBot**        | `@infra-bot`     | CI/CD, runners, Docker, ports, systemd, Nginx           | `label:infra`, `/deploy`, `/restart` | workflow runs, deploy logs               |
| **ReleaseBot**      | `@release-bot`   | Versioning, changelogs, tags, GitHub Releases           | `label:release`, `/cut-release`      | tags, release notes, SBOMs               |
| **DocsBot**         | `@docs-bot`      | Sync README/architecture/CLI help                       | `label:docs`, `/sync-docs`           | updated docs, TOC, link checks           |
| **UI/UX Coach**     | `@ui-coach`      | Improve Unified UI layout, mobile flows                 | `label:ui-ux`, `/ux-review`          | diffs, Figma notes, Tailwind suggestions |
| **Gateway SRE**     | `@gw-sre`        | Validate gateway/WS health, CORS, envs                  | `label:gateway`, `/gateway-check`    | health reports, `.env` upserts           |
| **Licensing/Legal** | `@legal`         | License headers, notices, patent refs                   | `label:legal`, `/audit-licenses`     | headers, NOTICE, SPDX fixes              |

> Agents are invoked via labels **or** slash commands in PR/Issue comments. Humans remain DRIs (directly responsible individuals).

---

## 2) Labels & Slash Commands

### Canonical Labels

* **Work type:** `codex`, `tests`, `infra`, `docs`, `security`, `ui-ux`, `gateway`, `legal`
* **State:** `needs-prd`, `needs-review`, `ready-to-merge`, `blocked`, `backport`
* **Risk:** `safe`, `medium-risk`, `high-risk`
* **Priority:** `p0`, `p1`, `p2`

### Slash Commands (comment in PR/Issue)

* `/draft-prd <title>` → PRD Architect creates `docs/prd/<slug>.md` w/ acceptance criteria.
* `/codex plan` or `/codex apply` → Codex posts plan or opens a `codex/*` branch with changes.
* `/add-tests [path]` → TestSprite adds unit/integration tests.
* `/security-scan` → SecurityBot runs SAST/deps audit, opens remediation PR if trivial.
* `/deploy <env>` (dev|staging|prod) → InfraBot runs the appropriate workflow; posts links/logs.
* `/restart ui|gateway` → InfraBot restarts the selected service.
* `/cut-release <scope>` → ReleaseBot bumps semver, composes CHANGELOG, creates tag.
* `/sync-docs` → DocsBot syncs CLI help and architecture diagrams.
* `/ux-review` → UI/UX Coach posts a punch list for mobile & accessibility.
* `/gateway-check` → Gateway SRE verifies `/health`, WS, CORS, and `.env`.

> Commands require write/admin permissions. Agents fail gracefully if permissions/secrets are missing.

---

## 3) Branching & Merge Policy

* **Default branch:** `main`
* **Working branches:** `feature/<topic>`, `fix/<topic>`, `codex/<topic>`, `hotfix/<topic>`
* **Release branches:** `release/vX.Y`
* **Backports:** `maintenance/vX.(Y-1)`

**Merge gates:**

1. CI green (build, tests, lint, format, security scan)
2. ≥ 1 human reviewer approval
3. If `infra`/`deploy` changed → InfraBot “preflight ok”
4. If `security` label → SecurityBot “ok” or DRI waiver

**Hotfix:** allowed with `p0` + `hotfix/*`, requires InfraBot auto-deploy + rollback ready.

---

## 4) CI/CD Hand-off (what each agent checks)

* **Codex**

  * `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test -q`
  * UI: `npm ci && npm run build` (no *critical/high* audit blockers)
  * Provides patch if failing.

* **TestSprite**

  * Adds tests; targets ≥ **80%** coverage for touched crates/dirs.

* **SecurityBot**

  * `cargo deny`, dependency audit; NPM `npm audit` (no **critical**)
  * Updates `SECURITY.md` if needed.

* **InfraBot**

  * Lints workflow YAML; strips non-ASCII whitespace.
  * Verifies ports not in use; `.env` keys present.
  * Deploys with health checks & idempotent restarts.

* **ReleaseBot**

  * Enforces Conventional Commits → semantic version bump.
  * Generates release notes + SBOM (CycloneDX) if configured.

---

## 5) Environments & Secrets

* **Dev:** single host; permissive CORS; can use defaults.
* **Staging:** mirrors prod topology; smoke tests must pass.
* **Prod:** locked versions; only ReleaseBot/InfraBot deploy via approved workflows.

**Secrets policy**

* No secrets in code or PR bodies.
* Required CI secrets (examples):
  `DEPLOY_HOST`, `DEPLOY_PORT`, `DEPLOY_USER`, `DEPLOY_SSH_KEY`, `DEPLOY_FINGERPRINT`
  Public build-time env (`NEXT_PUBLIC_*`) still come from **repo variables** or **server .env**, never hardcoded.

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

> Any change to `deploy/**` or workflows pings **InfraBot** and requires preflight.

---

## 7) **Server Health & Diagnostics** ✅

This section defines **manual commands** and the **`/gateway-check`** automation. Use it to verify the UI/gateway stack quickly after deploys or incidents.

### 7.1 Minimal `.env` (server)

```
NEXT_PUBLIC_ENABLE_FULL_UI=1
NEXT_PUBLIC_GATEWAY_URL=https://ui.ippan.org/api
NEXT_PUBLIC_API_BASE_URL=https://ui.ippan.org/api
NEXT_PUBLIC_WS_URL=wss://ui.ippan.org/ws
GATEWAY_ALLOWED_ORIGINS=https://ui.ippan.org
```

> The deploy workflow upserts these keys automatically if you run with defaults.

### 7.2 Local service checks (on the host via SSH)

```bash
# Where the stack lives (or use $DEPLOY_APP_DIR)
APP_DIR="$HOME/apps/ippan-ui"
cd "$APP_DIR" || exit 1

# 1) Compose state
docker compose ps
docker compose logs --tail=100 --timestamps

# 2) .env sanity
grep -E 'NEXT_PUBLIC_(ENABLE_FULL_UI|GATEWAY_URL|API_BASE_URL|WS_URL)|GATEWAY_ALLOWED_ORIGINS' .env || true

# 3) Ports (adjust if you mapped differently)
# Gateway (HTTP) default 8081, UI (HTTP) default 3001
ss -ltnp | grep -E ':8081|:3001' || true
lsof -iTCP:8081 -sTCP:LISTEN || true
lsof -iTCP:3001 -sTCP:LISTEN || true

# 4) Local health endpoints
curl -sS -m 5 -i "http://127.0.0.1:8081/" || true
curl -sS -m 5     "http://127.0.0.1:8081/health" || true
curl -sS -m 5 -i "http://127.0.0.1:3001/" || true

# 5) Reverse proxy (if Nginx)
sudo nginx -t || true
sudo systemctl status nginx --no-pager || true

# 6) System sanity
df -h
free -m
uptime
```

### 7.3 Public checks (from anywhere)

```bash
# UI front door
curl -sS -I "https://ui.ippan.org/"

# Gateway/API front door
curl -sS -I "https://ui.ippan.org/api/"
curl -sS    "https://ui.ippan.org/api/health" || true

# TLS certificate expiry
echo | openssl s_client -servername ui.ippan.org -connect ui.ippan.org:443 2>/dev/null \
  | openssl x509 -noout -dates
```

### 7.4 WebSocket checks

> Best with `websocat` or `wscat`. If not available, you can at least verify the **HTTP 101** handshake with `curl`.

**Handshake (expect `101 Switching Protocols`):**

```bash
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Host: ui.ippan.org" \
  -H "Origin: https://ui.ippan.org" \
  -H "Sec-WebSocket-Key: SGVsbG8sIHdvcmxkIQ==" \
  -H "Sec-WebSocket-Version: 13" \
  "https://ui.ippan.org/ws"
```

**End-to-end (if `websocat` installed):**

```bash
# Should connect and allow sending/receiving small ping messages
websocat -t wss://ui.ippan.org/ws
```

### 7.5 Fast restart & recovery

```bash
# Restart everything idempotently
docker compose up -d --force-recreate

# If a single service is the culprit (replace SERVICE with actual name)
docker compose restart SERVICE

# If ports are stuck by zombie processes (e.g., 8081)
sudo lsof -ti:8081 | xargs --no-run-if-empty sudo kill -9
docker compose up -d --force-recreate
```

### 7.6 What `/gateway-check` automation does

When you comment **`/gateway-check`** on a PR/Issue:

1. **Loads context**: server host/port/user/key from CI secrets.
2. **SSH to host** and:

   * Prints key envs from `.env`.
   * Hits **local** gateway endpoints:

     * `http://127.0.0.1:${GATEWAY_HOST_PORT}/` (expect 200)
     * `http://127.0.0.1:${GATEWAY_HOST_PORT}/health` (expect JSON/OK)
   * Optionally checks UI local port (e.g., `127.0.0.1:3001`).
3. **Public checks**:

   * `https://ui.ippan.org/` returns 200/304.
   * `https://ui.ippan.org/api/` returns 200/3xx (not 4xx/5xx).
   * (If enabled) attempts WS handshake to `wss://ui.ippan.org/ws` and expects **101**.
4. **Outputs** a short report in the PR comment with pass/fail and next steps.

**Pass criteria** (all true):

* `.env` contains the four `NEXT_PUBLIC_*` keys and `GATEWAY_ALLOWED_ORIGINS`.
* Local `GET /health` returns success (HTTP 200, body contains “ok” or status).
* Public `HEAD /` and `HEAD /api/` return HTTP 2xx/3xx.
* (If WS required) WS handshake returns **101**.

**Fail criteria** (any true):

* Missing env keys, or local `health` 5xx/timeout.
* Public `/` or `/api/` returns 4xx/5xx.
* WS handshake not 101 (CORS/Origin or proxy misconfig likely).

### 7.7 Typical failure → fix map

| Symptom                    | Likely Cause                                                            | Quick Fix                                                           |                                  |
| -------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------- | -------------------------------- |
| **UI shows short menu**    | Missing `NEXT_PUBLIC_ENABLE_FULL_UI=1` or bad `NEXT_PUBLIC_*` endpoints | Update `.env` then `docker compose up -d --force-recreate`          |                                  |
| **API 502/504**            | Nginx upstream mismatch, gateway not listening                          | Check compose ports; `ss -ltnp` for 8081; restart gateway           |                                  |
| **CORS errors in browser** | `GATEWAY_ALLOWED_ORIGINS` not set to `https://ui.ippan.org`             | Update `.env`, restart gateway                                      |                                  |
| **Port already allocated** | Stale process on 8081/3001                                              | `sudo lsof -ti:PORT                                                 | xargs sudo kill -9` then restart |
| **WS fails (no 101)**      | Proxy missing `Upgrade`/`Connection` headers                            | Fix Nginx/Envoy config to forward WS upgrade; verify with handshake |                                  |
| **TLS expired**            | Certbot/renewal failed                                                  | Renew certs; re-load Nginx; re-run public checks                    |                                  |

---

## 8) Playbooks

### 8.1 Merge Conflict (HTML/lockfiles)

1. Comment `/codex plan` → Codex proposes resolution.
2. Codex rebases `codex/<topic>` onto `main`, resolves, force-pushes.
3. TestSprite re-runs tests; DocsBot syncs READMEs if templates changed.

### 8.2 “Short menu” in Unified UI

1. `/gateway-check` → SRE posts `.env` keys detected.
2. If missing, InfraBot updates server `.env` with:

   ```
   NEXT_PUBLIC_ENABLE_FULL_UI=1
   NEXT_PUBLIC_GATEWAY_URL=https://ui.ippan.org/api
   NEXT_PUBLIC_API_BASE_URL=https://ui.ippan.org/api
   NEXT_PUBLIC_WS_URL=wss://ui.ippan.org/ws
   ```
3. `/restart ui` then verify `/api` and `/ws` in browser network tab.

### 8.3 Port conflict on deploy

1. InfraBot frees the port or remaps.
2. If still failing, marks PR `blocked` with PID + suggested compose patch.

### 8.4 YAML invalid in workflows

1. InfraBot lints YAML; replaces non-breaking spaces; re-indents.
2. Posts fixed diff; blocks merge until fixed.

### 8.5 Hotfix & rollback

1. Label `p0` and comment `/deploy prod`.
2. InfraBot snapshots previous images and `.env`.
3. If post-deploy health fails, auto-rollback and attach logs.

---

## 9) PR Ready Checklist (agents enforce)

* [ ] Conventional Commit(s) in PR title/squash
* [ ] Code formatted & lint-clean (`cargo fmt`, `clippy`, ESLint if UI)
* [ ] Tests added/updated; coverage ≥ 80% on touched code
* [ ] Security scan shows no **critical** issues (Rust/NPM)
* [ ] Docs updated (`README`, `docs/prd/*`, ADR if architectural)
* [ ] If deploy/infra touched → InfraBot preflight ✅
* [ ] No secrets in diffs, logs, or comments

---

## 10) Documentation Conventions

* **PRDs:** `docs/prd/<slug>.md` → problem, scope, non-goals, acceptance criteria, telemetry.
* **ADRs:** `docs/adr/NNNN-<title>.md` → decision, context, alternatives.
* **Diagrams:** `docs/diagrams/` (source + exported PNG/SVG).
* **CHANGELOG:** generated by ReleaseBot; manual edits discouraged.

---

## 11) Guardrails & Escalation

* Agents never merge failing CI or bypass reviews.
* If agents disagree (e.g., SecurityBot vs Codex), the **DRI** decides and records the rationale in the PR.
* **Escalation path:** DRI → repo maintainer(s) → org admin.

---

**Maintainers:** Update this file whenever labels, commands, or CI gates change so people and agents stay in sync.
