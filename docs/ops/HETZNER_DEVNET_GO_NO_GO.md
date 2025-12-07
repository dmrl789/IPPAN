# Hetzner Devnet Go/No-Go (IPPAN)

## What this is

A small, controlled devnet on Hetzner to validate ops (deployment, upgrades, monitoring, backups, stability).

This is NOT mainnet.

## Go criteria (deploy devnet now)

- master is green:
  - Build & Test (Rust): ✅
  - AI Determinism & DLC Consensus: ✅
  - Audit Pack: ✅
  - Readiness Pulse (Weekly): ✅ (at least 1 fresh green run)

- D-GBDT model is pinned:
  - config/dlc.toml contains [dgbdt.model] path + expected_hash
  - `verify_model_hash` passes

## No-Go criteria (wait)

- Readiness Pulse is red, or weekly soak/fuzz/audit gates are not reproducible

- No monitoring/alerting plan

- No backup/restore drill

## Minimum devnet topology (suggested)

- 3 validator nodes

- 1 RPC/observer node (rate-limited public RPC if exposed)

- Optional: 1 seed/bootnode (or reuse observer)

## Ops must-haves (day 1)

- Systemd services (preferred) or docker compose (server-side Linux only)

- Persistent storage volumes

- Metrics + logs collection (Prometheus/Grafana + Loki or equivalent)

- Alerting (node down, high latency, disk pressure, crash loops)

- Backup + restore test for node data

## Rollout plan

1) Bring up nodes privately (firewall allowlist)

2) Verify peer discovery + stable multi-node operation (24h)

3) Run upgrade rehearsal (rolling restart, config update)

4) Publish devnet RPC only after stability and rate limits

## Mainnet is a separate decision

Mainnet go/no-go requires multi-day soak stability, incident runbooks, and security review.

