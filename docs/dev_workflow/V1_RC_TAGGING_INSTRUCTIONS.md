# IPPAN v1.0.0-rc1 Tagging Instructions
**Creating the Release Candidate Tag**

**Date:** 2025-11-24  
**Target Commit:** `36eb7c3335180e71069f2bb24cf54cbba1136f31`

---

## Pre-Tag Checklist

Before creating the tag, verify:

- [ ] All phases 1-10 are complete (see `CHECKLIST_AUDIT_MAIN.md`)
- [ ] All tests pass:
  ```bash
  cargo test --workspace
  cargo test -p ippan-consensus-dlc long_run_emission_and_fairness_invariants -- --nocapture
  cargo test -p ippan-consensus-dlc long_run_fairness_roles_remain_balanced -- --nocapture
  ```
- [ ] No uncommitted changes:
  ```bash
  git status
  # Should show: "nothing to commit, working tree clean"
  ```
- [ ] Audit package document updated with final commit hash
- [ ] All documentation cross-linked and up-to-date

---

## Tagging Commands

### Step 1: Ensure You're on Master

```bash
git checkout master
git pull origin master
```

**Current HEAD:** `36eb7c3335180e71069f2bb24cf54cbba1136f31`

### Step 2: Create Annotated Tag

```bash
git tag -a v1.0.0-rc1 36eb7c3335180e71069f2bb24cf54cbba1136f31 -m "$(cat <<'EOF'
IPPAN v1.0.0-rc1: Release Candidate for External Audit

This release candidate includes:
- Deterministic fork-choice and DAG-Fair emission
- Comprehensive test coverage (~80-90% for critical crates)
- Long-run DLC simulations (240-512 rounds)
- AI determinism harness with 50 golden test vectors
- RPC/P2P hardening (rate limiting, size caps) default-on
- Prometheus + Grafana observability
- Multi-node partition test infrastructure
- Complete developer & architecture documentation

Audit-ready deliverables:
- TEST_COVERAGE_REPORT_2025_11_24.md
- ACT_DLC_SIMULATION_REPORT_2025_11_24.md
- AI_DETERMINISM_X86_REPORT_2025_11_24.md
- AI_DETERMINISM_REPRO_REPORT_2025_11_24.md
- AUDIT_PACKAGE_V1_RC1_2025_11_24.md

See AUDIT_PACKAGE_V1_RC1_2025_11_24.md for complete audit scope.

Commit: 36eb7c3335180e71069f2bb24cf54cbba1136f31
Date: 2025-11-24
Branch: master
EOF
)"
```

### Step 3: Verify Tag

```bash
git show v1.0.0-rc1
```

**Expected Output:**
- Tag message with full description
- Commit hash: `36eb7c3335180e71069f2bb24cf54cbba1136f31`
- Commit message and diff

### Step 4: Push Tag to Remote

```bash
git push origin v1.0.0-rc1
```

---

## Creating GitHub Release

### Step 1: Navigate to Releases

1. Go to https://github.com/dmrl789/IPPAN/releases
2. Click "Draft a new release"

### Step 2: Fill Release Form

**Tag:** `v1.0.0-rc1`

**Release Title:** `IPPAN v1.0.0-rc1 - Release Candidate for External Audit`

**Description:**

```markdown
# IPPAN v1.0.0-rc1: Release Candidate

## 🎯 Purpose

This release candidate is prepared for **external security and cryptography audit**. All core consensus, emission, AI fairness, and network components are feature-complete and audit-ready.

## ✨ Highlights

- ✅ **Deterministic fork-choice** with DAG-Fair emission
- ✅ **Comprehensive test coverage** (~80-90% for critical crates)
- ✅ **Long-run DLC simulations** (240-512 rounds with adversarial scenarios)
- ✅ **AI determinism harness** (50 golden test vectors, no floats)
- ✅ **RPC/P2P hardening** (rate limiting, size caps) enabled by default
- ✅ **Prometheus + Grafana** observability
- ✅ **Multi-node partition tests** with docker-compose
- ✅ **Complete documentation** (dev guide, architecture, operator docs)

## 📦 Audit Deliverables

| Document | Description |
|----------|-------------|
| `AUDIT_PACKAGE_V1_RC1_2025_11_24.md` | Master audit package with scope & checklist |
| `TEST_COVERAGE_REPORT_2025_11_24.md` | Test coverage for critical crates |
| `ACT_DLC_SIMULATION_REPORT_2025_11_24.md` | Long-run simulation results |
| `AI_DETERMINISM_X86_REPORT_2025_11_24.md` | AI determinism validation (x86_64) |
| `AI_DETERMINISM_REPRO_REPORT_2025_11_24.md` | Cross-architecture repro guide |

## 🔒 Security

- **Slashing:** 50% for double-signing, 10% for invalid blocks
- **Shadow verifiers:** Detect primary misbehavior
- **Rate limiting:** Per-IP caps on RPC requests
- **Message size limits:** Drop oversized P2P messages
- **Threat model:** See `SECURITY_THREAT_MODEL.md`

## 🚀 Quick Start

### Run a Node

```bash
git clone https://github.com/dmrl789/IPPAN.git
cd IPPAN
git checkout v1.0.0-rc1
cargo build --release
./target/release/ippan-node --config config/local-node.toml
```

### Run Tests

```bash
cargo test --workspace
cargo test -p ippan-consensus-dlc long_run_emission_and_fairness_invariants -- --nocapture
```

### Run Determinism Harness

```bash
cargo run --bin determinism_harness -- --format json
```

## 📚 Documentation

- **Developer Guide:** `docs/dev_guide.md`
- **Architecture Overview:** `docs/architecture_overview.md`
- **Operator Monitoring:** `docs/operators/monitoring.md`
- **Feature Checklist:** `CHECKLIST_AUDIT_MAIN.md`

## 🔍 For Auditors

1. Clone repository: `git clone https://github.com/dmrl789/IPPAN.git && cd IPPAN`
2. Checkout tag: `git checkout v1.0.0-rc1`
3. Read audit package: `AUDIT_PACKAGE_V1_RC1_2025_11_24.md`
4. Run test suite: `cargo test --workspace`
5. Review critical crates:
   - `crates/consensus/`
   - `crates/consensus_dlc/`
   - `crates/ai_core/`
   - `crates/time/`
   - `crates/storage/`

## ⚠️ Known Limitations

- RPC tests require OpenSSL headers (environment-specific)
- Long-run chaos tests are manual (not automated in CI)
- Cross-architecture determinism validated, not CI-gated

## 📅 Roadmap

1. **Current:** External audit (this RC)
2. **Next:** v1.0.0-rc2 (if critical fixes needed)
3. **Target:** v1.0.0 mainnet launch

## 📬 Contact

- **Issues:** https://github.com/dmrl789/IPPAN/issues
- **Discussions:** https://github.com/dmrl789/IPPAN/discussions
- **Security:** See `SECURITY.md` for responsible disclosure

---

**Commit:** 36eb7c3335180e71069f2bb24cf54cbba1136f31  
**Date:** 2025-11-24  
**Status:** Audit-Ready
```

### Step 3: Attach Artifacts (Optional)

If you want to attach pre-built binaries or documentation archives:

```bash
# Build release binary
cargo build --release

# Create documentation archive
tar -czf ippan-v1-rc1-docs.tar.gz \
  AUDIT_PACKAGE_V1_RC1_2025_11_24.md \
  TEST_COVERAGE_REPORT_2025_11_24.md \
  ACT_DLC_SIMULATION_REPORT_2025_11_24.md \
  AI_DETERMINISM_X86_REPORT_2025_11_24.md \
  AI_DETERMINISM_REPRO_REPORT_2025_11_24.md \
  docs/ \
  grafana_dashboards/

# Attach via GitHub UI:
# - target/release/ippan-node (optional)
# - ippan-v1-rc1-docs.tar.gz
```

### Step 4: Publish Release

1. Select "Set as a pre-release" (this is an RC, not a stable release)
2. Click "Publish release"

---

## Post-Tag Tasks

### Update Documentation

Update `AUDIT_READY.md` with the final tag:

```markdown
- **Release Candidate:** v1.0.0-rc1
- **Target Commit:** 36eb7c3335180e71069f2bb24cf54cbba1136f31
- **Tagged:** 2025-11-24
- **GitHub Release:** https://github.com/dmrl789/IPPAN/releases/tag/v1.0.0-rc1
```

### Notify Auditors

Send email/message to auditors with:
- GitHub release link
- Audit package document link
- Point of contact for questions
- Expected timeline for audit completion

---

## Verification

### Verify Tag Locally

```bash
git tag -v v1.0.0-rc1  # If signed with GPG
git show v1.0.0-rc1
```

### Verify Tag on GitHub

```bash
git ls-remote --tags origin | grep v1.0.0-rc1
```

**Expected:** `refs/tags/v1.0.0-rc1` pointing to `36eb7c3`

### Clone and Build from Tag

```bash
git clone https://github.com/dmrl789/IPPAN.git ippan-rc1-test
cd ippan-rc1-test
git checkout v1.0.0-rc1
cargo build --release
cargo test --workspace
```

**Expected:** All builds and tests pass

---

## Rollback (If Needed)

If you need to delete the tag:

```bash
# Delete local tag
git tag -d v1.0.0-rc1

# Delete remote tag
git push origin :refs/tags/v1.0.0-rc1

# Delete GitHub release
# (Manual: Navigate to releases and delete)
```

**Note:** Only do this if absolutely necessary before auditors start work.

---

## Summary

✅ **Tag Created:** `v1.0.0-rc1`  
✅ **Commit:** `36eb7c3335180e71069f2bb24cf54cbba1136f31`  
✅ **GitHub Release:** Published as pre-release  
✅ **Audit Package:** Referenced in release notes  
✅ **Documentation:** All deliverables linked  

**Status:** Ready for external audit

---

**Prepared:** 2025-11-24  
**Maintainers:** Ugo Giuliani, Desirée Verga, Kambei Sapote
