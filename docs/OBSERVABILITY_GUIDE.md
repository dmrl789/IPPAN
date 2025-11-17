# Observability Guide

IPPAN nodes expose lightweight health and metrics endpoints so operators can
track consensus progress, DHT status, storage liveness, and AI/DLC mode without
attaching a debugger or enabling verbose logs.

## `/health`

* **Method:** `GET`
* **Response type:** JSON (`ippan_types::health::HealthStatus`)
* **Fields:**
  * `consensus_mode` – textual label such as `"PoA"` or `"DLC"`.
  * `consensus_healthy` / `last_consensus_round` – whether the consensus loop
    responded to the snapshot request and its most recent round height.
  * `ai_enabled` – true when the DLC AI pipeline is active.
  * `dht_file_mode` / `dht_handle_mode` – `"stub"` vs `"libp2p"` for each DHT.
  * `dht_healthy` – true when at least one DHT service is wired.
  * `rpc_healthy` – RPC service status (always true once serving requests).
  * `storage_healthy` / `last_finalized_round` – storage read probes and the
    most recent finalized round number.
  * `peer_count`, `mempool_size`, `uptime_seconds`, `requests_served` – core
    counters captured as integers for dashboards and alerting.
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

If metrics are disabled the endpoint returns `503` with the message
"Prometheus metrics disabled" so scrapers can alert on misconfiguration.

## `/ai/status`

* **Method:** `GET`
* **Response type:** JSON summary of the deterministic AI/DLC subsystem.
* **Fields:** `enabled`, `using_stub`, optional `model_hash` and `model_version`.

Use `/ai/status` alongside `/health` to confirm whether the DLC fairness model
is live on the node and whether a stub or production model is serving.

## Integration tips

* **Dashboards:** plot `peer_count`, `last_consensus_round`, and `mempool_size`
  from `/health` together with Prometheus scrapes to track liveness and backlog.
* **Alerting:** trigger alerts when `consensus_healthy`, `storage_healthy`, or
  `dht_healthy` switch to `false`, or when `/metrics` is unavailable.
* **Gateways:** the gateway `/api/health` routes directly to the node `/health`
  endpoint, so external monitors can use either surface depending on firewall
  rules.
