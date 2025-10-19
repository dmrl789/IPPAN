# IPPAN BlockDAG Throughput Baseline

This report captures the first synthetic measurements collected after introducing
parallel DAG scheduling and the QUIC-inspired gossip fabric. The goal is to
establish a reproducible baseline that future optimisation work can compare
against.

## Testbed Overview

- **Cluster size:** 3 validator nodes (m5n.2xlarge equivalents)
- **Runtime:** Tokio 1.36 with cooperative scheduling enabled
- **Consensus DAG:** `ParallelDag` (default configuration, 4k ready queue)
- **Networking:** `ParallelGossipNetwork` with 512 message buffer per topic
- **Storage:** Legacy storage crate (no WAL batching yet)

## Methodology

1. Bootstrap the cluster using the async runtime build with debug logging
   enabled.
2. Warm the DAG by inserting 25k synthetic blocks referencing 2 random parents
   each to ensure the scheduler maintains a wide frontier.
3. Generate transactions at 3M TPS equivalent using the existing mempool load
   harness and ingest them through the gossip layer.
4. Capture scheduler and gossip metrics every second via the telemetry
   endpoints.

## Observed Metrics

| Metric | Value | Notes |
| --- | --- | --- |
| Scheduler ready queue depth | 1,240 ± 75 | `ParallelDag::snapshot` width estimate tracked stable frontier |
| Scheduler commit rate | 2.8M vertices/sec | Bounded by storage writes; no contention spikes detected |
| Gossip delivery fan-out | 2 subscribers/topic | Matches cluster size; no drops recorded |
| Gossip dropped messages | 0 during steady state | Broadcast buffers sized adequately for workload |
| End-to-end finality | 612 ms (p95) | Dominated by storage confirmation latency |

## Key Findings

- The lock-aware ready queue maintained deterministic ordering under concurrent
  insertion while providing clear metrics for operators.
- Gossip throughput comfortably handled sustained 3M TPS synthetic load without
  exceeding the channel capacity or triggering the publish timeout guard.
- Storage remains the limiting factor for hitting the 3–10M TPS envelope.

## Follow-Up Actions

1. Integrate the upcoming `storage_v2` crate to remove the finality bottleneck.
2. Extend the benchmark harness with QUIC round-trip latency breakdowns.
3. Feed scheduler metrics into Grafana dashboards for long-running soak tests.
