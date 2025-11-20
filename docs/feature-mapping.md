# IPPAN Feature → Code → Tests Map

A quick reference for reviewers to see where each major IPPAN feature lives in code, how it is tested, and which CI workflows exercise it. Paths are relative to the repository root.

## IPPAN Time & HashTimer
- **Code:**
  - `crates/time/src/hashtimer.rs` implements the HashTimer primitive, signing/verification helpers, and deterministic constructors for tx/block/round timers.
  - `crates/time/src/ippan_time.rs` and `crates/time/src/sync.rs` expose the deterministic IPPAN time source and synchronization helpers.
  - `crates/types/src/time_service.rs` wires the time client into shared types.
- **Tests:**
  - Deterministic derivation, ordering, and encoding/decoding tests in `crates/time/src/hashtimer.rs` (round/tx/block constructors, signature verification).
  - HashTimer serialization/ordering coverage in `crates/types/src/tests.rs`.
  - Consensus pipeline exercises HashTimer usage in `crates/consensus/tests/dlc_integration_tests.rs`.
- **CI:**
  - Covered by the workspace test suites in `.github/workflows/ci.yml` and `.github/workflows/nightly-validation.yml`.

## Deterministic AI / D-GBDT & DLC Consensus
- **Code:**
  - `crates/ai_core/src/gbdt/*` and `crates/ai_core/src/lib.rs` expose fixed-point model parsing, hashing, and inference.
  - `crates/ai_registry/src/lib.rs` manages deterministic model manifests, activation, and persistence.
  - `crates/consensus_dlc/src` hosts DLC scoring, verifier selection, and replay-safe consensus state.
  - `crates/consensus/src/dgbdt.rs` and `crates/consensus/src/dlc.rs` integrate deterministic scoring into the round loop.
- **Tests:**
  - Determinism and fixed-point inference suites under `crates/ai_core/tests/`.
  - Registry manifest and activation coverage in `crates/ai_registry/tests/manifest_tests.rs`.
  - DLC verifier selection/scoring simulations in `crates/consensus_dlc/tests/long_run_simulation.rs`.
  - DLC integration and scoring checks in `crates/consensus/tests/dlc_integration_tests.rs`.
- **CI:**
  - `.github/workflows/ai-determinism.yml` (model hash/determinism checks) and `.github/workflows/ci.yml` (unit/integration suites) gate DLC + AI paths; nightly reruns via `.github/workflows/nightly-validation.yml`.

## Shadow Verifiers & Deterministic Selection
- **Code:**
  - Shadow verifier selection and parallel verification in `crates/consensus/src/dlc.rs` and `crates/consensus/src/shadow_verifier.rs`.
  - Deterministic weighted selection logic in `crates/consensus/src/dgbdt.rs`.
  - DLC verifier set management in `crates/consensus_dlc/src/verifier.rs`.
- **Tests:**
  - Primary/shadow selection stability and parallel validation in `crates/consensus/tests/dlc_integration_tests.rs`.
  - Verifier set consistency and branch handling in `crates/consensus_dlc/tests/long_run_simulation.rs`.
- **CI:**
  - Exercised by the consensus suites in `.github/workflows/ci.yml` and nightly validation.

## Emission & Fairness Rules (DAG-Fair)
- **Code:**
  - Emission math, parameters, and distribution in `crates/ippan_economics/src/` and `crates/economics/src/` (supply caps, halving, proposer/verifier splits).
  - Treasury sink wiring in `crates/treasury/src` and economics helpers consumed by consensus.
- **Tests:**
  - DAG-Fair integration, halving, cap enforcement, and proposer/verifier ratio checks in `tests/emission_integration.rs`.
  - Economics crate integration coverage in `crates/ippan_economics/tests/integration_tests.rs`.
- **CI:**
  - Part of the workspace tests in `.github/workflows/ci.yml` and nightly validation.

## No-Float Runtime Enforcement
- **Code:**
  - Runtime crates use fixed-point/int types; float usage is isolated to docs/examples only.
- **Tests/Checks:**
  - Static scan in `.github/workflows/no-float-runtime.yml` blocks float regressions across runtime crates.
  - Deterministic AI tests in `crates/ai_core/tests/` ensure inference uses integer math.
- **CI:**
  - `no-float-runtime` workflow plus standard CI/nightly suites.

## RPC Surface (Payments, Handles, Files, AI/Health)
- **Code:**
  - Payments, health, AI status, and security guards in `crates/rpc/src/server.rs`.
  - File publish/lookup handlers in `crates/rpc/src/files.rs` with routing from `server.rs`.
  - Handle registration/lookup wired through `crates/rpc/src/server.rs` into L1/L2 registries.
- **Tests:**
  - Payment, account history, handle registration, health, and security guard tests in `crates/rpc/src/server.rs`.
  - File publish/lookup and DHT fallback tests in `crates/rpc/src/files_tests.rs`.
- **CI:**
  - RPC tests run via `.github/workflows/ci.yml` (OpenSSL dependency noted) and nightly validation; security/static coverage via `codeql.yml`.

## Storage, Snapshots, and Replay
- **Code:**
  - Storage backends and chain state in `crates/storage/src` (sled + in-memory) with snapshot/export helpers.
  - Chain state/round/transaction types in `crates/types/src`.
- **Tests:**
  - Storage integration tests for blocks/tx/accounts/L2 data/snapshots in `crates/storage/tests/integration_tests.rs`.
  - Node-level snapshot/export/import behaviors covered by `crates/storage/tests` and workspace demos.
- **CI:**
  - Exercised in `.github/workflows/ci.yml` and nightly validation suites.

## Security (Rate Limiting, IP Policies, Audit)
- **Code:**
  - Rate limiter, circuit breaker, validation, and audit hooks in `crates/security/src` with `SecurityManager` consumed by RPC.
  - RPC guard wiring in `crates/rpc/src/server.rs` (shared across routes) and file endpoints in `crates/rpc/src/files.rs`.
- **Tests:**
  - Security error mapping, guard enforcement, and IP policy checks in `crates/rpc/src/server.rs` tests.
- **CI:**
  - Security-sensitive paths covered by `.github/workflows/ci.yml`; CodeQL scan in `.github/workflows/codeql.yml`.

## P2P / DHT / File & Handle Discovery
- **Code:**
  - IPNDHT helpers and libp2p integration in `crates/p2p/src/lib.rs` and `crates/p2p/src/ipndht.rs`.
  - File DHT/backends in `crates/files/src/dht.rs`; handle DHT in `crates/l2_handle_registry/src/dht.rs`.
  - Node wiring for stub/libp2p DHT services in `node/src/main.rs` and RPC app state.
- **Tests:**
  - IPNDHT resilience and multi-node discovery tests (ignored by default) in `crates/p2p/tests/ipndht_resilience.rs`.
  - File publish/lookup DHT behavior in `crates/rpc/src/files_tests.rs`.
- **CI:**
  - DHT-related unit tests run in `.github/workflows/ci.yml` and nightly validation when not ignored.

## Monitoring & Health
- **Code:**
  - `/health`, `/metrics`, and `/ai/status` endpoints in `crates/rpc/src/server.rs`, including dependency wiring.
  - Metrics/exporter toggles integrated through RPC state and consensus telemetry.
- **Tests:**
  - Health endpoint coverage (healthy/degraded dependencies) and metrics wiring in `crates/rpc/src/server.rs` tests.
- **CI:**
  - Exercised by RPC suites in `.github/workflows/ci.yml` and nightly validation.
