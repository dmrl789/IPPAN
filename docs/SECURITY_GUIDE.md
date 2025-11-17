# RPC Security Guide

This guide summarizes how the IPPAN RPC server protects its surface area and how operators should expose it in production.

## Guard stack

* Every router endpoint in `crates/rpc/src/server.rs` calls `guard_request`, which runs the configured `SecurityManager` checks (IP allow/block lists, abuse heuristics, circuit breakers, and global rate limits). Violations are rejected and recorded through `record_security_failure`, while happy paths call `record_security_success`.
* Additional Axum middleware layers (rate limiter + circuit breaker) still wrap the router, so the handler-level guard is an explicit extra line of defense.
* File descriptor routes in `crates/rpc/src/files.rs` use the same guard via `guard_file_request`.

## Endpoint matrix

| Endpoint(s) | Purpose | Classification | Notes |
|-------------|---------|----------------|-------|
| `/health`, `/status`, `/time`, `/version`, `/metrics`, `/ai/status` | Node telemetry | Public-safe read | Guarded + logged; still recommended to front with HTTPS/reverse proxy.
| `/tx/:hash`, `/block/:id`, `/account/:address`, `/account/:address/payments`, `/files/:id`, `/l2/config`, `/l2/networks`, `/l2/commits`, `/l2/exits` | Ledger/state queries | Public-safe read | Guarded and rate limited; no mutations.
| `/tx`, `/tx/payment`, `/handle/register`, `/files/publish` | Operator mutations | Authenticated/operator-only | Require guard + consensus/mempool access. Protect via firewall or authenticated API gateway.
| `/p2p/blocks`, `/p2p/block-response`, `/p2p/transactions`, `/p2p/peer-info`, `/p2p/peer-discovery`, `/p2p/block-request` | Node-to-node gossip | Internal-only | Guarded, but intended for internal mesh/VPN traffic only.
| `/dev/fund` | Local faucet/reset | Dev-only | Requires `IPPAN_DEV_MODE=true` **and** loopback IP (127.0.0.1/::1). Route is only registered when dev mode is enabled.

## Dev mode rules

* Set `IPPAN_DEV_MODE=true` or pass `--dev` to `ippan-node` to enable development conveniences (e.g., `/dev/fund`).
* `/dev/fund` double-checks that the incoming socket IP is loopback before mutating storage, even if dev mode is on.
* Outside dev mode the router omits the `/dev/*` namespace entirely, so production nodes cannot accidentally expose faucets.

## Binding guidance

* When dev mode is **off**, the node now defaults `RPC_HOST` to `127.0.0.1`. Operators should terminate TLS and auth in a reverse proxy (nginx, envoy, etc.) or restrict the port via firewall/VPN.
* When dev mode is enabled (via env or `--dev`) the default host flips to `0.0.0.0` so local demos can reach the API from other devices. The CLI flag also upgrades log verbosity and only widens the bind if the host was still set to the loopback default.
* If a production config explicitly binds to `0.0.0.0` the node logs a warning reminding the operator to keep the RPC behind a proxy or firewall.

## Abuse tracking & rate limits

* `SecurityManager` records every success/failure to the audit log (if configured), so repeated violations can be blocked at the IP level.
* `RateLimiterLayer` (200 req/sec by default) and the circuit breaker protect the process from bursts. Because every handler now invokes `guard_request`, even read-only endpoints participate in the centralized accounting.

## Operational checklist

1. Keep `IPPAN_DEV_MODE=false` (default) in production; use reverse proxies or VPNs to expose only the read-only endpoints you intend to share publicly.
2. For public explorers or dashboards, only expose the public-safe endpoints listed above. Everything else (mutating and P2P routes) should remain on private networks.
3. Review `CHECKLIST_AUDIT_MAIN.md` section 10 before releases to ensure no new endpoints skip the guard stack.
4. Plan for future auth (API keys/JWT) if you need to expose operator-only endpoints beyond trusted infrastructure.
