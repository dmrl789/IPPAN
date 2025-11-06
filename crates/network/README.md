# IPPAN Network

## Overview
- Defines deterministic networking primitives for discovery, gossip, and reputation.
- Provides the building blocks used by higher-level P2P services and RPC servers.
- Focuses on predictable peer management suitable for DAG consensus traffic.

## Key Modules
- `connection` and `discovery`: manage peer dialing, bootstrap, and service discovery.
- `parallel_gossip` and `deduplication`: propagate DAG updates while avoiding duplicates.
- `peers`, `reputation`, and `health`: maintain peer state, scores, and health monitoring.
- `protocol` and `metrics`: define wire formats and export runtime metrics.

## Integration Notes
- Embed `ParallelGossip` within node services to broadcast blocks and transactions.
- Surface peer snapshots through `PeerDirectory` when building operator tooling.
- Combine `HealthMonitor` outputs with security policies to quarantine unreliable peers.
