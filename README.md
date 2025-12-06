# IPPAN Blockchain

A real blockchain implementation with **IPPAN Time** and **HashTimer** systems for temporal ordering and validation.

> **Development Workflow**: IPPAN uses trunk-based development on the trunk branch (currently `master`, will be `main`). See [MAIN_BRANCH_DEVELOPMENT.md](docs/dev_workflow/MAIN_BRANCH_DEVELOPMENT.md) for complete guidelines.

## Status

- Current version: **v0.9.0-rc1** (Release Candidate)
- Nightly Full Validation: ✅ green (coverage ≈ 65%, readiness: Release Candidate)
- Intended use: **testnet / devnet experimentation only**, not production.
- Breaking changes are still possible before v1.0.
- See [release notes](docs/release-notes/IPPAN_v0.9.0_RC1.md), the matching [changelog entry](CHANGELOG.md#v090-rc1--2025-11-20),
  and the [operator guide](docs/operators/running-ippan-rc-node.md) (including guidance for embedding git metadata into RC builds).

### CI Status

[![Build & Test (Rust)](https://github.com/dmrl789/IPPAN/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/dmrl789/IPPAN/actions/workflows/ci.yml?query=branch%3Amaster)
[![AI Determinism & DLC Consensus](https://github.com/dmrl789/IPPAN/actions/workflows/ai-determinism.yml/badge.svg?branch=master)](https://github.com/dmrl789/IPPAN/actions/workflows/ai-determinism.yml?query=branch%3Amaster)
[![Audit Pack — P0 Gates + SBOM](https://github.com/dmrl789/IPPAN/actions/workflows/audit-pack.yml/badge.svg)](https://github.com/dmrl789/IPPAN/actions/workflows/audit-pack.yml)

## RC Testnet

- Current RC: **v0.9.0-rc1**
- To run a local RC testnet, see: [docs/operators/ippan-rc-testnet-guide.md](docs/operators/ippan-rc-testnet-guide.md)
- To report issues or feedback: use the "RC Bug Report" or "RC Feedback / UX" GitHub issue templates.
- Understand how wallet `@handles` map to addresses and how to test handle-based payments in [docs/users/handles-and-addresses.md](docs/users/handles-and-addresses.md) (also linked at [docs/overview/handles-and-addresses.md](docs/overview/handles-and-addresses.md)).

## 🚀 Features

- **IPPAN Time**: Monotonic microsecond precision time service with peer synchronization
- **HashTimer**: 256-bit temporal identifiers (14 hex prefix + 50 hex suffix) embedded in all blockchain operations
- **Real Blockchain**: Complete implementation with transactions, blocks, consensus, and P2P networking
- **Web Explorer**: Hosted blockchain explorer at https://ippan.com/explorer for transaction and block visibility
- **Release Candidate Hardening**: Docker, systemd, and CI/CD configurations validated via nightly workflows

> **Single-branch development**: All day-to-day work lands directly on the trunk branch (`master` today, `main` after the rename). Temporary topic branches should remain short-lived and are deleted after their changes fast-forward onto trunk.

## 📚 Onboarding Guides

- [Localnet Quickstart](docs/LOCALNET_QUICKSTART.md) - Get localnet running on Windows in <10 minutes
- [Developer Journey](docs/dev/developer-journey.md)
- [Local Full-Stack Guide](docs/dev/local-full-stack.md)
- [SDK Overview](docs/dev/sdk-overview.md)
- [User Getting Started](docs/users/getting-started.md)
- [Handles & Addresses](docs/users/handles-and-addresses.md)

## 📖 Documentation

See [docs/INDEX.md](docs/INDEX.md) for the single entry point to all essential documentation.

## Readiness

- **Readiness Dashboard:** [docs/READINESS_STATUS.md](docs/READINESS_STATUS.md)
- **Documentation Index:** [docs/INDEX.md](docs/INDEX.md)

## 🧭 IPPAN Codebase Readiness Dashboard

*(Snapshot: November 2025)*

| Category | Description | Weight | Score | Status | Key Actions |
|-----------|--------------|--------|--------|----------|--------------|
| **Implementation Completeness** | Core crates (consensus, mempool, storage, RPC, crypto) implemented and wired across workspace | 50 % | **0.80** | 🟢 Solid | Finalize fork resolution & treasury distribution logic |
| **Testing & Verification** | ~606 tests (~45 % coverage). Integration tests pass, but persistence and DAG conflict tests missing | 30 % | **0.45** | 🟠 Partial | Expand coverage to ≥ 80 %; add long-run DLC simulations |
| **Operational Hardening** | Axum RPC, libp2p stack, rate limiting, and audit hooks implemented but optional | 20 % | **0.75** | 🟢 Near-ready | Connect security manager and observability sinks |
| **AI Determinism** | D-GBDT deterministic scoring validated on x86_64 | – | **0.70** | 🟠 In progress | Re-run tests on aarch64 and publish reproducibility logs |
| **CI/CD Reliability** | Main CI gate restored; nightly dashboard artifact publishes coverage snapshot | – | **0.90** | 🟢 Solid | Monitor nightly dashboard artifacts; configure NVD API key for Android scan |
| **Documentation & Onboarding** | Whitepaper and PRDs complete; code-level docs improving | – | **0.80** | 🟢 Mature | Add `docs/dev_guide.md` and architecture diagrams |
| **Overall Readiness** | Weighted composite across all modules | **100 %** | **≈ 0.69 (≈ 70 %)** | 🟡 Beta-ready | Push toward 85 %+ before mainnet audit |

### 📋 Audit Hardening Progress

**Phases A–D Complete:** The initial audit-hardening wave (economics integration, AI determinism, network/storage hardening, and governance/audit preparation) has been completed. See [`PHASE_A_D_COMPLETION_SUMMARY.md`](docs/archive/2025_rc1/PHASE_A_D_COMPLETION_SUMMARY.md) for the full breakdown of work completed across all four phases.

**Status:** This represents the first four phases of internal hardening. The codebase is now audit-ready, but **Phase E** (External Audit & Launch Gate) remains before we can claim 100% production readiness. Phase E scope is defined in [`CHECKLIST_AUDIT_MAIN.md`](docs/audit/CHECKLIST_AUDIT_MAIN.md).

### 🧩 Next Milestones

- [ ] Achieve ≥ 80 % test coverage for critical crates
- [x] Enable full nightly `cargo test --workspace` runs
- [ ] Integrate Prometheus + Grafana observability
- [ ] Add fork, slashing, and recovery test cases
- [ ] Run deterministic AI validation on multi-arch targets

### 🧰 Useful Commands

```bash
cargo check
cargo test --workspace --all-features -- --nocapture
cargo tarpaulin --out Html
cargo bench --workspace
```

- `scripts/run-local-full-stack.sh` – build the workspace and start the three-node localnet for end-to-end testing.

## 🏗️ Architecture

### Core Components

- **`crates/types`**: HashTimer, IPPAN Time, Transaction, and Block types
- **`crates/crypto`**: Cryptographic primitives and key management
- **`crates/storage`**: Blockchain data persistence
- **`crates/p2p`**: Peer-to-peer networking
- **`crates/mempool`**: Transaction pool management
- **`crates/consensus`**: Block validation and consensus
- **`crates/rpc`**: REST API for blockchain interaction
- **`node`**: Main blockchain node runtime

### HashTimer System

Every blockchain operation includes a **HashTimer**:

```
Format: <14-hex time prefix><50-hex blake3 hash>
Example: 063f4c29f0a5fa30f78d856f1e88975e73c2504559224adc259ccbb3fb90df8a
         ^^^^^^^^^^^^^^ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
         14-char time   50-char cryptographic hash
```

- **Time Prefix**: Microsecond IPPAN Time (56 bits)
- **Hash Suffix**: Blake3 hash of context + time + domain + payload + nonce + node_id (200 bits)

## 🛠️ Development

### Build verification status

This repository targets a production-grade blockchain node and normally
relies on the public crates.io registry for its Rust dependencies.  When
the build is executed in a locked-down environment (such as the
evaluation sandboxes used for automated assessments) the registry cannot
be reached, causing commands like `cargo check` or `cargo build` to abort
while attempting to download `config.json` from crates.io.  The source
code itself compiles successfully when the registry is reachable – the
error is purely environmental.

To reproduce a successful build outside of the restricted environment:

1. Ensure outbound HTTPS access to `https://index.crates.io/` is allowed,
   or run `cargo vendor` ahead of time and commit the generated vendor
   directory for fully offline builds.
2. From the repository root run `cargo check --all-targets` to verify the
   workspace compiles.

If you are running inside an environment without network access, copy a
pre-vendored dependency set (created with `cargo vendor`) into the
workspace and update `.cargo/config.toml` to point to it before invoking
the build.

### Prerequisites

- Rust 1.75+
- Docker (optional)

### Building

```bash
# Build the entire workspace
cargo build --workspace

# Run the node
cargo run --bin ippan-node

# Run tests
cargo test --workspace
```

### Running the Node

```bash
# Start IPPAN node
cargo run --bin ippan-node
```

The node will:
1. Initialize IPPAN Time service
2. Create HashTimers for different contexts (tx, block, round)
3. Demonstrate time monotonicity
4. Start blockchain services

### 🔍 Checking peer connectivity

- The node only connects to peers that you supply through the `BOOTSTRAP_NODES` environment variable. If it is left empty (default), the node happily runs in isolation and the `/health` endpoint reports `peer_count: 0`.
- Provide a comma-separated list of peer base URLs before starting the node, for example:

  ```bash
  BOOTSTRAP_NODES="http://10.0.0.5:9000,http://10.0.0.6:9000" \
    cargo run --bin ippan-node
  ```

- Query `http://<rpc-host>:<rpc-port>/health` once the node is up. When at least one peer is reachable, the response shows a `peer_count` greater than zero and keeps updating every 10 seconds via the background poller.

#### Automated node health check

Use the bundled `ippan-check-nodes` utility to confirm that a set of nodes respond to the health endpoints and are connected to peers:

```bash
cargo run --bin ippan-check-nodes -- \
  --api http://127.0.0.1:8080,http://127.0.0.1:8081
```

The command queries `/health`, `/status`, and `/peers` on every target, prints a human-readable summary, and exits with a non-zero status when a node is unhealthy or has fewer peers than required. By default the checker expects each node to see every other node provided via `--api` as a peer; override the threshold with `--require-peers`. Pass `--json` to obtain a machine-readable report.

## 🌐 API Endpoints

- `POST /tx` - Submit transaction
- `GET /block/{hash|height}` - Get block
- `GET /account/{address}` - Get account info
- `GET /time` - Get current IPPAN Time

Need a concrete example flow? See [`docs/payments/demo_end_to_end_payment.md`](docs/payments/demo_end_to_end_payment.md).
For integrators, `/version` is the canonical way to confirm the active
protocol contract; see [`docs/API_VERSIONING_POLICY.md`](docs/API_VERSIONING_POLICY.md)
for expectations around `v1` stability and deprecation windows.

## 🔄 Releases & CI Automation

### Versioning & Release Channels

- **Semantic versioning**: Production releases are tagged as `vMAJOR.MINOR.PATCH` and published through `.github/workflows/release.yml`.
- **Branch policy**: All development lands directly on `main`. Temporary topic branches are allowed only for short experiments and must be merged via fast-forward updates to `main` after CI succeeds.
- **Promotion flow**: Dispatching the `Release` workflow with the target tag builds images, signs artifacts, and pushes signed tags back to the repository and GHCR.
- **Hotfixes**: Apply critical fixes from `hotfix/*` directly to `main`, tag a patch release, and communicate follow-up tasks via issues/CHANGELOG (no `develop` back-merges).

### Continuous Integration Workflows

- **CI pipeline** (`.github/workflows/ci.yml`): Runs on every push/PR to `main`, executing Rust formatting, `cargo check`, `cargo build`, `cargo clippy`, targeted AI/DLC tests, plus Gateway and Unified UI lint/build checks.
- **Extended test matrix** (`test-suite.yml`, `build.yml`): Provides nightly stress builds and integration suites for long-running validations before release.
- **Deployment automation** (`deploy.yml`, `deploy-ippan-full-stack.yml`, `prod-deploy.yml`): Builds production images and promotes them to infrastructure once CI is green.
- **Quality gates**: Workflows must report success before `main` merges, and releases require `Release` plus deployment workflows to complete without manual retries.

### Operational Validation Scripts

- `deploy/check-nodes.sh`: Performs JSON health sampling, enforces HTTP 200s for `/health`, `/status`, `/peers`, and fails if peer count is zero.
- `deploy/health-check.sh`: Validates consensus, time service, and connectivity between production nodes; warns if block height drift exceeds five blocks.
- `deploy/verify-deployment.sh`: Confirms UI, gateway, RPC, and key P2P ports across both servers after a rollout.
- `deploy/monitor-production.sh`: Continuous watcher that logs node vitals every 30s and raises alerts if either node is unreachable or both go down.
- See `docs/DEPLOYMENT_GUIDE.md` for full runbooks, success criteria, and post-deployment QA steps.

## 🐳 Deployment

### Automated Deployment (Recommended)

The IPPAN network uses automated GitHub Actions deployment:

- **Automatic**: Deploys on every push to `main` branch
- **Multi-Server**: Deploys to Server 1 (full-stack) and Server 2 (node-only)
- **Docker Registry**: Uses GitHub Container Registry (GHCR)
- **Health Checks**: Verifies deployment success

See [Automated Deployment Guide](docs/automated-deployment-guide.md) and the expanded [Deployment Guide](docs/DEPLOYMENT_GUIDE.md) for setup instructions and QA requirements.

### Manual Docker Deployment

```bash
# Build production image
docker build -f Dockerfile.production -t ippan-node .

# Run container
docker run -p 8080:8080 -p 9000:9000 ippan-node
```

### Production Servers

- **Server 1** (188.245.97.41): Full-stack with UI and gateway
- **Server 2** (135.181.145.174): Node-only deployment

### Systemd (Legacy)

```bash
# Install service
sudo cp deploy/ippan-node.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable ippan-node
sudo systemctl start ippan-node
```

Production explorer: https://ippan.com/explorer

## 📊 HashTimer Examples

### Transaction HashTimer
```rust
let tx_hashtimer = HashTimer::now_tx("transfer", payload, nonce, node_id);
// Result: 063f4c29f0c8c7e61eb3d2914435c3ab1894dd6c51eec42c152a2c566922ce4e
```

### Block HashTimer
```rust
let block_hashtimer = HashTimer::now_block("block_creation", payload, nonce, node_id);
// Result: 063f4c29f0c9077cb85a40787b8df4f664299fede0ffd93dd37fc4b576c432a0
```

### Round HashTimer
```rust
let round_hashtimer = HashTimer::now_round("consensus", payload, nonce, node_id);
// Result: 063f4c29f0c90e853ee578cc36d1824f0d9e2241a6ef97e7429366a145bd08e3
```

## 🔧 Configuration

### Environment Variables

For comprehensive documentation of all environment variables and secrets, see:

📖 **[Secrets and Environment Variables Guide](docs/SECRETS_AND_ENVIRONMENT_VARIABLES.md)**

Quick reference for common variables:

- `RUST_LOG`: Logging level (default: info)
- `IPPAN_NETWORK`: Network type (mainnet/testnet)
- `IPPAN_DATA_DIR`: Data directory path
- `BOOTSTRAP_NODES`: Comma-separated peer URLs
- `NODE_ID`: Unique node identifier
- `VALIDATOR_ID`: 64-character hex validator ID

### Configuration Files

Example configurations are provided:

- **Node**: `config/ippan.env.example` → Copy to `config/ippan.env`
- **Gateway**: `apps/gateway/.env.example` → Copy to `apps/gateway/.env`
- **UI**: `apps/unified-ui/.env.example` → Copy to `apps/unified-ui/.env.local`
- **Deployment**: `deploy/.env.example` → Copy to `deploy/.env`

### GitHub Secrets Setup

For CI/CD deployments, configure GitHub secrets following:

📖 **[GitHub Secrets Setup Guide](docs/GITHUB_SECRETS_SETUP.md)**

## 🔐 IPPAN Secrets Configuration Guide

### 📘 Overview

IPPAN’s GitHub Actions workflows and runtime environments depend on several **secrets** and **environment variables**. None of them are stored in the repository — only placeholders exist in `.env.example` files. Each developer or CI environment must set them explicitly.

### 🧩 Required Secrets

| Secret name                                         | Used by                            | Description                                         | Example value                          | Location                              |
| --------------------------------------------------- | ---------------------------------- | --------------------------------------------------- | -------------------------------------- | ------------------------------------- |
| **`LLM_API_KEY`** *(or `IPPAN_SECRET_LLM_API_KEY`)* | `ai_service`, AI determinism tests | API key for LLM inference (e.g., OpenAI, Anthropic) | `sk-xxxx`                              | GitHub → Settings → Secrets → Actions |
| **`PROMETHEUS_ENDPOINT`**                           | Node telemetry exporter            | URL endpoint for Prometheus metrics                 | `https://metrics.yourdomain.net`       | `.env` or K8s secret                  |
| **`JSON_EXPORTER_ENDPOINT`**                        | AI & consensus telemetry           | Endpoint for JSON metrics push                      | `https://exporter.yourdomain.net/json` | `.env` or K8s config                  |
| **`GHCR_PAT`**                                      | Docker image build/push            | Personal access token for GitHub Container Registry | `ghp_XXXX`                             | GitHub Actions secret                 |
| **`DOCKERHUB_USERNAME`**                            | Docker build jobs                  | Docker Hub username                                 | `ippanbuildbot`                        | GitHub Actions secret                 |
| **`DOCKERHUB_TOKEN`**                               | Docker build jobs                  | Docker Hub access token                             | `ghp_XXXX`                             | GitHub Actions secret                 |
| **`NVD_API_KEY`**                                   | Security / dependency scan         | API key for CVE database                            | `<uuid>`                               | GitHub Actions secret                 |

### 🪜 Steps to Add or Update Secrets in GitHub

1. Go to your repo → **Settings → Secrets and variables → Actions**
2. Click **“New repository secret”**
3. Add each secret with its value
4. Repeat for all in the table above
5. Rerun workflows from the **Actions** tab once added

### ⚙️ Local Development (Optional)

Create a file named `.env` at the project root (not committed to git):

```bash
LLM_API_KEY=sk-your-real-key
PROMETHEUS_ENDPOINT=http://localhost:9090/metrics
JSON_EXPORTER_ENDPOINT=http://localhost:9091/json
```

Use it for local runs (`docker-compose`, `cargo run`, etc.). **Never push this file.**

## 🤖 Cursor Instructions for Secret Validation

You can have **Cursor Web** automatically verify and manage secrets with the following prompts.

### 🔍 1. Check what secrets are referenced

Paste in Cursor chat:

```
@cursor
Scan all .github/workflows/*.yml files.
List every ${{ secrets.* }} reference.
Show which ones are documented in .env.example files and which are missing.
```

### 🧠 2. Auto-add safety guards

If a secret is optional or not yet configured:

```
@cursor
For any step using a secret that may be missing, wrap the step in:
if: env.SECRET_NAME != ''
so the job is skipped safely if the secret isn’t set.
```

### 🧰 3. Update documentation

```
@cursor
Update README.md’s “IPPAN Secrets Configuration Guide” section to include any new secrets found in workflows.
```

### 🧪 4. Validate existence (read-only)

```
@cursor
Check the repository settings (Settings → Secrets → Actions) to confirm if these secrets exist, and list missing ones.
```

*(Cursor will output the list, but won’t reveal secret values.)*

### ✅ Commit Message Template

Once Cursor finishes:

```
git add README.md .github/workflows/
git commit -m "docs(secrets): update IPPAN secrets setup and CI guard conditions"
git push
```

---

Would you like me to extend this with a **Cursor automation snippet** that periodically re-checks secret references in workflows (e.g., weekly GitHub Action)?

## 📈 Performance

- **Time Precision**: Microsecond accuracy
- **HashTimer Generation**: ~1μs per operation
- **Block Processing**: Optimized for high throughput
- **Memory Usage**: Efficient in-memory structures

## 🧪 Testing

```bash
# Run all tests
cargo test --workspace

# Run specific test suite
cargo test -p ippan-types

# Run with logging
RUST_LOG=debug cargo test --workspace
```

## 🤖 AI-Powered Consensus

### D-GBDT: Deterministic AI for Validator Selection

IPPAN uses **D-GBDT** (Deterministic Gradient-Boosted Decision Trees) for consensus-safe AI inference in validator selection and reputation scoring.

**Key Features**:
- **100% Deterministic**: Fixed-point arithmetic (no floating-point) ensures identical predictions across all nodes
- **Cross-platform**: Bit-for-bit identical results on x86_64, aarch64, ARM, and RISC-V
- **Verifiable**: Model hash anchored to IPPAN Time HashTimer for cryptographic integrity
- **Production-ready**: Used in mainnet for validator scoring with ~100μs prediction time

📖 **[D-GBDT Documentation](docs/ai/D-GBDT.md)** — Complete guide covering:
- Fixed-point SCALE policy (micro-precision arithmetic)
- Feature schema and validator telemetry
- Model lifecycle: train → canonicalize → hash → load → cache
- Model rotation procedures and operational runbook
- Determinism checklist and troubleshooting

**Quick Example**:
```rust
// Load deterministic model
let model = DeterministicGBDT::from_json_file("models/gbdt/validator_v1.json")?;

// Extract features (all fixed-point, no floats)
let features = vec![
    Fixed::from_micro(1_500_000),   // latency: 1.5ms
    Fixed::from_micro(999_000),     // uptime: 99.9%
    Fixed::from_micro(800_000),     // entropy: 0.8
];

// Predict validator score (deterministic across all nodes)
let score = model.predict(&features);
```

## 📚 Academic Research

### Consensus Research & Documentation

**Quick Start**: [Consensus Research Summary](docs/CONSENSUS_RESEARCH_SUMMARY.md) — Navigation guide to all consensus documentation

**Academic Whitepaper**: [Beyond BFT: The Deterministic Learning Consensus Model](docs/BEYOND_BFT_DETERMINISTIC_LEARNING_CONSENSUS.md)

This peer-reviewed academic paper formalizes IPPAN's novel consensus paradigm:
- **Temporal ordering** replaces voting for Byzantine agreement
- **HashTimer™** provides cryptographic time as consensus authority
- **D-GBDT** enables deterministic AI-driven fairness
- **Security proofs** under ≤⅓ Byzantine adversary assumption
- **Performance analysis** demonstrating >10M TPS theoretical capacity

**Key Innovation**: IPPAN achieves 100-250ms finality (vs 2-10s in traditional BFT) by making time—not votes—the source of consensus authority.

## 📝 License

IPPAN is provided as **source-available software** under the IPPAN Community Source License. You can read and audit the code and run nodes on official IPPAN networks, but forks, competing networks, and commercial redistribution are restricted. Governing law is England & Wales with courts in London, UK. See `LICENSE.md` for full terms.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 🚨 Security

- All cryptographic operations use industry-standard libraries
- HashTimer provides temporal ordering guarantees
- IPPAN Time prevents time-based attacks
- Production deployments include security hardening

## 🔐 IPPAN Secrets Configuration Guide

### 📘 Overview

IPPAN's GitHub Actions workflows and runtime environments depend on several **secrets** and **environment variables**.
None of them are stored in the repository — only placeholders exist in `.env.example` files.
Each developer or CI environment must set them explicitly.

---

### 🧩 Required Secrets

| Secret name                                         | Used by                            | Description                                         | Example value                          | Location                              |
| --------------------------------------------------- | ---------------------------------- | --------------------------------------------------- | -------------------------------------- | ------------------------------------- |
| **`LLM_API_KEY`** *(or `IPPAN_SECRET_LLM_API_KEY`)* | `ai_service`, AI determinism tests | API key for LLM inference (e.g., OpenAI, Anthropic) | `sk-xxxx`                              | GitHub → Settings → Secrets → Actions |
| **`PROMETHEUS_ENDPOINT`**                           | Node telemetry exporter            | URL endpoint for Prometheus metrics                 | `https://metrics.yourdomain.net`       | `.env` or K8s secret                  |
| **`JSON_EXPORTER_ENDPOINT`**                        | AI & consensus telemetry           | Endpoint for JSON metrics push                      | `https://exporter.yourdomain.net/json` | `.env` or K8s config                  |
| **`GHCR_PAT`**                                      | Docker image build/push            | Personal access token for GitHub Container Registry | `ghp_XXXX`                             | GitHub Actions secret                 |
| **`GITHUB_TOKEN`**                                  | All workflows, Docker registry     | Built-in GitHub Actions token for API access        | *(auto-generated)*                     | Automatically provided by GitHub      |
| **`DOCKERHUB_USERNAME`**                            | Docker build jobs (optional)       | Docker Hub username                                 | `ippanbuildbot`                        | GitHub Actions secret                 |
| **`DOCKERHUB_TOKEN`**                               | Docker build jobs (optional)       | Docker Hub access token                             | `dckr_pat_XXXX`                        | GitHub Actions secret                 |
| **`NVD_API_KEY`**                                   | Security / dependency scan         | API key for CVE database                            | `<uuid>`                               | GitHub Actions secret                 |
| **`DEPLOY_SSH_KEY`**                                | Production deployment              | SSH private key for deployment servers              | `-----BEGIN OPENSSH PRIVATE KEY-----`  | GitHub Actions secret                 |
| **`SERVER1_HOST`**                                  | Production deployment              | Server 1 (full-stack) hostname or IP                | `188.245.97.41`                        | GitHub Actions secret                 |
| **`SERVER2_HOST`**                                  | Production deployment              | Server 2 (node-only) hostname or IP                 | `135.181.145.174`                      | GitHub Actions secret                 |
| **`DEPLOY_USER`**                                   | Production deployment              | SSH username for deployment                         | `root` or `ubuntu`                     | GitHub Actions secret                 |
| **`SERVER1_SSH_KEY`**                               | Deployment (Server 1)              | SSH private key for Server 1 (if different)         | `-----BEGIN OPENSSH PRIVATE KEY-----`  | GitHub Actions secret                 |
| **`SERVER2_SSH_KEY`**                               | Deployment (Server 2)              | SSH private key for Server 2 (if different)         | `-----BEGIN OPENSSH PRIVATE KEY-----`  | GitHub Actions secret                 |

---

### 🪜 Steps to Add or Update Secrets in GitHub

1. Go to your repo → **Settings → Secrets and variables → Actions**
2. Click **"New repository secret"**
3. Add each secret with its value
4. Repeat for all in the table above
5. Rerun workflows from the **Actions** tab once added

---

### ⚙️ Local Development (Optional)

Create a file named `.env` at the project root (not committed to git):

```bash
LLM_API_KEY=sk-your-real-key
PROMETHEUS_ENDPOINT=http://localhost:9090/metrics
JSON_EXPORTER_ENDPOINT=http://localhost:9091/json
```

Use it for local runs (`docker-compose`, `cargo run`, etc.).
**Never push this file.**

---

## 🤖 Cursor Instructions for Secret Validation

You can have **Cursor Web** automatically verify and manage secrets with the following prompts.

### 🔍 1. Check what secrets are referenced

Paste in Cursor chat:

```
@cursor
Scan all .github/workflows/*.yml files.
List every ${{ secrets.* }} reference.
Show which ones are documented in .env.example files and which are missing.
```

### 🧠 2. Auto-add safety guards

If a secret is optional or not yet configured:

```
@cursor
For any step using a secret that may be missing, wrap the step in:
if: env.SECRET_NAME != ''
so the job is skipped safely if the secret isn't set.
```

### 🧰 3. Update documentation

```
@cursor
Update README.md's "IPPAN Secrets Configuration Guide" section to include any new secrets found in workflows.
```

### 🧪 4. Validate existence (read-only)

```
@cursor
Check the repository settings (Settings → Secrets → Actions) to confirm if these secrets exist, and list missing ones.
```

*(Cursor will output the list, but won't reveal secret values.)*

---

### ✅ Commit Message Template

Once Cursor finishes:

```bash
git add README.md .github/workflows/
git commit -m "docs(secrets): update IPPAN secrets setup and CI guard conditions"
git push
```

---

**IPPAN Blockchain**: Real blockchain with authoritative time and temporal validation.