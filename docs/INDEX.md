# IPPAN Documentation Index

> **Single entry point** to all essential IPPAN documentation for auditors and developers.

**Readiness Dashboard:** [docs/READINESS_STATUS.md](READINESS_STATUS.md)

---

## Start Here (2-Minute Orientation)

### What IPPAN Is
- **Real blockchain** with IPPAN Time and HashTimer systems for temporal ordering
- **D-GBDT consensus**: Deterministic AI-driven validator selection (no floating-point)
- **Current version**: v0.9.0-rc1 (Release Candidate)
- **Status**: Testnet/devnet ready, not production

### Determinism Rules
- **No f32/f64** in runtime code (enforced in CI)
- **Canonical hashing**: Blake3 with deterministic JSON serialization
- **Model hash lock**: D-GBDT models verified at startup via expected hash
- **Cross-platform**: Bit-for-bit identical results on x86_64, aarch64, ARM, RISC-V

### How to Run Core Checks

**Windows (PowerShell):**
```powershell
# Format check
cargo fmt --all -- --check

# Linting
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo test --workspace --all-targets

# Verify model hash
cargo run -p ippan-ai-core --bin verify_model_hash -- config/dlc.toml

# Build release binaries
cargo build --workspace --release
```

**CI Links:**
- Main CI: `.github/workflows/ci.yml` - Format, lint, build, test
- AI Determinism: `.github/workflows/ai-determinism.yml` - Cross-platform determinism
- Nightly Validation: `.github/workflows/nightly-validation.yml` - Extended validation
- Security: `.github/workflows/codeql.yml` - CodeQL security analysis

---

## For Developers

### Build & Test

- **Localnet Quickstart**: [docs/LOCALNET_QUICKSTART.md](LOCALNET_QUICKSTART.md) - Get localnet running on Windows in <10 minutes
- **Developer Journey**: [docs/dev/developer-journey.md](dev/developer-journey.md) - Complete onboarding guide
- **Local Full-Stack**: [docs/dev/local-full-stack.md](dev/local-full-stack.md) - Detailed setup and usage
- **SDK Overview**: [docs/dev/sdk-overview.md](dev/sdk-overview.md) - SDK integration guide
- **Developer Guide**: [docs/DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) - General development practices

### Localnet (Docker)

- **Quickstart**: [docs/LOCALNET_QUICKSTART.md](LOCALNET_QUICKSTART.md) - One-command start with Docker
- **Three-Node Demo**: [docs/localnet_three_node_demo.md](localnet_three_node_demo.md) - Multi-node setup
- **Docker Compose**: `localnet/docker-compose.full-stack.yaml` - Full stack configuration

### RPC / API Surface

- **API Explorer Surface**: [docs/API_EXPLORER_SURFACE.md](API_EXPLORER_SURFACE.md) - Read-only RPC routes for explorers
- **API Versioning Policy**: [docs/API_VERSIONING_POLICY.md](API_VERSIONING_POLICY.md) - Version contract and deprecation windows
- **Payment API Guide**: [docs/PAYMENT_API_GUIDE.md](PAYMENT_API_GUIDE.md) - Payment transaction API
- **End-to-End Payment Demo**: [docs/payments/demo_end_to_end_payment.md](payments/demo_end_to_end_payment.md) - Complete payment flow

### AI / D-GBDT Workflows

- **D-GBDT Documentation**: [docs/ai/D-GBDT.md](ai/D-GBDT.md) - Complete guide to deterministic AI
  - Fixed-point SCALE policy (micro-precision arithmetic)
  - Feature schema and validator telemetry
  - Model lifecycle: train → canonicalize → hash → load → cache
  - Model rotation procedures
  - Determinism checklist and troubleshooting
- **AI Implementation Guide**: [docs/AI_IMPLEMENTATION_GUIDE.md](AI_IMPLEMENTATION_GUIDE.md) - AI system overview
- **AI Model Lifecycle**: [docs/AI_MODEL_LIFECYCLE.md](AI_MODEL_LIFECYCLE.md) - Model training and deployment
- **Model Registry**: [docs/ai/MODEL_REGISTRY.md](ai/MODEL_REGISTRY.md) - Model storage and verification
- **AI Training Dataset**: [docs/AI_TRAINING_DATASET.md](AI_TRAINING_DATASET.md) - Training data collection
- **AI Security**: [docs/AI_SECURITY.md](AI_SECURITY.md) - Security considerations
- **AI Status API**: [docs/AI_STATUS_API.md](AI_STATUS_API.md) - AI metrics and status endpoints

### Soak Tests

- **Soak Testing Guide**: [docs/soak/README.md](soak/README.md) - Long-run DLC determinism and stability tests
  - Manual runs via GitHub Actions
  - Scheduled weekly runs (Sundays 03:00 UTC)
  - Local reproduction steps
  - Artifact analysis

### Fuzz Tests

- **Fuzz Testing Guide**: [docs/fuzz/README.md](fuzz/README.md) - Property-based fuzzing for critical components
  - Fuzz targets: canonical_hash, rpc_body_limit, proof_parsing
  - Smoke tests (PR gate, 60 seconds)
  - Nightly long-run tests (15+ minutes)
  - Crash reproduction and analysis

### Development Workflow

- **Trunk Branch Development**: [docs/dev_workflow/MAIN_BRANCH_DEVELOPMENT.md](dev_workflow/MAIN_BRANCH_DEVELOPMENT.md) - Trunk-based workflow (master → main)
- **CI/CD Guide**: [docs/dev_workflow/CI_STABILIZATION_SUMMARY.md](dev_workflow/CI_STABILIZATION_SUMMARY.md) - CI pipeline overview
- **Workflow README**: [docs/dev_workflow/README.md](dev_workflow/README.md) - Development workflow details

---

## For Auditors

### Repro Steps (Exact Commands)

See **"How to Run Core Checks"** section above for exact PowerShell commands.

**Additional Audit Commands:**
```powershell
# Check for floating-point operations (should be empty)
rg -n "f32|f64" crates/ --type rust | grep -v "test\|\.disabled\|deprecated\|//"

# Verify no f32/f64 in runtime (CI gate)
# See: .github/workflows/no-float-runtime.yml

# Run AI determinism tests
cargo test -p ippan-ai-core --test determinism

# Run DLC consensus tests
cargo test -p ippan-consensus-dlc --test fairness_invariants
```

### Protocol / Spec Docs

- **Protocol Specification**: [docs/spec/IPPAN_PROTOCOL_SPEC.md](spec/IPPAN_PROTOCOL_SPEC.md) - Complete protocol spec
- **Consensus Research Summary**: [docs/CONSENSUS_RESEARCH_SUMMARY.md](CONSENSUS_RESEARCH_SUMMARY.md) - Navigation guide to consensus docs
- **Academic Whitepaper**: [docs/BEYOND_BFT_DETERMINISTIC_LEARNING_CONSENSUS.md](BEYOND_BFT_DETERMINISTIC_LEARNING_CONSENSUS.md) - Peer-reviewed consensus model
- **HashTimer Implementation**: [docs/protocol/HASHTIMER_IMPLEMENTATION_ANALYSIS.md](protocol/HASHTIMER_IMPLEMENTATION_ANALYSIS.md) - Temporal ordering analysis
- **Consensus Block Creation**: [docs/consensus/ippan_block_creation_validation_consensus.md](consensus/ippan_block_creation_validation_consensus.md) - Block validation logic
- **Consensus README**: [docs/consensus/README.md](consensus/README.md) - Consensus overview

### Security Posture

- **Security Threat Model**: [docs/audit/SECURITY_THREAT_MODEL.md](audit/SECURITY_THREAT_MODEL.md) - Threat analysis
- **Security Guide**: [docs/SECURITY_GUIDE.md](SECURITY_GUIDE.md) - Security practices
- **Security and Audit Playbook**: [docs/SECURITY_AND_AUDIT_PLAYBOOK.md](SECURITY_AND_AUDIT_PLAYBOOK.md) - Audit procedures
- **Dependency Audit**: [docs/audit/CARGO_DEPENDENCY_AUDIT.md](audit/CARGO_DEPENDENCY_AUDIT.md) - Dependency policy and audit
- **Dependency Fixes**: [docs/audit/CARGO_DEPENDENCY_FIXES_APPLIED.md](audit/CARGO_DEPENDENCY_FIXES_APPLIED.md) - Applied fixes
- **CI Gates**: See `.github/workflows/` for all CI gates (fmt, clippy, tests, determinism, no-float-runtime)
- **Determinism Harness**: [docs/ai/D-GBDT.md](ai/D-GBDT.md) - Determinism validation procedures

### Readiness Plan

- **Readiness 100% Plan**: [docs/READINESS_100_PLAN.md](READINESS_100_PLAN.md) - Complete readiness tracking
  - P0 items (release blockers)
  - P1 items (post-RC)
  - Exact commands for auditors
  - Definition of done

### Audit Package

- **Audit Index**: [docs/audit/AUDIT_INDEX.md](audit/AUDIT_INDEX.md) - Entry point for audit materials
- **Audit Ready Checklist**: [docs/audit/AUDIT_READY.md](audit/AUDIT_READY.md) - Pre-audit checklist
- **RC1 Audit Package**: [docs/audit/AUDIT_PACKAGE_V1_RC1_2025_11_24.md](audit/AUDIT_PACKAGE_V1_RC1_2025_11_24.md) - Complete audit package
- **Audit Summary**: [docs/audit/AUDIT_SUMMARY.md](audit/AUDIT_SUMMARY.md) - Audit status summary
- **Main Audit Checklist**: [docs/audit/CHECKLIST_AUDIT_MAIN.md](audit/CHECKLIST_AUDIT_MAIN.md) - Comprehensive checklist
- **Go/No-Go Checklist**: [docs/audit/GO_NO_GO_CHECKLIST.md](audit/GO_NO_GO_CHECKLIST.md) - Release gate checklist
- **Bug Triage Workflow**: [docs/audit/AUDIT_BUG_TRIAGE_WORKFLOW.md](audit/AUDIT_BUG_TRIAGE_WORKFLOW.md) - Bug handling procedures
- **Patch Re-test Protocol**: [docs/audit/AUDIT_PATCH_RETEST_PROTOCOL.md](audit/AUDIT_PATCH_RETEST_PROTOCOL.md) - Re-testing procedures

---

## Operations / Release

### CI Workflows Overview

- **Main CI** (`.github/workflows/ci.yml`): Format, lint, build, test on every push/PR
- **AI Determinism** (`.github/workflows/ai-determinism.yml`): Cross-platform determinism validation
- **Nightly Validation** (`.github/workflows/nightly-validation.yml`): Extended test suite with coverage
- **Fuzz Smoke** (`.github/workflows/fuzz-smoke.yml`): Quick fuzz tests (60 seconds) on PRs
- **Fuzz Nightly** (`.github/workflows/fuzz-nightly.yml`): Long-run fuzz tests (15+ minutes, weekly)
- **Soak DLC** (`.github/workflows/soak-dlc-longrun.yml`): Long-run DLC determinism tests (hours)
- **Security** (`.github/workflows/codeql.yml`): CodeQL security analysis
- **No Float Runtime** (`.github/workflows/no-float-runtime.yml`): Enforces no f32/f64 in runtime code

### How to Run Manual Workflows

**AI Determinism:**
- Trigger: Automatic on changes to AI/consensus code
- Manual: GitHub Actions → AI Determinism → Run workflow

**Soak Tests:**
- Manual: GitHub Actions → Soak — DLC Long-Run Determinism → Run workflow
- Configure: `minutes` (default: 180), `profile` (smoke/standard/heavy)
- Scheduled: Sundays 03:00 UTC

**Fuzz Nightly:**
- Manual: GitHub Actions → Fuzz Nightly → Run workflow
- Configure: `minutes` (default: 15)
- Scheduled: Sundays 02:00 UTC

**Nightly Validation:**
- Manual: GitHub Actions → Nightly Full Validation → Run workflow
- Scheduled: Daily at 02:00 UTC

### Release Checklist / RC Process

- **Release Process**: [docs/release/RELEASE_PROCESS.md](release/RELEASE_PROCESS.md) - Complete release procedures
- **v1 Mainnet Checklist**: [docs/release/v1-mainnet-checklist.md](release/v1-mainnet-checklist.md) - Mainnet readiness checklist
- **Release Engineering**: [docs/RELEASE_ENGINEERING.md](RELEASE_ENGINEERING.md) - Release automation and procedures
- **Release Notes**: [docs/release-notes/IPPAN_v0.9.0_RC1.md](release-notes/IPPAN_v0.9.0_RC1.md) - Current RC release notes

### Deployment

- **Deployment Guide**: [docs/DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md) - Production deployment procedures
- **Automated Deployment**: [docs/automated-deployment-guide.md](automated-deployment-guide.md) - GitHub Actions deployment
- **Node Operator Guide**: [docs/operators/NODE_OPERATOR_GUIDE.md](operators/NODE_OPERATOR_GUIDE.md) - Production node operations
- **Running RC Node**: [docs/operators/running-ippan-rc-node.md](operators/running-ippan-rc-node.md) - RC testnet guide
- **Production Deployment**: [docs/operators/PRODUCTION_DEPLOYMENT_GUIDE.md](operators/PRODUCTION_DEPLOYMENT_GUIDE.md) - Production rollout guide
- [IPPAN Time — Monotonic TimeState](ops/time.md)
- **Runbook: Devnet rollback + drift SOP**: [docs/ops/runbooks/devnet-rollback-and-drift.md](ops/runbooks/devnet-rollback-and-drift.md)

### Rollouts

- [2025-12-17 — ippan-time monotonic fix (devnet)](ops/rollouts/2025-12-17-ippan-time-monotonic.md)
- Devnet verification: `scripts/ops/check-devnet.(sh|ps1)` checks `/status`, `/peers`, `/time` on `:8080` and `sha256sum` of `/usr/local/bin/ippan-node` via SSH.

---

## Additional Resources

### Architecture & Design

- **Architecture Overview**: [docs/architecture_overview.md](architecture_overview.md) - System architecture
- **IPPAN Architecture Update**: [docs/IPPAN_Architecture_Update_v1.0.md](IPPAN_Architecture_Update_v1.0.md) - Architecture v1.0
- **Consensus Network Mempool**: [docs/IPPAN_Consensus_Network_Mempool_v2.md](IPPAN_Consensus_Network_Mempool_v2.md) - Network architecture

### Economics & Protocol

- **Tokenomics**: [docs/protocol/TOKENOMICS.md](protocol/TOKENOMICS.md) - Token economics
- **DAG Fair Emission**: [docs/DAG_FAIR_EMISSION.md](DAG_FAIR_EMISSION.md) - Emission system
- **Fees and Emission**: [docs/FEES_AND_EMISSION.md](FEES_AND_EMISSION.md) - Fee structure

### Testing

- **Testing Guide**: [docs/testing/comprehensive-testing-phase1.md](testing/comprehensive-testing-phase1.md) - Testing strategy
- **Adversarial and Fuzzing**: [docs/testing/adversarial-and-fuzzing.md](testing/adversarial-and-fuzzing.md) - Security testing

### Users

- **Getting Started**: [docs/users/getting-started.md](users/getting-started.md) - User onboarding
- **Handles and Addresses**: [docs/users/handles-and-addresses.md](users/handles-and-addresses.md) - Handle system

---

**Last Updated**: 2025-01-XX  
**Maintained By**: IPPAN Team  
**Questions?** See [docs/README.md](README.md) for doc organization details.

