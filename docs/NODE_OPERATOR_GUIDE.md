# IPPAN Node Operator Guide

This guide summarizes how to configure, start, monitor, and troubleshoot an IPPAN node.

## Prerequisites
- **Rust toolchain:** stable Rust (via `rustup`) and `cargo` for building from source.
- **System resources:** 2+ CPU cores, 4 GB RAM, and at least 10 GB free disk for the default sled database.
- **Open ports:**
  - RPC/HTTP: `8080` (configurable)
  - P2P: `9000` (configurable)
  - Optional libp2p DHT: `9100` (configurable when using libp2p file/handle DHT modes)

## Configuration
A commented reference config is available at `config/ippan_node.toml`. Copy it to a writable path (e.g., `~/.config/ippan/node.toml`) and adjust the values for your environment.

Common settings:
- `network.id`: choose `localnet` for local development or `testnet` for shared testing environments.
- `rpc.host` / `rpc.port`: the HTTP API bind address. Use `127.0.0.1` with a reverse proxy for hardened deployments.
- `p2p.port`: peer-to-peer listener port.
- `PROMETHEUS_ENABLED`: toggle the `/metrics` endpoint.
- `PID_FILE`: controls where the node writes its PID for stop/status commands.

Environment variables prefixed with `IPPAN_` override config values (e.g., `IPPAN_RPC_PORT=18100`).

## CLI usage
The `ippan-node` binary now exposes explicit lifecycle and inspection commands:

```bash
# Start the node using the default config
cargo run --bin ippan-node -- --config config/ippan_node.toml start

# Pick a network and log level from the CLI
cargo run --bin ippan-node -- --config config/ippan_node.toml \
  --network testnet --log-level info start

# Check node health without starting it
cargo run --bin ippan-node -- --config config/ippan_node.toml status

# Stop a running node using its PID file
cargo run --bin ippan-node -- --config config/ippan_node.toml stop
```

Flags such as `--log-level`, `--log-format`, `--network`, `--rpc-port`, `--p2p-port`, `--pid-file`, and `--disable-metrics` override configuration file values for one-off runs. Use `--check` to run config/port validations without launching the node.

## Health and metrics
- **Health:** `GET /health` returns a structured JSON payload describing consensus health, storage availability, peer counts, mempool size, uptime, and DHT status. This is the fastest way to confirm a node is live after startup.
- **Metrics:** `GET /metrics` returns Prometheus-formatted metrics when `PROMETHEUS_ENABLED` is true. Disable it with the `--disable-metrics` CLI flag or by setting `PROMETHEUS_ENABLED = false` in your config.

## Observability tips
- **Logs:** control verbosity with `--log-level info|debug|trace` and select `--log-format json` for log aggregation systems. Startup logs enumerate the network ID, RPC/P2P bindings, and DHT mode selection.
- **Key metrics:**
  - `node_build_info{version,commit}`, `node_uptime_seconds`, `node_health`
  - `p2p_connected_peers`, `p2p_peers_connected_total`, `p2p_peers_dropped_total`
  - `consensus_current_round`, `consensus_finalized_round`, `consensus_blocks_proposed_total`, `consensus_forks_total`
  - `rpc_requests_total{path,method}`, `rpc_requests_failed_total{path,method}`, `rpc_request_duration_microseconds{path,method}`
  - `/health` payload fields such as `peer_count` and `uptime_seconds` expose live connectivity and runtime duration.
- **Dashboards:** scrape `/metrics` with Prometheus and visualize consensus round/finalized gauges, P2P peer counts, RPC latency histograms, and mempool size alongside `/health` peer counts in Grafana to alert on stalled gossip or consensus.

## Troubleshooting
- **Port conflicts:** if `/health` fails immediately after startup, re-run `ippan-node --check --rpc-port <alt> --p2p-port <alt>` to find open ports.
- **Stale PID file:** if `stop` reports a missing or invalid PID, remove the PID file path specified in `PID_FILE` and restart.
- **Missing data directory:** the node will create the `data_dir` and database parent directory automatically, but ensure the process user can write to both paths.
- **Metrics disabled:** if `/metrics` returns 404, confirm `PROMETHEUS_ENABLED` is true or omit `--disable-metrics` when starting the node.

## Operational checkpoints
- Monitor `/health` for `storage_healthy` and `consensus_healthy` flags before routing traffic.
- Keep RPC bound to localhost or behind a firewall unless `DEV_MODE` is intentionally enabled.
- Persist your configuration in version control or infrastructure tooling so rolling restarts reproduce the same settings.
