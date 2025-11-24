# IPPAN API Versioning Policy

## Scope
The IPPAN public interfaces include:
- Core RPC endpoints used by nodes, wallets, and explorers (payments, handles, files, consensus health).
- Explorer / operator surfaces documented in `docs/API_EXPLORER_SURFACE.md` and `docs/operators/` runbooks.
- AI status and model metadata (`/ai/status`, `/version`, `/metrics`).

## Version scheme
- The current protocol and RPC surface is published as **v1**.
- Additive, backward-compatible changes may be introduced within `v1` (new fields marked as optional in client logic).
- Breaking changes require either a new versioned path or a deprecation window with dual responses.
- Testnet/devnet may ship breaking changes with prior notice; mainnet deployments receive a published deprecation schedule before removals.

## Compatibility and deprecations
- Stable fields in `v1` remain consistent until a new major revision is announced.
- When deprecating or removing behavior, operators receive release notes plus a minimum grace period where the old and new responses coexist.
- Clients should tolerate additional optional fields but must not rely on undocumented behavior or ordering.

## Discoverability
- Use the `/version` endpoint to detect the active protocol version and network:
  - `protocol_version`: logical API contract (e.g., `v1`).
  - `version`: crate/package version baked into the binary.
  - `commit`: git commit hash when provided at build time.
  - `network`: deployment flavor (`devnet`, `testnet`, `mainnet`, or an override via `IPPAN_NETWORK`).
- Stable `/version` example:

```json
{
  "protocol_version": "v1",
  "version": "1.0.0",
  "commit": "<git-hash-or-unknown>",
  "mode": "PoA",
  "network": "devnet"
}
```
- Integrators should log `/version` at startup and during health checks to detect mismatched deployments early.

## Expectations for integrators
- Target `v1` today; prepare clients to follow redirects or version bumps announced in release notes.
- Treat `/version` as the canonical contract signal. If `protocol_version` changes, align client expectations before enabling writes.
- Report incompatibilities or ambiguous fields via the security or support channels so the next revision can document them.
