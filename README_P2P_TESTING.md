# IPPAN P2P Testing Harness

Repeatable P2P test harness for validating bootstrap, peer discovery, gossip propagation, and resilience under churn/chaos.

## Prerequisites

* Rust and Cargo
* Bash (WSL2 supported)
* `tc` (optional, for advanced netem chaos)

## Quickstart

Run the following scripts from the repository root:

```bash
# Smoke test (4 nodes, 30s timeout)
bash scripts/p2p/smoke.sh

# Gossip propagation test (8 nodes, 200 messages)
bash scripts/p2p/gossip.sh

# Resilience under churn (10 nodes, random kill/restart)
bash scripts/p2p/churn.sh

# Adverse network conditions (6 nodes, loss/latency/jitter)
bash scripts/p2p/chaos_netem.sh
```

## Configuration

The harness uses environment variables for configuration:

* `IPPAN_NODE_CMD`: The command to start an IPPAN node (default: `cargo run -p ippan-node --`).
* `IPPAN_NODE_ARGS`: Extra arguments to pass to every node.

Example:
```bash
export IPPAN_NODE_CMD="cargo run --release -p ippan-node --"
export IPPAN_NODE_ARGS="--log-json"
bash scripts/p2p/smoke.sh
```

## Scenarios

### Smoke
Validates that nodes can bootstrap and form a basic mesh. Every node must connect to at least one peer within 20s.

### Gossip
Measures delivery rate and latency. node0 publishes messages on the `ippan/test/gossip` topic.
- **Metrics**: Unique delivery rate (deduplicated), latency (p50/p95/max), duplicate counts.
- **Success**: ≥ 95% unique delivery rate.

### Churn
Simulates nodes joining and leaving the network. Randomly kills and restarts peers while gossip is running.

### Chaos
Applies packet loss, latency, and jitter using IPPAN's built-in chaos configuration.
- **Verification**: Runs a gossip burst under chaos to validate propagation.
- **Success**: ≥ 70% unique delivery rate (configurable).

## Artifacts

Each run produces artifacts in `./artifacts/p2p/<timestamp>/`:

* `node<i>.log`: Combined stdout/stderr for each node.
* `summary.json`: Machine-readable test results and metrics.

## Troubleshooting

* **Nodes fail to connect:** Ensure `cargo build -p ippan-node` succeeds. Check `node<i>.log` for errors.
* **Port conflicts:** The harness uses ports 19000+ for P2P and 18080+ for RPC. Ensure these are free.
* **Clean data:** Use `bash scripts/p2p/clean.sh` to remove old logs and data directories.
