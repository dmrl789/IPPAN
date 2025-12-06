# IPPAN Readiness 100% Plan

**Status:** Release Candidate (v0.9.0-rc1) → Production Ready (100%)

**Last Updated:** 2025-01-XX

---

## A) Ready Today (Already Complete)

✅ **D-GBDT Model Trained & Promoted**
- Model trained, promoted, and `expected_hash` set in `config/dlc.toml`
- Model hash verification binary (`verify_model_hash`) operational
- Model path: `crates/ai_registry/models/ippan_d_gbdt_offline_20251204T141734Z.json`
- Expected hash: `27022221142b96080e4c1c037ed879816c5ffee1c88aa756dbf34098d68f2dc1`

✅ **CI Disk-Space Fix Merged**
- Master branch CI green
- Disk-space cleanup action integrated in workflows
- No CI timeouts or resource exhaustion

✅ **Core Gates Present**
- `cargo fmt` - code formatting enforcement
- `cargo clippy` - linting and best practices
- `cargo test` - unit and integration tests
- Model hash verification in CI (`test-model-hash` job)
- AI determinism tests (cross-platform)
- No floating-point operations (f32/f64) in runtime code

---

## B) Remaining to Reach 100% (P0/P1)

### P0 (Release Blockers)

#### 1. Long-Run Determinism / DLC Stability Soak Tests
**Status:** Missing  
**Priority:** P0 (Release Blocker)

**Requirement:**
- Extended determinism tests running for hours (not minutes)
- DLC consensus stability validation over extended periods
- Scheduled or manual trigger capability
- Results stored as artifacts for analysis

**Implementation:**
- New workflow file or extended job in existing workflow
- Use `workflow_dispatch` + `schedule` (weekly)
- Separate job that runs extended tests without timeout
- Store determinism logs, DLC state snapshots, and metrics

**Acceptance Criteria:**
- [ ] Workflow can run 4+ hour soak tests
- [ ] No determinism drift detected over extended runs
- [ ] DLC consensus remains stable (no forks, no validator selection anomalies)
- [ ] Results archived and accessible

---

#### 2. Fuzz / Property Tests for Canonical Hashing + Proof Bundles + Critical Parsers
**Status:** Missing  
**Priority:** P0 (Release Blocker)

**Requirement:**
- Fuzz targets for canonical hashing (model hash, block hash, transaction hash)
- Property tests for proof bundle serialization/deserialization
- Fuzz tests for critical parsers (RPC, consensus messages, DLC config)

**Target Crates:**
- `crates/rpc` - RPC message parsing
- `crates/ai_core` - canonical hashing, model parsing
- `crates/consensus_dlc` - proof bundles, DLC messages
- `crates/consensus` - block/transaction parsing

**Implementation:**
- Add `cargo-fuzz` targets or `proptest` property tests
- CI job for short fuzz smoke tests (PR gate)
- Nightly long fuzz runs (hours)
- Coverage reporting for fuzz targets

**Acceptance Criteria:**
- [ ] Fuzz targets for canonical hashing (model, block, tx)
- [ ] Property tests for proof bundle round-trip
- [ ] Fuzz tests for RPC and consensus parsers
- [ ] CI runs short fuzz on PRs (< 5 min)
- [ ] Nightly long fuzz runs (2+ hours) with crash reports

---

#### 3. Release Pipeline Hardening
**Status:** Partial  
**Priority:** P0 (Release Blocker)

**Requirement:**
- Reproducible builds (deterministic compilation)
- SBOM (Software Bill of Materials) generation
- Signed artifacts (binaries, releases)
- Versioning policy enforcement

**Implementation:**
- Build reproducibility checks (compare builds across runners)
- SBOM generation via `cargo-cyclonedx` or similar
- GPG signing for release artifacts
- Semantic versioning enforcement in CI

**Acceptance Criteria:**
- [ ] Reproducible builds verified (same hash across builds)
- [ ] SBOM generated for each release
- [ ] Release artifacts GPG signed
- [ ] Version tags follow semver policy

---

#### 4. Documentation Consolidation
**Status:** Needs Audit  
**Priority:** P0 (Release Blocker)

**Requirement:**
- Remove/merge outdated documentation
- Single entry point (`docs/INDEX.md`)
- Clear "what to run" section for auditors
- Remove duplicate/overlapping docs

**Implementation:**
- Audit `/docs` directory
- Identify obsolete files (archive or remove)
- Merge overlapping documentation
- Create `docs/INDEX.md` as single source of truth
- Update `README.md` to point to index

**Acceptance Criteria:**
- [ ] `docs/INDEX.md` exists and is comprehensive
- [ ] No duplicate/conflicting documentation
- [ ] Obsolete docs archived or removed
- [ ] README points to `docs/INDEX.md`
- [ ] Clear "Auditor Quick Start" section

---

### P1 (Post-RC)

#### 1. Performance Baselines + Regression Thresholds
**Status:** Missing  
**Priority:** P1 (Post-RC)

**Requirement:**
- TPS (transactions per second) baseline measurement
- Latency percentiles (p50, p95, p99)
- Regression thresholds in CI
- Performance dashboard/tracking

**Acceptance Criteria:**
- [ ] TPS baseline established
- [ ] Latency metrics collected
- [ ] CI fails if performance degrades > 10%
- [ ] Performance dashboard available

---

#### 2. Chaos Testing / Fault Injection
**Status:** Missing  
**Priority:** P1 (Post-RC)

**Requirement:**
- Network partition simulation
- Byzantine message injection
- Node failure scenarios
- Recovery validation

**Acceptance Criteria:**
- [ ] Chaos tests for network partitions
- [ ] Byzantine validator behavior tests
- [ ] Node failure recovery tests
- [ ] Results documented

---

#### 3. Security Audit Prep Pack
**Status:** Partial  
**Priority:** P1 (Post-RC)

**Requirement:**
- Threat model document
- Security audit scope definition
- Dependency policy (allowed/blocked crates)
- Security checklists

**Acceptance Criteria:**
- [ ] Threat model document complete
- [ ] Audit scope clearly defined
- [ ] Dependency policy enforced in CI
- [ ] Security checklist for releases

---

## C) Exact Commands Auditors Can Run

### Local Verification (Windows)

```powershell
# 1. Format check
cargo fmt --all -- --check

# 2. Linting
cargo clippy --all-targets --all-features -- -D warnings

# 3. Run all tests
cargo test --workspace --all-targets

# 4. Verify model hash
cargo run -p ippan-ai-core --bin verify_model_hash -- config/dlc.toml

# 5. Build release binaries
cargo build --workspace --release
```

### CI Links

- **Main CI:** `.github/workflows/ci.yml` - Format, lint, build, test
- **AI Determinism:** `.github/workflows/ai-determinism.yml` - AI determinism, DLC consensus
- **Nightly Validation:** `.github/workflows/nightly-validation.yml` - Extended validation
- **Security:** `.github/workflows/codeql.yml` - CodeQL security analysis

### Reproducing Locally

**Windows (PowerShell):**
- Install Rust via `rustup` or `winget install Rustlang.Rustup`
- Run commands above (cargo is available on Windows)
- No WSL required

**Linux/WSL:**
- Same commands as Windows
- May require additional system dependencies (see CI workflows)

---

## D) Definition of Done — Ready = 100%

### Release Readiness Checklist

- [ ] **P0-1:** Long-run determinism soak tests implemented and passing
- [ ] **P0-2:** Fuzz/property tests for canonical hashing + proof bundles + parsers
- [ ] **P0-3:** Release pipeline hardening (reproducible builds, SBOM, signing, versioning)
- [ ] **P0-4:** Documentation consolidated (INDEX.md, obsolete docs removed)
- [ ] **All P0 items:** Verified in CI and documented
- [ ] **Model hash verification:** Passing locally and in CI
- [ ] **CI gates:** All green (fmt, clippy, tests, model hash)
- [ ] **No f32/f64:** Runtime code verified clean
- [ ] **DLC consensus:** All tests passing
- [ ] **Cross-platform determinism:** Verified (x86_64 ↔ aarch64)
- [ ] **Audit Pack workflow run on HEAD passes and artifacts are published (SBOM + cargo-deny + logs).**

### Measurable Pass/Fail Criteria

| Criterion | Pass | Fail |
|-----------|------|------|
| Long-run soak tests | 4+ hours, no drift | Drift detected or timeout |
| Fuzz tests | No crashes in 2+ hour run | Crash found |
| Reproducible builds | Same hash across builds | Hash mismatch |
| Documentation | INDEX.md exists, no duplicates | Missing or conflicting docs |
| Model hash | Matches expected | Mismatch |
| CI gates | All green | Any red |
| f32/f64 check | None in runtime | Found in runtime code |

---

## Next Steps

1. **Create tracking issues** for P0 items (see `docs/issues/`)
2. **Implement P0-1:** Long-run soak test workflow
3. **Implement P0-2:** Fuzz targets and property tests
4. **Implement P0-3:** Release pipeline hardening
5. **Implement P0-4:** Documentation consolidation
6. **Verify locally:** Run all commands in section C
7. **CI validation:** Ensure all gates pass
8. **Mark 100%:** When all P0 items complete

---

**Questions or Updates?**  
Update this document as items are completed. This is the single source of truth for readiness tracking.

