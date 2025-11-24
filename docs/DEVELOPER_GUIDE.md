# IPPAN Developer Guide

This guide equips contributors with the workflow, tooling, and context needed to build within the IPPAN repository while honoring deterministic and scope-aware development practices.

## Prerequisites

- Linux, macOS, or WSL2 with build-essential tooling (C compiler, make, pkg-config).
- [Rustup](https://rustup.rs/) with the stable toolchain plus `rustfmt` and `clippy` components (configured in `rust-toolchain.toml`).
- Node.js 20 LTS (or newer) with npm for the gateway service and unified UI.
- Docker (optional) for container builds and deployment validation.

Verify your toolchain:

```bash
rustc --version
cargo --version
node --version
npm --version
```

## Environment Setup

1. Clone the repository and enter the workspace:
   ```bash
   git clone <repo-url>
   cd ippan
   ```
2. Ensure the Rust toolchain matches the project baseline:
   ```bash
   rustup show
   rustup component add rustfmt clippy
   ```
3. Install JavaScript dependencies when working on Node/Next.js apps:
   ```bash
   cd apps/unified-ui && npm install
   cd ../gateway && npm install
   ```
4. Optional utilities:
   - `cargo install cargo-nextest` for faster Rust test runs.
   - `npm install --global @mermaid-js/mermaid-cli` for regenerating diagrams under `docs/diagrams`.

## Repository Layout

| Area | Path | What Lives Here |
| --- | --- | --- |
| Core protocol crates | `crates/` | Consensus, AI, networking, wallet, and supporting Rust libraries. |
| Applications | `apps/` | `unified-ui` (Next.js frontend), `gateway` (Express proxy), and mobile clients. |
| Documentation | `docs/` | Specifications, PRDs, operational guides, and diagrams (see [`docs/README.md`](./README.md)). |
| Deploy & infra | `deploy/`, `deployments/`, `docker/` | Scripts, manifests, and container definitions for CI/CD and runtime environments. |
| Benchmarks & tests | `benchmarks/`, `tests/` | Performance harnesses and cross-crate integration suites. |

## Daily Workflow

1. **Format Rust code**
   ```bash
   cargo fmt --all
   ```
2. **Static analysis**
   ```bash
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```
3. **Build & test**
   ```bash
   cargo check --workspace
   cargo test --workspace --all-features
   ```
4. **Frontend linting & type safety**
   ```bash
   cd apps/unified-ui
   npm run lint
   npm run type-check
   ```
5. **Gateway smoke test**
   ```bash
   cd apps/gateway
   npm run dev
   ```
6. **Document your changes** by touching the closest module README (for example `docs/consensus/README.md`) and referencing new files in commit messages.

## Running Services Locally

- **Unified UI**
  ```bash
  cd apps/unified-ui
  npm run dev
  ```
  The app binds to `http://localhost:3000` by default and expects the gateway/WebSocket endpoints defined in `.env.local`.

- **Gateway API**
  ```bash
  cd apps/gateway
  npm run dev
  ```
  Use `.env` settings or command-line flags to point upstream requests to local validators or mock services.

- **Rust binaries**
  Each crate documents its own entrypoints. Typical patterns use `cargo run -p <crate-name>`; consult the crate-level README before launching to confirm feature flags and configuration.

## Determinism Checklist

- Favor `BTreeMap`, `BTreeSet`, and sorted vectors for deterministic iteration.
- Normalize numeric operations to fixed-point or integer math—avoid unchecked floating-point logic.
- Seed randomness from deterministic inputs (e.g., block hashes) when randomness is unavoidable.
- Avoid system time and non-deterministic IO in consensus-critical code; use the HashTimer abstractions instead.
- Ensure tests are hermetic and do not rely on the execution order of asynchronous tasks.

## Documentation & Support

- Start with the [`docs/README.md`](./README.md) index to find module-specific references.
- Protocol deep dives live in [`AI_IMPLEMENTATION_GUIDE.md`](./AI_IMPLEMENTATION_GUIDE.md), [`CONSENSUS_RESEARCH_SUMMARY.md`](./CONSENSUS_RESEARCH_SUMMARY.md), and the `consensus/` module.
- Product direction, acceptance criteria, and roadmap are under [`prd/`](./prd/README.md).
- Wallet CLI workflows and smoke tests are documented in [`dev/wallet-cli.md`](./dev/wallet-cli.md).
- For agent governance and scope rules, revisit [`.cursor/AGENT_CHARTER.md`](../.cursor/AGENT_CHARTER.md).

When in doubt, prefer discussion in code reviews or issue threads so knowledge stays visible to other contributors.
