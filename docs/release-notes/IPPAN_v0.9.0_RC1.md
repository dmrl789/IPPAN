# IPPAN v0.9.0-rc1 Release Candidate Notes

## What’s in this RC
- Deterministic AI/D-GBDT (DLC) consensus integrated and gated by CI.
- Cross-architecture determinism validation on x86_64 and aarch64.
- Float-free runtime enforcement across core crates to protect consensus safety.
- IPPAN Time with HashTimer ordering plus genesis replay coverage.
- Expanded RPC surface: payments, handles, files, AI status, operator health.
- Rate-limiting and IP whitelisting security hardening with accompanying tests.
- Nightly Full Validation workflow publishing coverage (≈65%) and readiness scores.

## What is NOT in this RC
- Finalized mainnet economics and emission parameters (subject to change).
- ZK-STARK integrations and advanced privacy features.
- Production-grade audit trail and SBOM signing pipeline.
- Long-horizon stability tuning for large validator sets.

## Intended use
- **Testnet/devnet experimentation only.** Not for production or mainnet funds.
- Expect breaking changes before v1.0 as economics, privacy, and observability evolve.

## Operator quick links
- Operator guide: [running an IPPAN RC node](../operators/running-ippan-rc-node.md).
- Repository status: see the README status block for CI and coverage signals.
- Feedback: open issues in this repository with the `release` label for RC feedback.

## Upgrade and compatibility
- Nodes compiled with v0.9.0-rc1 should be rebuilt when the final v0.9.0 is tagged.
- Config compatibility is not guaranteed across RC iterations; review defaults each upgrade.
- Embedding the git revision into builds via `GIT_COMMIT_HASH=$(git rev-parse --short HEAD)` is
  recommended so operators can verify which RC commit is running in logs.

## Reporting issues
- File issues with reproduction steps, architecture (x86_64/aarch64), and log excerpts.
- Security concerns should use a private channel to maintainers; do not post secrets.
