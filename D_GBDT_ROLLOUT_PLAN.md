# D-GBDT Rollout Orchestration Plan

**Branch:** `feat/d-gbdt-rollout`  
**Goal:** Enforce zero floating-point usage in GBDT inference runtime across `ai_core`, `ai_registry`, `consensus_dlc`  
**Strategy:** 7-phase sequential rollout with strict acceptance gates

---

## 🎯 Acceptance Gates (Every Phase)

Before merging each phase PR:

1. **Float Check:** `rg -n "(f32|f64)" crates/ai_core/src crates/ai_registry/src crates/consensus_dlc/src | grep -v "tests/" | grep -v "//.*f32\|f64"` returns EMPTY
2. **Build:** `cargo test --workspace --no-run` succeeds
3. **Tests:** New unit tests + at least 1 cross-arch determinism test
4. **Hashing:** Model JSON canonicalization + BLAKE3 validation (from Phase 3+)
5. **CI:** All workflows pass on phase branch

---

## 📋 Phase Breakdown

### Phase 1: Deterministic Math Foundation
**Branch:** `phase1/deterministic-math` (from `feat/d-gbdt-rollout`)

**Agent 1 Tasks:**
- Audit `crates/ai_core/src/fixed.rs` and `fixed_point.rs` for bit-identical operations
- Remove any remaining f32/f64 from non-test runtime paths
- Implement saturating arithmetic and overflow handling
- Add cross-platform unit tests (x86_64, aarch64, wasm32)

**Deliverables:**
- [ ] Hardened fixed-point math module
- [ ] 100% deterministic arithmetic operations
- [ ] Test suite validating bit-identical outputs

**PR Target:** `feat/d-gbdt-rollout`

---

### Phase 2: Inference Engine Rewrite
**Branch:** `phase2/inference-engine` (from `feat/d-gbdt-rollout`)

**Agent 2 Tasks:**
- Refactor `crates/ai_core/src/deterministic_gbdt.rs` to use only fixed-point math
- Replace all float operations in tree traversal, split evaluation, and leaf scoring
- Ensure prediction pipeline is fully deterministic
- Create test vectors with known inputs/outputs for validation

**Deliverables:**
- [ ] Zero-float inference engine
- [ ] Test vectors covering edge cases (underflow, overflow, negative values)
- [ ] Benchmark showing <5% performance regression vs floating-point

**PR Target:** `feat/d-gbdt-rollout`

---

### Phase 3: Model Registry Determinism
**Branch:** `phase3/model-registry` (from `feat/d-gbdt-rollout`)

**Agent 3 Tasks:**
- Implement canonical JSON serialization in `crates/ai_registry/src/manifest.rs`
- Fix BLAKE3 hashing to ensure reproducible model IDs across platforms
- Add model versioning with hash-based integrity checks
- Update storage layer to validate determinism on load

**Deliverables:**
- [ ] Canonical model serialization (sorted keys, fixed precision)
- [ ] Reproducible BLAKE3 model hashes
- [ ] Registry tests validating cross-platform model identity

**PR Target:** `feat/d-gbdt-rollout`

---

### Phase 4: Consensus DLC Integration
**Branch:** `phase4/consensus-integration` (from `feat/d-gbdt-rollout`)

**Agent 4 Tasks:**
- Integrate deterministic GBDT inference into `crates/consensus_dlc`
- Ensure all validators compute identical predictions for the same model+input
- Add consensus validation tests with multi-node simulation
- Document consensus failure modes and recovery

**Deliverables:**
- [ ] DLC module using deterministic inference
- [ ] Multi-node determinism tests (3+ validators)
- [ ] Consensus validation passing with 100% agreement

**PR Target:** `feat/d-gbdt-rollout`

---

### Phase 5: CI Determinism Enforcement
**Branch:** `phase5/ci-determinism` (from `feat/d-gbdt-rollout`)

**Agent 5 Tasks:**
- Update `.github/workflows/ai-determinism.yml` with cross-arch matrix
- Add jobs for x86_64-linux, aarch64-linux, x86_64-macos
- Implement output comparison logic (BLAKE3 hash of predictions)
- Add float usage linting as blocking CI check

**Deliverables:**
- [ ] Matrix CI job testing 3+ architectures
- [ ] Automated float detection preventing merges
- [ ] Determinism validation on every PR

**PR Target:** `feat/d-gbdt-rollout`

---

### Phase 6: Trainer CLI Updates
**Branch:** `phase6/trainer-cli` (from `feat/d-gbdt-rollout`)

**Agent 6 Tasks:**
- Update training CLI to emit models with fixed-point weights
- Add `--deterministic` flag enforcing zero-float constraint
- Implement quantization strategy for converting trained models
- Add validation step ensuring trained models pass determinism checks

**Deliverables:**
- [ ] Trainer producing deterministic models
- [ ] Quantization tooling for legacy model migration
- [ ] Training workflow documentation

**PR Target:** `feat/d-gbdt-rollout`

---

### Phase 7: Documentation & Migration Guide
**Branch:** `phase7/docs` (from `feat/d-gbdt-rollout`)

**Agent 7 Tasks:**
- Document D-GBDT architecture in `docs/ai/deterministic-gbdt.md`
- Create migration guide for existing models and pipelines
- Add API documentation for fixed-point math module
- Update README with determinism guarantees

**Deliverables:**
- [ ] Architecture documentation
- [ ] Migration guide with examples
- [ ] Updated API docs

**PR Target:** `feat/d-gbdt-rollout`

---

## 🔀 Merge Flow

```
main
  └─ feat/d-gbdt-rollout (long-lived feature branch)
       ├─ phase1/deterministic-math → PR #1 → merged
       ├─ phase2/inference-engine    → PR #2 → merged
       ├─ phase3/model-registry      → PR #3 → merged
       ├─ phase4/consensus-integration → PR #4 → merged
       ├─ phase5/ci-determinism      → PR #5 → merged
       ├─ phase6/trainer-cli         → PR #6 → merged
       └─ phase7/docs                → PR #7 → merged
  ← feat/d-gbdt-rollout → main (final PR after all phases)
```

---

## 🚦 Gate Validation Commands

```bash
# Float detection (should return empty)
rg -n "(f32|f64)" crates/ai_core/src crates/ai_registry/src crates/consensus_dlc/src \
  | grep -v "tests/" | grep -v "//.*\(f32\|f64\)"

# Build check
cargo test --workspace --no-run

# Run determinism tests
cargo test --workspace determinism

# Cross-arch CI trigger
gh workflow run ai-determinism.yml --ref feat/d-gbdt-rollout
```

---

## 📊 Success Metrics

- **Zero floats** in runtime inference paths
- **100% determinism** across x86_64, aarch64, wasm32
- **<5% performance regression** vs floating-point baseline
- **All CI green** on every phase PR
- **Model hashes reproducible** across platforms

---

## 🤝 Agent Assignments

| Phase | Agent | Crate(s) | Estimated Effort |
|-------|--------|----------|------------------|
| 1 | Agent-Alpha | ai_core (math) | 2-3 days |
| 2 | Agent-Alpha | ai_core (inference) | 3-4 days |
| 3 | Agent-Theta | ai_registry | 2-3 days |
| 4 | Agent-Alpha | consensus_dlc | 3-4 days |
| 5 | Agent-Sigma | .github/workflows | 1-2 days |
| 6 | Agent-Zeta | ai_core (training) | 2-3 days |
| 7 | DocsAgent | docs/ | 1-2 days |

**Total Estimated Timeline:** 14-21 days (sequential) or 7-10 days (parallel where safe)

---

## 🔒 Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Performance regression | Benchmark every phase; optimize hot paths |
| Model compatibility break | Version models; provide migration tooling |
| Consensus disagreement | Extensive multi-node testing in Phase 4 |
| CI instability | Isolate determinism tests; retry logic |
| Floating-point creep | Automated float detection in CI (Phase 5) |

---

**Created:** 2025-11-12  
**Status:** Phase 0 (Planning) → Phase 1 Ready  
**Next Action:** Assign Agent 1 to Phase 1 branch
