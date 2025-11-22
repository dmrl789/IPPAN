# Changelog

All notable changes to this project will be documented in this file.
The format roughly follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Placeholder for upcoming changes.

## v0.9.0-rc1 (Audit Candidate) — 2025-11-20

- Deterministic AI/D-GBDT (DLC) consensus path hardened with canonical JSON + BLAKE3 hashing and cross-arch determinism tests.
- No-float runtime enforcement expanded across consensus, RPC, storage, and AI-core crates to keep execution deterministic.
- HashTimer + time ordering pipelines finalized with genesis replay validation and snapshot import/export coverage.
- RPC/API security guardrails consolidated on `SecurityManager` (rate limits, IP allow/deny lists) across payments, handles, files, AI status, and health endpoints.
- IPNDHT/libp2p resilience improvements for descriptor validation and peer churn handling, backed by multi-node tests.
- Nightly full validation workflow retained for coverage scoring and readiness tracking ahead of external audit.

## [0.1.0] - 2025-11-06

- Establish release governance automation scaffolding.
- Configure shared workspace metadata for coordinated versioning.
- Add release checklist covering changelog, tag safety, builds, and license audit.

