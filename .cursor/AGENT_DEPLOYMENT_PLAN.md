# 🧩 IPPAN Agent Deployment Plan
**Version:** 2025-11-08  
**Purpose:** Structured recovery and revision plan for the IPPAN monorepo after widespread CI failures.  
**Companion Document:** `.cursor/AGENT_CHARTER.md`

---

## 🚀 Objective

This plan defines how to systematically assign Cursor/Codex agents to each crate, repair all compilation and CI issues, and safely merge into `main` **without overlapping edits**.

All agents must obey the Charter rules.

---

## 🧭 1. Staging Branch Setup

Before starting any agent:

```bash
git checkout main
git pull
git checkout -b fix/stabilize-2025-11-08
```

This isolates all fixes from production (`main`).

---

## 🧠 2. Agent Assignment Matrix

| Order | Subsystem / Path          | Agent Name             | Branch                 | Main Tasks                                                          |
| ----- | ------------------------- | ---------------------- | ---------------------- | ------------------------------------------------------------------- |
| 1️⃣   | `/crates/ai_core`         | `ai-core-determinism`  | `cursor/fix-ai-core`   | Fix deterministic math, remove float ops, pass AI determinism tests |
| 2️⃣   | `/crates/consensus`       | `consensus-validation` | `cursor/fix-consensus` | Fix DLC rounds, shadow verifier logic, validator selection          |
| 3️⃣   | `/crates/storage`         | `storage-db`           | `cursor/fix-storage`   | Repair Sled operations, serialization errors, database paths        |
| 4️⃣   | `/crates/crypto`          | `crypto-validation`    | `cursor/fix-crypto`    | Fix Ed25519 keypair conversions, signing, and address encoders      |
| 5️⃣   | `/crates/p2p`             | `p2p-network`          | `cursor/fix-p2p`       | Fix libp2p imports, feature flags, and NAT hole-punching logic      |
| 6️⃣   | `/crates/mempool`         | `mempool-integration`  | `cursor/fix-mempool`   | Fix transaction validation and relay consistency                    |
| 7️⃣   | `/crates/ippan_economics` | `economics-engine`     | `cursor/fix-economics` | Fix emission, rewards distribution, treasury balance tests          |
| 8️⃣   | `/apps/gateway`           | `gateway-api`          | `cursor/fix-gateway`   | Fix Warp routes, API responses, Dockerfile build                    |
| 9️⃣   | `/apps/ui`                | `ui-checks`            | `cursor/fix-ui`        | Fix Next.js build, static export, environment variables             |
| 🔟    | `.github/workflows`       | `infra-ci`             | `cursor/fix-ci`        | Repair CI/CD jobs, dependency scans, Docker build matrix            |

---

## ⚙️ 3. Workflow for Each Agent

For each crate:

1. **Create branch**

   ```bash
   git checkout fix/stabilize-2025-11-08
   git checkout -b cursor/fix-<crate>
   ```

2. **Create agent in Cursor**
   Name: same as `Agent Name` in the table above.

3. **Paste this message to start:**

   ```
   You are <agent-name>.
   Follow the Charter in `.cursor/AGENT_CHARTER.md`.
   Your scope is <path>.
   Task: make this crate compile, test, and integrate cleanly with the workspace.
   Do not touch other crates or shared files.
   ```

4. **Let agent work, then validate locally:**

   ```bash
   cargo check -p ippan_<crate> --all-features
   cargo test -p ippan_<crate> --all-features
   ```

5. **Commit and push:**

   ```bash
   git add .
   git commit -m "[<agent-name>] Stabilize crate and pass tests"
   git push origin cursor/fix-<crate>
   ```

6. **Create PR** → base: `fix/stabilize-2025-11-08`, head: `cursor/fix-<crate>`

---

## 🧪 4. After All Crates Are Fixed

Validate entire workspace:

```bash
git checkout fix/stabilize-2025-11-08
cargo clean
cargo check --workspace --all-features
cargo clippy --workspace -- -D warnings
cargo test --workspace --all-features
```

If all green ✅ → merge into main.

---

## 🧰 5. CI/CD Recovery (Infra Agent)

Once code compiles:

1. Create agent `infra-ci`
2. Scope: `.github/workflows`
3. Task:

   ```
   You are `infra-ci`.
   Follow the Charter in `.cursor/AGENT_CHARTER.md`.
   Task: repair all CI/CD failures (Docker builds, determinism tests, DLC benchmarks).
   Ensure jobs use correct crate names, paths, and branch filters.
   ```
4. Validate with:

   ```bash
   gh workflow run CI.yml
   ```

---

## 📜 6. Merge Strategy

1. Merge each agent PR sequentially into `fix/stabilize-2025-11-08`.
2. When all pass:

   ```bash
   git checkout main
   git pull
   git merge fix/stabilize-2025-11-08
   git push origin main
   ```
3. Close temporary branches.

---

## ✅ 7. Final Checklist

* [ ] `.cursor/AGENT_CHARTER.md` committed
* [ ] `fix/stabilize-2025-11-08` branch created
* [ ] One agent per crate
* [ ] All crates compile and test individually
* [ ] Workspace builds cleanly
* [ ] CI/CD workflows fixed
* [ ] Merge to `main` complete

---

## 🧱 8. Notes

* Agents must **not** modify dependencies or workflow YAMLs unless assigned.
* Keep commits atomic and descriptive.
* Determinism and reproducibility are top priority — floating-point, randomness, and concurrency must be deterministic.
* Only merge once all determinism and DLC tests pass on both `x86_64` and `aarch64`.

---

### End of Plan

