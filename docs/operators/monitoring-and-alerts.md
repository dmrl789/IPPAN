# Monitoring & Alerting Quickstart

IPPAN nodes export Prometheus metrics on `/metrics` when enabled in `config/dlc.toml`. Use the provided Grafana exports under `grafana_dashboards/` as a starting point.

## Importing dashboards
1. In Grafana, go to **Dashboards → Import**.
2. Upload `grafana_dashboards/dashboard_consensus.json` for consensus health and AI visibility.
3. Upload `grafana_dashboards/dashboard_node_health.json` for node-level health and resource monitoring.
4. Point both at the Prometheus data source that scrapes your nodes.

Key metrics visualized:
- Consensus and connectivity: `ippan_rounds_finalized_total`, `ippan_consensus_forks_total`, `ippan_finalized_height`, `ippan_peer_count`.
- Block production: `ippan_blocks_proposed_total`, `ippan_blocks_validated_total`.
- RPC health: `ippan_rpc_requests_total`, `ippan_rpc_errors_total`, `ippan_health_status`.
- Node resources: `process_cpu_seconds_total`, `process_resident_memory_bytes`, `ippan_mempool_transactions` (if exported by your Prometheus agent).

## Example Prometheus alerts
Paste into your Prometheus `rules.yml` (adjust labels/instances to match your deployment):

```yaml
- alert: IppanNodeMissing
  expr: up{job="ippan"} == 0
  for: 3m
  labels:
    severity: page
  annotations:
    summary: "IPPAN node missing from scrape"
    description: "Prometheus has failed to scrape the node for 3 minutes. Check process, network, or firewall."

- alert: ConsensusForksHigh
  expr: rate(ippan_consensus_forks_total[5m]) > 0.1
  for: 5m
  labels:
    severity: warn
  annotations:
    summary: "Consensus fork rate elevated"
    description: "Forks detected in the last 5 minutes. Investigate peer health and validator versions."

- alert: RpcErrorRateHigh
  expr: rate(ippan_ai_selection_fallback[5m]) / rate(ippan_ai_selection_total[5m]) > 0.05
  for: 5m
  labels:
    severity: warn
  annotations:
    summary: "RPC/AI fallback rate above 5%"
    description: "Clients are hitting fallback paths. Verify model hash, registry status, and /health."
```

## Operator tips
- Track `/health` and `/version` in your existing uptime checks; alert if `rpc_healthy` or `consensus_healthy` flip to `false`.
- Include peer count and mempool depth (`ippan_peer_count`, `ippan_mempool_transactions` if exported) to spot isolation or backlog.
- Keep historical dashboards for validator selection balance and AI latency so regressions are visible before release tags.
