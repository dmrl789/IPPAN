# IPPAN Release Process

**Version:** 1.0  
**Last Updated:** 2025-11-24

---

## Overview

This document describes the end-to-end process for creating IPPAN releases, from code freeze to production deployment.

**Release Types:**
- **Release Candidate (RC):** Pre-release for testing and audits (e.g., `v1.0.0-rc1`)
- **Stable Release:** Production-ready version (e.g., `v1.0.0`)
- **Patch Release:** Bug fixes only (e.g., `v1.0.1`)
- **Minor Release:** New features, backward compatible (e.g., `v1.1.0`)
- **Major Release:** Breaking changes (e.g., `v2.0.0`)

---

## Versioning Scheme

IPPAN follows [Semantic Versioning 2.0.0](https://semver.org/):

```
v<MAJOR>.<MINOR>.<PATCH>[-<PRERELEASE>][+<BUILD>]
```

**Examples:**
- `v1.0.0-rc1` — Release Candidate 1
- `v1.0.0` — Stable mainnet release
- `v1.0.1` — Patch release (bug fixes)
- `v1.1.0` — Minor release (new features)
- `v2.0.0` — Major release (breaking changes)

**Numbering:**
- **MAJOR:** Increment for incompatible protocol changes
- **MINOR:** Increment for backward-compatible features
- **PATCH:** Increment for backward-compatible bug fixes
- **PRERELEASE:** `-rc1`, `-beta1`, `-alpha1`

---

## Release Roles

| Role | Responsibility |
|------|----------------|
| **Release Manager** | Coordinates release process, creates tags |
| **Lead Architect** | Approves feature completeness, signs off on RC |
| **QA Team** | Runs smoke tests, validates test coverage |
| **Security Team** | Reviews security fixes, runs audits |
| **DevOps** | Builds binaries, deploys to testnet/mainnet |
| **Documentation Lead** | Ensures release notes and docs are updated |

---

## Release Phases

### Phase 1: Planning (2-4 weeks before release)

#### 1.1 Feature Freeze

- Create a GitHub milestone for the release (e.g., `v1.1.0`)
- Mark all PRs as "must-have" or "deferred"
- Code freeze: Only critical bug fixes allowed

#### 1.2 Release Checklist

Create a release tracking issue:

```markdown
## Release Checklist: v1.1.0

**Target Date:** 2025-12-15

### Pre-Release
- [ ] All milestone PRs merged
- [ ] CI/CD green on master
- [ ] Test coverage ≥80% for critical crates
- [ ] No open P0/P1 bugs
- [ ] Release notes drafted
- [ ] Changelog updated
- [ ] Docs reviewed and updated

### Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Long-run simulations (512 rounds)
- [ ] Determinism harness validated
- [ ] Manual smoke tests completed

### Artifacts
- [ ] Binaries built for Linux/macOS/Windows
- [ ] Docker images tagged
- [ ] SBOM generated
- [ ] Release notes finalized

### Deployment
- [ ] Testnet rollout successful
- [ ] Mainnet deployment planned
```

---

### Phase 2: Testing & Validation (1-2 weeks)

#### 2.1 Automated Tests

Run full test suite:

```bash
# All unit tests
cargo test --workspace

# Long-run simulations
cargo test -p ippan-consensus-dlc long_run_emission_and_fairness_invariants -- --nocapture --test-threads=1
cargo test -p ippan-consensus-dlc long_run_fairness_roles_remain_balanced -- --nocapture --test-threads=1
cargo test -p ippan-consensus-dlc long_run_dlc_with_churn_splits_slashing_and_drift -- --nocapture --test-threads=1

# Determinism harness
cargo run --bin determinism_harness -- --format json > determinism_$(uname -m).json

# Property tests
cargo test --workspace --test property_*

# Benchmarks (optional)
cargo bench --workspace
```

**Pass Criteria:**
- All tests green
- No new compiler warnings
- No regressions in benchmarks
- Determinism harness matches golden digest

#### 2.2 Manual Smoke Tests

**Testnet Deployment:**
1. Deploy to isolated testnet
2. Run for 24-48 hours
3. Validate:
   - Node stays online
   - Transactions process normally
   - No memory leaks (check with `top`/`htop`)
   - Metrics endpoint responsive

**Payment Flow:**
```bash
# Generate keys
cargo run -p ippan-wallet -- create --output test_wallet.json

# Fund wallet (devnet)
curl -X POST http://localhost:8080/dev/fund \
  -H "Content-Type: application/json" \
  -d '{"address":"0x...","amount":1000000000}'

# Send payment
cargo run -p ippan-wallet -- send \
  --to "0x..." \
  --amount 1000000 \
  --wallet test_wallet.json

# Verify
curl http://localhost:8080/account/0x.../payments | jq
```

**Handle Registration:**
```bash
cargo run -p ippan-wallet -- register-handle \
  --handle "@alice" \
  --wallet test_wallet.json
```

---

### Phase 3: Release Candidate (RC)

#### 3.1 Create RC Tag

```bash
git checkout master
git pull origin master

# Ensure clean working tree
git status

# Create RC tag
git tag -a v1.1.0-rc1 -m "$(cat <<'EOF'
IPPAN v1.1.0-rc1: Release Candidate 1

New Features:
- Feature X: Description
- Feature Y: Description

Bug Fixes:
- Fix for issue #123
- Fix for issue #456

Known Issues:
- None

Testing:
- 512-round DLC simulation: PASS
- Determinism harness: PASS (digest: abc123...)
- Coverage: 85%

Artifacts:
- Linux binary: v1.1.0-rc1-linux-x86_64.tar.gz
- macOS binary: v1.1.0-rc1-darwin-aarch64.tar.gz
- Docker image: ippan/node:v1.1.0-rc1
EOF
)"

# Push tag
git push origin v1.1.0-rc1
```

#### 3.2 Build Release Artifacts

```bash
# Build for Linux (x86_64)
cargo build --release --target x86_64-unknown-linux-gnu
tar -czf ippan-v1.1.0-rc1-linux-x86_64.tar.gz \
  target/x86_64-unknown-linux-gnu/release/ippan-node

# Build for macOS (aarch64)
cargo build --release --target aarch64-apple-darwin
tar -czf ippan-v1.1.0-rc1-darwin-aarch64.tar.gz \
  target/aarch64-apple-darwin/release/ippan-node

# Build Docker image
docker build -t ippan/node:v1.1.0-rc1 -f Dockerfile.production .
docker push ippan/node:v1.1.0-rc1
```

#### 3.3 Generate SBOM

```bash
cargo install cargo-sbom
cargo sbom > ippan-sbom-v1.1.0-rc1.spdx.json

# Sign SBOM (optional)
gpg --detach-sign --armor ippan-sbom-v1.1.0-rc1.spdx.json
```

#### 3.4 Create GitHub Release

1. Go to https://github.com/dmrl789/IPPAN/releases
2. Click "Draft a new release"
3. Fill in:
   - **Tag:** `v1.1.0-rc1`
   - **Title:** `IPPAN v1.1.0-rc1`
   - **Description:** Copy from tag message + add:
     - Download links for binaries
     - Docker pull command
     - Testnet join instructions
     - Known issues
   - **Pre-release:** Check this box for RC
4. Attach binaries:
   - `ippan-v1.1.0-rc1-linux-x86_64.tar.gz`
   - `ippan-v1.1.0-rc1-darwin-aarch64.tar.gz`
   - `ippan-sbom-v1.1.0-rc1.spdx.json`
5. Click "Publish release"

#### 3.5 Announce RC

- Post to Discord/Telegram: "v1.1.0-rc1 is live! Join testnet: [link]"
- Update website: "Latest RC: v1.1.0-rc1"
- Email validators: "Please test v1.1.0-rc1 and report issues"

---

### Phase 4: Testnet Rollout

#### 4.1 Deploy to Testnet

**Prerequisites:**
- Testnet infrastructure ready (seed nodes, bootstap peers)
- Genesis config defined (see §4.2)
- Monitoring dashboards configured

**Deployment Steps:**

1. **Update seed nodes:**
   ```bash
   # SSH to seed node 1
   ssh seed1.testnet.ippan.io
   cd /opt/ippan
   
   # Stop old version
   systemctl stop ippan-node
   
   # Backup data
   tar -czf data-backup-$(date +%s).tar.gz data/
   
   # Update binary
   wget https://github.com/dmrl789/IPPAN/releases/download/v1.1.0-rc1/ippan-v1.1.0-rc1-linux-x86_64.tar.gz
   tar -xzf ippan-v1.1.0-rc1-linux-x86_64.tar.gz
   
   # Update config (if needed)
   # vim config.toml
   
   # Start new version
   systemctl start ippan-node
   
   # Check logs
   journalctl -u ippan-node -f
   ```

2. **Validate seed node:**
   ```bash
   curl http://seed1.testnet.ippan.io:8080/health | jq
   curl http://seed1.testnet.ippan.io:8080/metrics | grep ippan_version
   ```

3. **Repeat for remaining seed nodes**

4. **Monitor network health:**
   - Check Grafana dashboards
   - Verify peer count increasing
   - Confirm blocks finalizing

#### 4.2 Testnet Genesis Config

**File:** `config/testnet-genesis.json`

```json
{
  "network_id": "ippan-testnet-v1",
  "genesis_time": "2025-12-01T00:00:00Z",
  "initial_supply": 0,
  "genesis_accounts": [
    {
      "address": "i0x1111111111111111111111111111111111111111111111111111111111111111",
      "balance": 1000000000000,
      "nonce": 0,
      "note": "Testnet faucet"
    }
  ],
  "genesis_validators": [
    {
      "id": "validator-1",
      "public_key": "0x2222222222222222222222222222222222222222222222222222222222222222",
      "bond": 10000000000,
      "reputation": 9000
    },
    {
      "id": "validator-2",
      "public_key": "0x3333333333333333333333333333333333333333333333333333333333333333",
      "bond": 10000000000,
      "reputation": 9000
    },
    {
      "id": "validator-3",
      "public_key": "0x4444444444444444444444444444444444444444444444444444444444444444",
      "bond": 10000000000,
      "reputation": 9000
    }
  ],
  "consensus_params": {
    "round_duration_ms": 200,
    "finality_depth": 2,
    "min_validator_bond": 10000000000,
    "max_validators": 100
  },
  "emission_params": {
    "r0": 10000,
    "halving_rounds": 315000000,
    "supply_cap": 21000000000000
  }
}
```

**Usage:**
```bash
cargo run -p ippan-node -- --genesis config/testnet-genesis.json
```

---

### Phase 5: Stable Release

#### 5.1 Post-RC Period

- Wait 1-2 weeks for testnet feedback
- Fix critical bugs (release new RC if needed)
- Gather validator sign-offs

#### 5.2 Go/No-Go Decision

**Criteria:**
- [ ] No P0/P1 bugs reported on testnet
- [ ] Validators confirm stable operation (≥5 validators, ≥7 days uptime)
- [ ] Security audit complete (for major releases)
- [ ] Documentation complete
- [ ] Mainnet deployment plan approved

**Decision Makers:**
- Lead Architect (Ugo Giuliani)
- Security Lead
- Release Manager

#### 5.3 Create Stable Tag

```bash
git checkout master
git pull origin master

git tag -a v1.1.0 -m "IPPAN v1.1.0: Stable Release"
git push origin v1.1.0
```

#### 5.4 Publish Stable Release

Same as RC, but:
- Uncheck "Pre-release" in GitHub
- Update website: "Latest Stable: v1.1.0"
- Announce via all channels

---

### Phase 6: Mainnet Deployment

#### 6.1 Pre-Deployment Checklist

- [ ] Stable release published
- [ ] Validators notified 7 days in advance
- [ ] Deployment window scheduled (low-traffic period)
- [ ] Rollback plan prepared
- [ ] Monitoring alerts configured

#### 6.2 Deployment Process

**Coordinated Upgrade:**
1. Announce maintenance window: "Mainnet upgrade to v1.1.0 on 2025-12-15 at 00:00 UTC"
2. Validators upgrade in stages:
   - Stage 1: 33% of validators
   - Wait 1 hour, monitor
   - Stage 2: 33% of validators
   - Wait 1 hour, monitor
   - Stage 3: Remaining validators
3. Verify network health after each stage

**Emergency Rollback:**
If critical issues detected:
```bash
# Revert to previous version
systemctl stop ippan-node
cp /opt/ippan/bin/ippan-node-v1.0.0 /opt/ippan/bin/ippan-node
systemctl start ippan-node
```

#### 6.3 Post-Deployment

- Monitor for 24 hours
- Send "All clear" notification
- Update status page: "v1.1.0 deployed successfully"

---

## Hotfix Process

For critical bugs in production:

1. **Create hotfix branch:**
   ```bash
   git checkout -b hotfix/v1.1.1 v1.1.0
   ```

2. **Fix bug + add test:**
   ```bash
   # Make fix in hotfix branch
   # Add regression test
   cargo test
   ```

3. **Tag and release:**
   ```bash
   git tag -a v1.1.1 -m "Hotfix: Fix critical bug #789"
   git push origin v1.1.1
   ```

4. **Emergency deployment:**
   - Skip RC phase (if truly critical)
   - Deploy to mainnet within 24 hours
   - Notify all validators immediately

5. **Merge back to master:**
   ```bash
   git checkout master
   git merge hotfix/v1.1.1
   git push origin master
   ```

---

## Release Smoke Suite

**Location:** `scripts/smoke_release.sh`

Minimal tests that MUST pass before release:

```bash
#!/bin/bash
set -e

echo "=== IPPAN Release Smoke Tests ==="

# 1. Build
echo "[1/6] Building..."
cargo build --release

# 2. Start node
echo "[2/6] Starting node..."
cargo run --release -p ippan-node -- --config config/devnet.toml &
NODE_PID=$!
sleep 10

# 3. Health check
echo "[3/6] Health check..."
curl -f http://localhost:8080/health || exit 1

# 4. Send payment
echo "[4/6] Payment flow..."
# (payment test commands)

# 5. Verify metrics
echo "[5/6] Metrics check..."
curl -f http://localhost:9615/metrics | grep ippan_version || exit 1

# 6. Cleanup
echo "[6/6] Cleanup..."
kill $NODE_PID

echo "✅ All smoke tests passed"
```

**Run:**
```bash
bash scripts/smoke_release.sh
```

---

## Changelog Format

**File:** `CHANGELOG.md`

```markdown
# Changelog

## [1.1.0] - 2025-12-15

### Added
- Feature X: Detailed description
- Feature Y: Detailed description

### Changed
- Improved Z: Description
- Optimized A: Description

### Fixed
- Bug #123: Description
- Bug #456: Description

### Security
- Patched vulnerability CVE-2024-XXXX

### Deprecated
- API endpoint /old/path (use /new/path instead)

### Removed
- Removed deprecated feature from v1.0

## [1.0.1] - 2025-11-30

### Fixed
- Hotfix for critical bug #789
```

---

## Summary

| Phase | Duration | Output |
|-------|----------|--------|
| **Planning** | 2-4 weeks | Release checklist, milestone |
| **Testing** | 1-2 weeks | Test reports, green CI |
| **RC** | 1 day | RC tag, artifacts, GitHub release |
| **Testnet** | 1-2 weeks | Testnet deployment, validator feedback |
| **Stable** | 1 day | Stable tag, release notes |
| **Mainnet** | 1-3 days | Production deployment |

**Total Time:** 5-9 weeks from feature freeze to mainnet

---

## References

- [Semantic Versioning](https://semver.org/)
- [V1_RC_TAGGING_INSTRUCTIONS.md](../../V1_RC_TAGGING_INSTRUCTIONS.md)
- [TESTNET_JOIN_GUIDE.md](../../TESTNET_JOIN_GUIDE.md)
- [docs/operators/production-validator-runbook.md](../operators/production-validator-runbook.md)

---

**Maintainers:**  
- Release Manager: TBD
- Lead Architect: Ugo Giuliani

**Last Release:** v1.0.0-rc1 (2025-11-24)
