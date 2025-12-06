# IPPAN Readiness Status (Audit-Ready)

## Current Status

✅ **Readiness: 100%** (as of 2025-12-06)

## Evidence

- **Latest Audit Pack evidence:** [docs/audit/LAST_AUDIT_PACK_RUN.md](audit/LAST_AUDIT_PACK_RUN.md)
- **Readiness plan:** [docs/READINESS_100_PLAN.md](READINESS_100_PLAN.md)
- **Docs index:** [docs/INDEX.md](INDEX.md)

## Verification Links (GitHub Actions)

- **Build & Test (Rust):** [View workflow runs](https://github.com/dmrl789/IPPAN/actions/workflows/ci.yml?query=branch%3Amaster)
- **AI Determinism & DLC Consensus:** [View workflow runs](https://github.com/dmrl789/IPPAN/actions/workflows/ai-determinism.yml?query=branch%3Amaster)
- **Audit Pack — P0 Gates + SBOM:** [View workflow runs](https://github.com/dmrl789/IPPAN/actions/workflows/audit-pack.yml)
- **Soak — DLC Long-Run Determinism:** [View workflow runs](https://github.com/dmrl789/IPPAN/actions/workflows/soak-dlc-longrun.yml)
- **Fuzz Smoke Tests:** [View workflow runs](https://github.com/dmrl789/IPPAN/actions/workflows/fuzz-smoke.yml)
- **Fuzz Nightly (Long-Run):** [View workflow runs](https://github.com/dmrl789/IPPAN/actions/workflows/fuzz-nightly.yml)
- **Readiness Pulse (Weekly):** [View workflow runs](https://github.com/dmrl789/IPPAN/actions/workflows/readiness-pulse.yml)

## Readiness Pulse (Weekly)

The **Readiness Pulse** workflow runs weekly (Sundays 04:00 UTC) and can also be triggered manually. It provides a comprehensive health check by running:

1. **Audit Pack gates:**
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `cargo test --workspace --all-targets --all-features`
   - `cargo run -p ippan-ai-core --bin verify_model_hash -- config/dlc.toml`
   - `cargo deny check advisories bans licenses sources`
   - `cargo cyclonedx` (SBOM generation)

2. **Short soak loop:** Runs `fairness_invariants` test repeatedly for a configurable duration (default: 30 minutes)

3. **Short fuzz tests:** Runs each fuzz target for a configurable duration (default: 5 minutes per target)
   - `canonical_hash`
   - `rpc_body_limit`
   - `proof_parsing`

### Artifacts

All artifacts are uploaded as a single bundle: `readiness-pulse-<run_id>` with 21-day retention:
- `gates.log` — Format, clippy, and test output
- `verify_model_hash.log` — Model hash verification
- `sbom.json` — Software Bill of Materials
- `cargo-deny.txt` — Dependency audit results
- `soak.log` — Soak test iterations
- `fuzz/fuzz.log` — Fuzz test output
- `fuzz/artifacts/` — Any crash artifacts (if found)

### Running Manually

1. Go to **GitHub Actions → Readiness Pulse (Weekly)**
2. Click **Run workflow**
3. Optionally adjust:
   - `minutes_soak` (default: 30)
   - `minutes_fuzz_each` (default: 5)
4. Select branch (usually `master`) and click **Run workflow**

**Note:** Fuzz tool installation failures are non-blocking. The workflow will log "SKIPPED" and continue. Only core gates (fmt/clippy/test/verify_model_hash/cargo-deny/sbom) must pass.

## How to Verify in 3 Clicks

1. **Run Audit Pack manually:** GitHub Actions → Audit Pack — P0 Gates + SBOM → Run workflow on `master`
2. **Confirm run is green:** Open the completed run and verify all jobs passed
3. **Confirm artifacts:** Download and verify `sbom.json`, `cargo-deny.txt`, `gates.log`, and `verify_model_hash.log` are present

## What "100%" Means Here

- ✅ Deterministic AI model is hash-locked in `config/dlc.toml`
- ✅ CI is green on `master` branch
- ✅ Soak + fuzz workflows are in place and producing artifacts
- ✅ Audit Pack produces SBOM + license/advisory reports and gate logs
- ✅ All P0 gates (fmt, clippy, tests, model hash verification) passing
- ✅ No f32/f64 in runtime code (enforced in CI)
- ✅ Cross-platform determinism verified (x86_64 ↔ aarch64)

---

*This dashboard provides a single source of truth for IPPAN readiness status and verification procedures.*

