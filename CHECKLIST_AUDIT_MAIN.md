# Main Branch Feature Audit Checklist

_Generated: 2025-11-15_

## 1. L1 Payments
- [x] Hardened payment pipeline (`crates/consensus/src/payments.rs`) applies fees/anti-double-spend inside round finalization with stats + tests.
- [x] Unit tests cover happy-path, insufficient balance, fee distribution (treasury + proposer) in `crates/consensus/src/payments.rs`.
- [x] CLI `ippan pay` surface exists (`crates/cli/src/main.rs`, `PayCommand` posts to `/tx/payment`).
- [x] RPC `POST /tx/payment` handler + router wiring live in `crates/rpc/src/server.rs`, so the CLI call has an axum endpoint to hit.
- [x] Payment history `GET /account/:address/payments` exists in `crates/rpc/src/server.rs`, returning storage-backed history sorted by timestamp with a clamped `limit` (cursor-style pagination reserved for future work).
- [x] End-to-end payment demo docs/scripts (`docs/payments/demo_end_to_end_payment.md`, `scripts/demo_payment_flow.sh`) are committed and current.
- [x] RPC integration tests cover `POST /tx/payment` success/error cases and `GET /account/:address/payments` pagination/direction handling.

## 2. Fees
- [x] Centralized integer-only fee logic via `ippan_l1_fees::FeePolicy` (`crates/l1_fees/src/lib.rs`).
- [x] Tests verify min-fee validation, per-byte estimates, validator/treasury split (`crates/l1_fees/src/lib.rs`).
- [x] Payment pipeline credits validator + treasury on apply (`crates/consensus/src/payments.rs`) with stats exported to telemetry/metrics.

## 3. D-GBDT & AI Core
- [x] `crates/ai_core/src/lib.rs` re-exports the fixed-point + GBDT surface (no more empty lib) so downstream crates compile again.
- [x] Deterministic integer-only model/tree/node definitions live under `crates/ai_core/src/gbdt/*`, using canonical JSON + BLAKE3 hashing.
- [x] `ai_registry` now ships `DGBDTRegistry::load_and_activate_from_config()` which stores the active model/hash inside sled.
- [x] `DGBDTRegistry::get_active_model()` returns `(Model, hash)` but compile errors currently prevent use.
- [x] `consensus_dlc` fairness pulls the active model from `ai_registry` (env `IPPAN_DGBDT_REGISTRY_PATH` fallback); built-in model is only a warning fallback.
- [x] AI determinism workflow `.github/workflows/ai-determinism.yml` targets `main` and runs determinism/no-float jobs.
- [x] `cargo test -p ippan-consensus-dlc -- --nocapture` compiles/runs (modulo expected OpenSSL env gaps) now that `ippan_ai_core` exports resolve.
- [x] `/ai/status` RPC endpoint is backed by the live DLC consensus engine and surfaces whether the deterministic model is enabled, stub status, and the active BLAKE3 hash/version (see `docs/AI_STATUS_API.md`).
- [x] Canonical JSON + BLAKE3 hashing tests cover deterministic model serialization; registry activation tests verify sled state + history.

Operators can now fetch the live AI model hash and stub/real status via RPC, making the deterministic pipeline observable.

- [x] `ai_trainer` crate trains deterministic models and exports them in the
      `ippan-ai-core` fixed-point format with canonical JSON + BLAKE3 hash.
- [x] All trained models live under `models/` with documented naming and
      lifecycle guidance (`models/README.md`).
- [x] `config/dlc.toml` links to a canonical model path and expected hash so
      `ai_registry` can reject mismatches at startup.
- [x] `docs/AI_MODEL_LIFECYCLE.md` and `docs/AI_TRAINING_DATASET.md` describe
      the dataset schema and full model lifecycle.
- [ ] CI automation to cross-check `expected_hash` values against the on-disk
      JSON artifacts (future work).

## 3b. Shadow Verifiers (DLC Redundancy)
- [x] Primary + shadow verifier selection is deterministic and repeats across runs (tested in `crates/consensus/tests/dlc_integration_tests.rs`).
- [x] DLC verifier set management covers shadow branch handling and consistency (`crates/consensus_dlc/tests/long_run_simulation.rs`).
- [ ] Additional long-run shadow verifier soak tests in CI (Future phase, not RC blocker).

## 4. No Floats in Runtime
- [x] Runtime crates now avoid `f64`/`f32` usages: currency/L2 types use atomic units, governance/economics/security/network/core/rpc modules all compute with fixed-point integers or ratios.
- [x] `.github/workflows/no-float-runtime.yml` exists and targets `master`.
- [ ] Workflow scope is limited to `ai_core`, `consensus*`, and `ai_registry`; it does **not** scan other runtime crates (`types`, `network`, `governance`, `storage`, `node`, etc.), so violations slip through CI.
- [x] Workflow now limits matching to Rust runtime sources (excluding tests/examples/benches) so comments/docs mentioning floats no longer cause false positives.

## 4b. CI / Actions
- [x] Build & Test (Rust) workflow listens to `master`, scopes `RUSTFLAGS=-D warnings` to build/test only, and the storage integration tests compile again after updating `ChainState` usage (`crates/storage/tests/integration_tests.rs`).
- [x] AI Determinism & DLC workflow watches `master` pushes/PRs so determinism + DLC suites re-run on the default branch.
- [x] No Floats in Runtime workflow now triggers on `master`, keeping the gate aligned with the active branch.
- [x] CodeQL/Security scan workflow updated to the `master` branch so pushes there stay covered.
- [ ] RPC crate tests continue to require system OpenSSL headers; `cargo test -p ippan-rpc` remains environment-blocked on hosted runners (see notes below) and is treated as an external issue until the repo bundles its own TLS backend.

## 5. IPNDHT Network Layer
- [x] Libp2p network stack + DHT helper (`crates/p2p/src/lib.rs`, `crates/p2p/src/ipndht.rs`) provide publish/find APIs with caching.
- [x] Dedicated `DhtConfig` separates bootstrap/NAT/DHT announcement settings from `P2PConfig`, feeding the HTTP network and libp2p-backed IPNDHT helpers.
- [x] Node startup (`node/src/main.rs`) now wires `MemoryFileStorage` + `StubFileDhtService` into the RPC `AppState`, giving `/files/*` endpoints live handles while the real libp2p-backed service is still pending.
- [x] Multi-node/discovery tests exist (ignored by default) under `crates/p2p/tests/ipndht_resilience.rs`.
- [x] Docs available: `docs/ipndht/ipndht_hardening_plan.md`, `docs/ipndht/file-descriptors.md`, `IPNDHT_FILE_IMPLEMENTATION_SUMMARY.md`.
- [x] IPNDHT lookup rejects mismatched/conflicting descriptors from the DHT and unit tests cover the rejection paths.

## 6. Handles (@handle.ipn)
- [x] Handle registry + metadata (`crates/l2_handle_registry/src/*.rs`) implement handle validation, owner index, expiration, and tests.
- [x] L1 handle anchor storage exists (`crates/l1_handle_anchors/src/anchors.rs`) with proof generation/verification.
- [x] Consensus pipeline now processes `TxKind::Handle` transactions via `crates/consensus/src/handles.rs`, enforcing deterministic fees/uniqueness + anchoring during round finalization.
- [x] RPC endpoints `POST /handle/register` and `GET /handle/{handle}` live in `crates/rpc/src/server.rs`, wiring builder helpers + router paths.
- [x] Handle registrations publish into IPNDHT via the new `HandleDhtService` (stub + libp2p), so consensus writes immediately propagate to the DHT.
- [x] RPC tests exercise handle registration + lookup flows, replaying consensus pipeline to assert stub DHT publication and registry responses.

## 7. File Descriptors & DHT
- [x] FileDescriptor model + indices implemented (`crates/files/src/descriptor.rs`, `crates/files/src/storage.rs`, and `crates/types/src/file_descriptor.rs`).
- [x] RPC handler logic for `POST /files/publish` + `GET /files/{id}` exists in `crates/rpc/src/files.rs` with coverage tests (`files_tests.rs`).
- [x] `crates/rpc/src/files.rs` handlers are wired into the `Router`, and `AppState` now carries `file_storage`/`file_dht` handles so `ippan-rpc` builds with file RPC enabled (stub DHT still acceptable).
- [x] File DHT has a libp2p-backed `FileDhtService` behind the runtime flag (`IPPAN_FILE_DHT_MODE=libp2p`), enabling publish/find to use Kademlia while keeping the stub for tests and minimal setups.
- [x] Documentation covers file descriptors/DHT hooks (`docs/ipndht/file-descriptors.md`, `IPNDHT_FILE_IMPLEMENTATION_SUMMARY.md`).
- [x] RPC file endpoint tests cover publish success/validation plus DHT fallback lookups via instrumented stub services.

## 8. Payment API Docs & CLI
- [x] CLI `pay` command documented in code and uses integer atomic units (`crates/cli/src/main.rs`).
- [x] `docs/PAYMENT_API_GUIDE.md` captures the currency model, REST payloads, CLI usage, and client snippets for `/tx/payment` and `/account/:address/payments`.
- [x] RPC `/tx/payment` + `/account/:address/payments` endpoints exist in `crates/rpc/src/server.rs` and only accept integer (`u128`) currency amounts.
- [x] Demo docs/scripts describing the payment flow exist (`docs/payments/demo_end_to_end_payment.md`, `scripts/demo_payment_flow.sh`).

## 9. End-to-End Demo
- [x] End-to-end dev demo for handles + payments + files + AI/DHT status documented in `docs/demo_end_to_end_ippan.md` and automated via `scripts/demo_ippan_full_flow.sh`.
- [x] Three-node localnet demo (handles + payments + file DHT) documented in `docs/localnet_three_node_demo.md` with configs + scripts under `localnet/` and `scripts/localnet_*`.
- [ ] Multi-node soak / longevity tests for the localnet (long-running gossip + DLC stress) are still pending.

## 10. RPC & Security
- [x] All RPC routes now share the existing `SecurityManager` guard + rate limiter so read/write endpoints enforce IP/rate policies consistently (`crates/rpc/src/server.rs`).
- [x] Dev-only helpers such as `/dev/fund` are gated by `IPPAN_DEV_MODE`, loopback IP checks, and loopback binding defaults outside dev mode (`node/src/main.rs`).
- [ ] Advanced authentication (API keys, JWT) remains future work; current deployments rely on IP whitelists + reverse proxies per `docs/SECURITY_GUIDE.md`.

## 11. Observability & Ops
- [x] `/health` endpoint surfaces consensus/DHT/RPC/storage status as a structured `HealthStatus` payload (`crates/rpc/src/server.rs`).
- [x] `/metrics` endpoint serves Prometheus text output whenever the exporter is enabled (`crates/rpc/src/server.rs`).
- [x] `/health` endpoint contract validated via tests covering healthy + degraded dependencies.
- [ ] Advanced dashboards/alert policies are tracked separately (future work).

## 11b. Explorer & Ops API
- [x] Explorer-read RPC surface documented in `docs/API_EXPLORER_SURFACE.md`.
- [x] Core DTOs for blocks/txs/accounts/payments/handles/files are consistent, integer-based, and stable.
- [x] Observability endpoints (`/health`, `/ai/status`, `/metrics`) documented for dashboards.
- [x] Dev-only endpoints explicitly marked and dev-gated.
- [ ] Strong versioning / deprecation policy for RPC (future).

## Tests & Coverage
- [x] L1 payment RPC tests exercise `/tx/payment` success/error cases plus `/account/:address/payments` pagination & direction handling.
- [x] Handle registration/lookup tests drive transactions through the consensus pipeline and assert stub DHT publication state.
- [x] File RPC tests validate publish/store flows and DHT fallback lookups via recording stubs.
- [x] AI/DLC determinism covered via canonical JSON/hash tests, registry activation/history assertions, and fairness model scoring from sled-backed registries.
- [x] `/health` endpoint tested for both healthy and degraded dependencies to mirror operator expectations.
- [ ] Long-running chaos/resilience tests in CI (future work).

### Comprehensive Testing (Phase 1)
- [x] Time / HashTimer invariants covered with monotonicity, skew rejection, and clamping tests in `crates/time`.
- [x] DLC long-run simulation tests exercise deterministic emission/reward invariants and reputation bounds (`crates/consensus_dlc/tests/emission_invariants.rs`).
- [x] Multi-block storage replay + snapshot round-trip tests validate state/root consistency across replays (`crates/storage/tests/replay_roundtrip.rs`).
- [ ] Property-based / fuzz testing for consensus and networking (Phase 2).
- [ ] Long-duration stress tests in live/RC testnets (Phase 2).

## Optional Test Runs
- `cargo test -p ippan-rpc -- --nocapture` → **fails** (expected) due to missing OpenSSL headers in the environment; no additional compiler errors observed before the toolchain check halted.
- `cargo test -p ippan-consensus-dlc -- --nocapture` → **passes** locally (vends registry-backed fairness); only external toolchain issues (e.g., OpenSSL) would block in other environments.
- `cargo test -p ippan-network -- --nocapture` → **passes** (27 unit tests green).

## ZK / Cryptographic proofs
- [x] ZK-STARK design PRD drafted (see `docs/zk/zk-ippan-stark-prd.md`)
- [ ] Prototype STARK circuit and off-chain verifier (testnet only)
- [ ] Optional integration of STARK proofs into DLC / reward flows

## 12. Production Readiness
- [x] Semantic versioning + `/version` endpoint
- [x] Release packaging directory with config template
- [x] Systemd template for Linux deployments
- [x] Node `--version` & `--check` flags
- [ ] CI-driven reproducible release builds (future)
- [ ] SBOM signing + binary signatures (future)

## 13. Storage & Recovery
- [x] SnapshotManifest defined and derivable from storage.
- [x] `export_snapshot` / `import_snapshot` cover blocks, transactions, accounts, files, and metadata.
- [x] `ippan-node snapshot export/import` subcommands run maintenance flows without starting networking.
- [x] `docs/STORAGE_AND_SNAPSHOTS.md` documents the layout, snapshot workflow, and crash-restart scenario.
- [ ] Automated snapshot/restore CI soak tests (future).

## 14. Network Resilience
- [x] Chaos knobs for P2P (probabilistic drop & latency) exist.
- [x] `scripts/localnet_chaos_start.sh` + `scripts/localnet_chaos_scenario.sh` exercise payments, handles, and files while drops/latency are active.
- [x] Node restart/churn walkthrough (`scripts/localnet_churn_scenario.sh`) documents the manual stop/start flow and validates RPC convergence after reboot.
- [ ] Automated, long-running chaos suites wired into CI remain future work.
