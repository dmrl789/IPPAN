# Observability Guide

IPPAN nodes expose lightweight health and metrics endpoints so operators can
track consensus progress, DHT status, storage liveness, and AI/DLC mode without
attaching a debugger or enabling verbose logs.

## `/health`

* **Method:** `GET`
* **Response type:** JSON (`ippan_types::health::HealthStatus`)
* **Fields:**
  * `status` – quick `"ok"` vs `"error"` indicator derived from the checks
    below.
  * `consensus_mode` – textual label such as `"poa"` or `"dlc"` plus
    `consensus_healthy` and `last_consensus_round` for liveness.
  * `ai_enabled` – true when the DLC/AI fairness loop is turned on.
  * `dht_file_mode` / `dht_handle_mode` – `"stub"` vs `"libp2p"` for each DHT
    alongside `dht_healthy`.
  * `rpc_healthy`, `storage_healthy`, `last_finalized_round` – storage probes and
    round-finalization metadata.
  * `peer_count`, `mempool_size`, `uptime_seconds`, `requests_served` – integer
    counters suited for dashboards/alerts.
  * `node_id`, `version`, `dev_mode` – runtime identity markers.

Example response:

```json
{
  "consensus_mode": "poa",
  "consensus_healthy": true,
  "ai_enabled": false,
  "dht_file_mode": "stub",
  "dht_handle_mode": "stub",
  "dht_healthy": true,
  "rpc_healthy": true,
  "storage_healthy": true,
  "last_finalized_round": 128,
  "last_consensus_round": 130,
  "peer_count": 6,
  "mempool_size": 4,
  "uptime_seconds": 3600,
  "requests_served": 42,
  "node_id": "node-1",
  "version": "0.1.0",
  "dev_mode": false
}
```

## `/metrics`

* **Method:** `GET`
* **Response type:** Prometheus text format (`text/plain; version=0.0.4`).
* **Enabled via:** `IPPAN_PROMETHEUS_ENABLED=1` or equivalent config toggle.
* **Usage:**
  * Point Prometheus to `http://NODE:8080/metrics`.
  * Example curl: `curl -H 'Accept: text/plain' http://localhost:8080/metrics`.
  * Metrics include counters/gauges registered via the `metrics` crate
    (consensus round, mempool size, node health gauge, etc.).
  * Sample payload:

```
# HELP ippan_mempool_size current mempool size
# TYPE ippan_mempool_size gauge
ippan_mempool_size 4
# HELP ippan_consensus_round last consensus round seen
# TYPE ippan_consensus_round gauge
ippan_consensus_round 130
```

If metrics are disabled the endpoint returns `503` with the message
"Prometheus metrics disabled" so scrapers can alert on misconfiguration.

## `/ai/status`

* **Method:** `GET`
* **Response type:** JSON summary of the deterministic AI/DLC subsystem.
* **Fields:**
  * `enabled` – whether AI fairness is active.
  * `using_stub` – true when a placeholder model is loaded.
  * `model_hash` / `model_version` – optional identifiers for the active
    artifact.
  * `consensus_mode` – mirrors the node's current consensus label so dashboards
    can correlate AI state with consensus settings.

Example response:

```json
{
  "enabled": true,
  "using_stub": false,
  "model_hash": "b8f91b…",
  "model_version": "dlc-2025-11-01",
  "consensus_mode": "dlc"
}
```

Use `/ai/status` alongside `/health` to confirm whether the DLC fairness model
is live on the node and whether a stub or production model is serving.

## Integration tips

* **Dashboards:** plot `peer_count`, `last_consensus_round`, `mempool_size`, and
  `ai_enabled` from `/health` plus `/ai/status` while overlaying Prometheus
  scrapes to track backlog and fairness status.
* **Alerting:** trigger alerts when `consensus_healthy`, `storage_healthy`, or
  `dht_healthy` switch to `false`, or when `/metrics` is unavailable.
* **Gateways:** the gateway `/api/health` routes directly to the node `/health`
  endpoint, so external monitors can use either surface depending on firewall
  rules.
