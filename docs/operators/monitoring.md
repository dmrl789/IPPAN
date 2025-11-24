# IPPAN Node Monitoring Guide
**Observability with Prometheus + Grafana**

**Target Audience:** Node operators, DevOps engineers, SREs  
**Version:** v1.0.0-rc1

---

## Overview

IPPAN nodes expose Prometheus metrics and health endpoints for comprehensive observability. This guide covers:
- Enabling metrics collection
- Setting up Prometheus scraping
- Importing Grafana dashboards
- Key metrics to monitor
- Alerting best practices

---

## Quick Start

### 1. Enable Metrics in Node Configuration

Edit your node configuration file (e.g., `config/ippan_node.toml`):

```toml
[observability]
# Enable Prometheus metrics endpoint
metrics_enabled = true

# Metrics endpoint address and port
metrics_bind = "0.0.0.0:9615"

# Optional: Enable detailed tracing (increases overhead)
detailed_tracing = false
```

### 2. Start Node with Metrics

```bash
ippan-node --config config/ippan_node.toml
```

### 3. Verify Metrics Endpoint

```bash
curl http://localhost:9615/metrics
```

**Expected Output:** Prometheus text format metrics:
```
# HELP ippan_consensus_rounds_total Total number of consensus rounds processed
# TYPE ippan_consensus_rounds_total counter
ippan_consensus_rounds_total 12345

# HELP ippan_network_peers_total Current number of connected peers
# TYPE ippan_network_peers_total gauge
ippan_network_peers_total 42
...
```

---

## Prometheus Setup

### Install Prometheus

**Linux (systemd):**
```bash
wget https://github.com/prometheus/prometheus/releases/download/v2.45.0/prometheus-2.45.0.linux-amd64.tar.gz
tar xvfz prometheus-2.45.0.linux-amd64.tar.gz
cd prometheus-2.45.0.linux-amd64
```

**Docker:**
```bash
docker run -d --name prometheus -p 9090:9090 \
  -v $(pwd)/prometheus.yml:/etc/prometheus/prometheus.yml \
  prom/prometheus
```

### Configure Prometheus Scraping

Create `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'ippan-node'
    static_configs:
      - targets: ['localhost:9615']
        labels:
          instance: 'node1'
          network: 'mainnet'

  # Multi-node setup
  - job_name: 'ippan-cluster'
    static_configs:
      - targets:
          - 'node1.ippan.io:9615'
          - 'node2.ippan.io:9615'
          - 'node3.ippan.io:9615'
        labels:
          network: 'mainnet'
```

### Start Prometheus

```bash
./prometheus --config.file=prometheus.yml
```

Access Prometheus UI: `http://localhost:9090`

---

## Grafana Setup

### Install Grafana

**Linux:**
```bash
sudo apt-get install -y software-properties-common
sudo add-apt-repository "deb https://packages.grafana.com/oss/deb stable main"
sudo apt-get update
sudo apt-get install grafana
sudo systemctl start grafana-server
```

**Docker:**
```bash
docker run -d --name=grafana -p 3000:3000 grafana/grafana
```

Access Grafana UI: `http://localhost:3000`  
**Default credentials:** admin / admin

### Add Prometheus Data Source

1. Navigate to **Configuration → Data Sources**
2. Click **Add data source**
3. Select **Prometheus**
4. Set URL: `http://localhost:9090`
5. Click **Save & Test**

### Import IPPAN Dashboards

1. Navigate to **Dashboards → Import**
2. Upload JSON files from `/workspace/grafana_dashboards/`:
   - `ippan-consensus.json` - Consensus health metrics
   - `ippan-dlc-fairness.json` - DLC and D-GBDT fairness
   - `ippan-network.json` - Network and P2P health
   - `ippan-hashtimer.json` - HashTimer and time sync

3. Select Prometheus data source
4. Click **Import**

---

## Key Metrics

### Consensus Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ippan_consensus_rounds_total` | Counter | Total rounds processed |
| `ippan_consensus_finalized_blocks_total` | Counter | Total finalized blocks |
| `ippan_consensus_finality_latency_seconds` | Histogram | Time to finality |
| `ippan_consensus_forks_total` | Counter | Fork events detected |
| `ippan_dag_tips_count` | Gauge | Current DAG tips |
| `ippan_dag_pending_blocks` | Gauge | Pending (unfinalized) blocks |

### DLC & D-GBDT Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ippan_dgbdt_score_bucket` | Histogram | Validator score distribution |
| `ippan_dlc_primary_selections_total` | Counter | Primary role assignments |
| `ippan_dlc_shadow_events_total` | Counter | Shadow verifier activations |
| `ippan_dlc_shadow_disagreements_total` | Counter | Shadow-primary conflicts |

### Network Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ippan_network_peers_total` | Gauge | Connected peers |
| `ippan_network_messages_total` | Counter | Messages sent/received |
| `ippan_dht_operations_total` | Counter | DHT publish/find operations |
| `ippan_network_failures_total` | Counter | Network error count |

### HashTimer Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ippan_hashtimer_clock_skew_ms` | Gauge | Clock skew in milliseconds |
| `ippan_hashtimer_outliers_total` | Counter | Outlier timestamps rejected |
| `ippan_hashtimer_corrections_total` | Counter | Time correction events |

### Health Endpoint Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ippan_health_status` | Gauge | Overall health (1=healthy, 0=degraded) |
| `ippan_health_consensus_ok` | Gauge | Consensus subsystem health |
| `ippan_health_network_ok` | Gauge | Network subsystem health |
| `ippan_health_storage_ok` | Gauge | Storage subsystem health |

---

## Alerting Rules

### Prometheus Alerting

Create `alerts.yml`:

```yaml
groups:
  - name: ippan_critical
    interval: 30s
    rules:
      - alert: NodeDown
        expr: up{job="ippan-node"} == 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "IPPAN node {{ $labels.instance }} is down"

      - alert: HighFinalityLatency
        expr: ippan_consensus_finality_latency_seconds > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High finality latency on {{ $labels.instance }}"

      - alert: LowPeerCount
        expr: ippan_network_peers_total < 5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low peer count on {{ $labels.instance }}"

      - alert: HighClockSkew
        expr: abs(ippan_hashtimer_clock_skew_ms) > 500
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High clock skew detected on {{ $labels.instance }}"
```

Add to `prometheus.yml`:
```yaml
rule_files:
  - "alerts.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['localhost:9093']
```

---

## Security Considerations

### 1. Bind Address

**Development:**
```toml
metrics_bind = "0.0.0.0:9615"  # Listen on all interfaces
```

**Production:**
```toml
metrics_bind = "127.0.0.1:9615"  # Loopback only
```

Use a reverse proxy (Nginx/Caddy) with authentication for external access.

### 2. Firewall Rules

```bash
# Allow Prometheus scraping from internal network only
sudo ufw allow from 10.0.0.0/8 to any port 9615
sudo ufw deny 9615
```

### 3. Nginx Reverse Proxy with Basic Auth

```nginx
server {
    listen 443 ssl;
    server_name metrics.ippan.io;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location /metrics {
        auth_basic "Restricted";
        auth_basic_user_file /etc/nginx/.htpasswd;
        proxy_pass http://127.0.0.1:9615/metrics;
    }
}
```

Generate `.htpasswd`:
```bash
sudo htpasswd -c /etc/nginx/.htpasswd prometheus_user
```

---

## Troubleshooting

### Issue: Metrics endpoint returns 404

**Cause:** Metrics not enabled or wrong bind address

**Fix:**
```bash
# Check config
grep metrics_enabled config/ippan_node.toml

# Verify node logs
tail -f logs/ippan-node.log | grep metrics

# Test locally
curl http://127.0.0.1:9615/metrics
```

### Issue: Prometheus cannot scrape target

**Cause:** Firewall blocking or wrong target address

**Fix:**
```bash
# Check if port is listening
sudo netstat -tlnp | grep 9615

# Test from Prometheus host
curl http://<node-ip>:9615/metrics

# Check Prometheus targets
# http://localhost:9090/targets
```

### Issue: Missing metrics in Grafana dashboard

**Cause:** Metric names changed or not yet reported

**Fix:**
```bash
# Query Prometheus for available metrics
curl http://localhost:9090/api/v1/label/__name__/values | jq '.data[]' | grep ippan

# Verify node is emitting metrics
curl http://localhost:9615/metrics | grep ippan_consensus_rounds_total
```

---

## Performance Impact

### Overhead by Configuration

| Config | CPU Impact | Memory Impact | Network Impact |
|--------|------------|---------------|----------------|
| **Metrics disabled** | 0% | 0 MB | 0 B/s |
| **Basic metrics** | <1% | ~5 MB | ~10 KB/s |
| **Detailed tracing** | ~3-5% | ~20 MB | ~50 KB/s |

**Recommendation:** Use basic metrics for production. Enable detailed tracing only for debugging specific issues.

---

## Advanced Topics

### Custom Metrics

To add custom metrics, modify node code:

```rust
use prometheus::{Counter, Opts, Registry};

let custom_counter = Counter::with_opts(
    Opts::new("ippan_custom_events_total", "Custom event counter")
).unwrap();

registry.register(Box::new(custom_counter.clone())).unwrap();

// Increment in code
custom_counter.inc();
```

### Aggregation Queries

**Average finality latency over 1 hour:**
```promql
avg_over_time(ippan_consensus_finality_latency_seconds[1h])
```

**Peer count across cluster:**
```promql
sum(ippan_network_peers_total)
```

**DLC primary selection rate by validator:**
```promql
rate(ippan_dlc_primary_selections_total[1h])
```

---

## Summary

✅ **Metrics Enabled:** Configure `metrics_enabled = true`  
✅ **Prometheus Scraping:** Configure scrape targets  
✅ **Grafana Dashboards:** Import from `grafana_dashboards/`  
✅ **Alerting:** Set up critical alerts (node down, high latency, low peers)  
✅ **Security:** Bind to loopback in production, use reverse proxy with auth  

**Next Steps:**
1. Enable metrics on your nodes
2. Set up Prometheus and Grafana
3. Import IPPAN dashboards
4. Configure alerting rules
5. Monitor consensus health, network connectivity, and DLC fairness

---

**Documentation Version:** v1.0.0-rc1  
**Last Updated:** 2025-11-24  
**Support:** IPPAN Community Discord / GitHub Issues
