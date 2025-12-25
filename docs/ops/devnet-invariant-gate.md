# Devnet invariant gate (required before any txload/bot run)

This repo intentionally enforces a **cluster invariant** before performance testing.

## What this gate checks

Across `api1..api4`, the script verifies:

- `/health` responds quickly
- `/status.peer_count >= 3`
- `/status.validator_count == 4`
- `validator_ids_sample` matches across all nodes (identity drift detection)
- `round` advances over two samples (liveness)

## How to run

From your workstation (WSL is fine) at repo root:

```bash
chmod +x scripts/ops/devnet_invariant_check.sh
./scripts/ops/devnet_invariant_check.sh
```

Overrides:

```bash
RPC_PORT=8080 SLEEP_SECS=5 NODES="api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk" ./scripts/ops/devnet_invariant_check.sh
```

## Why “consensus fail-fast” is correct

The node process is designed to **exit(1)** if:

- the consensus task fails to start, exits, or panics, or
- the consensus submit channel closes

This is deliberate: it prevents a state where RPC stays “healthy” while consensus is dead.
Systemd restart loops are preferred to silent wedging.


