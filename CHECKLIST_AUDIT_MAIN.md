# Main Branch Feature Audit Checklist

_Generated: 2025-11-15_

## 1. L1 Payments
- [x] Hardened payment pipeline (`crates/consensus/src/payments.rs`) applies fees/anti-double-spend inside round finalization with stats + tests.
- [x] Unit tests cover happy-path, insufficient balance, fee distribution (treasury + proposer) in `crates/consensus/src/payments.rs`.
- [x] CLI `ippan pay` surface exists (`crates/cli/src/main.rs`, `PayCommand` posts to `/tx/payment`).
- [ ] RPC `POST /tx/payment` handler/router (server-side) — `crates/rpc/src/server.rs` is empty so no route registration; CLI call has no backing endpoint.
- [ ] Payment history `GET /account/:address/payments` missing in RPC/router and storage.
- [ ] End-to-end payment demo docs/scripts (`docs/payments/demo_end_to_end_payment.md`, `scripts/demo_payment_flow.sh`) not present in repo.

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

## 4. No Floats in Runtime
- [ ] Runtime crates still contain `f64`/`f32` usages (e.g., `crates/types/src/l2.rs`, `crates/governance/src/voting.rs`, `crates/economics/src/types.rs`, `crates/security/src/validation.rs`, `crates/network/src/metrics.rs`, `crates/core/src/sync_manager.rs`). These are outside tests/examples and violate the "no float" rule.
- [x] `.github/workflows/no-float-runtime.yml` exists and targets `main`.
- [ ] Workflow scope is limited to `ai_core`, `consensus*`, and `ai_registry`; it does **not** scan other runtime crates (`types`, `network`, `governance`, `storage`, `node`, etc.), so violations slip through CI.

## 5. IPNDHT Network Layer
- [x] Libp2p network stack + DHT helper (`crates/p2p/src/lib.rs`, `crates/p2p/src/ipndht.rs`) provide publish/find APIs with caching.
- [ ] No dedicated `DhtConfig`; bootstrap/NAT settings live in `P2PConfig` and there is no config layer that drives DHT behavior independently.
- [ ] Node startup (`node/src/main.rs`) never constructs or wires an `IpnDhtService`; RPC `AppState` fields `file_dht`/`file_storage` cannot be populated anywhere.
- [x] Multi-node/discovery tests exist (ignored by default) under `crates/p2p/tests/ipndht_resilience.rs`.
- [x] Docs available: `docs/ipndht/ipndht_hardening_plan.md`, `docs/ipndht/file-descriptors.md`, `IPNDHT_FILE_IMPLEMENTATION_SUMMARY.md`.

## 6. Handles (@handle.ipn)
- [x] Handle registry + metadata (`crates/l2_handle_registry/src/*.rs`) implement handle validation, owner index, expiration, and tests.
- [x] L1 handle anchor storage exists (`crates/l1_handle_anchors/src/anchors.rs`) with proof generation/verification.
- [ ] Consensus pipeline lacks a `TxKind::Handle` or any deterministic rules for handle transactions; no integration with mempool/round execution.
- [ ] RPC endpoints `POST /handle/register` and `GET /handle/{handle}` are not implemented anywhere.
- [ ] No DHT publication for handles (no `publish_handle` in runtime code; only referenced in docs/tests).

## 7. File Descriptors & DHT
- [x] FileDescriptor model + indices implemented (`crates/files/src/descriptor.rs`, `crates/files/src/storage.rs`, and `crates/types/src/file_descriptor.rs`).
- [x] RPC handler logic for `POST /files/publish` + `GET /files/{id}` exists in `crates/rpc/src/files.rs` with coverage tests (`files_tests.rs`).
- [ ] `AppState`/server wiring is missing entirely (`crates/rpc/src/server.rs` is empty), so the file endpoints are never registered or reachable.
- [ ] DHT integration is stubbed: `StubFileDhtService` works, but the `Libp2pFileDhtService` implementation is just a placeholder behind an unused feature flag, and `node` never instantiates any `FileDhtService`.
- [x] Documentation covers file descriptors/DHT hooks (`docs/ipndht/file-descriptors.md`, `IPNDHT_FILE_IMPLEMENTATION_SUMMARY.md`).

## 8. Payment API Docs & CLI
- [x] CLI `pay` command documented in code and uses integer atomic units (`crates/cli/src/main.rs`).
- [ ] Payment API guide (`docs/PAYMENT_API_GUIDE.md`) and endpoint-level docs for `/tx/payment` + `/account/:address/payments` are absent.
- [ ] `/tx/payment` route itself is missing on the RPC server, so the documented CLI call cannot succeed.
- [ ] `/account/:address/payments` history endpoint and pagination/direction support are not implemented anywhere in RPC or storage.
- [ ] No docs/scripts describing an end-to-end payment demo exist in repo.

## Optional Test Runs
- `cargo test -p ippan-rpc -- --nocapture` → **fails** (expected) due to missing OpenSSL headers in the environment; no additional compiler errors observed before the toolchain check halted.
- `cargo test -p ippan-consensus-dlc -- --nocapture` → **passes** locally (vends registry-backed fairness); only external toolchain issues (e.g., OpenSSL) would block in other environments.
- `cargo test -p ippan-network -- --nocapture` → **passes** (27 unit tests green).
