# IPPAN Monitoring Guide

## 📊 Comprehensive Monitoring & Observability

This guide covers comprehensive monitoring, alerting, and observability for IPPAN in production environments, including performance metrics, security monitoring, and health checks.

## 🎯 Monitoring Strategy

### Three Pillars of Observability
1. **Metrics**: Quantitative data about system performance
2. **Logs**: Detailed event records for debugging
3. **Traces**: Request flow through distributed systems

### Monitoring Layers
- **Infrastructure**: CPU, memory, disk, network
- **Application**: Response times, error rates, throughput
- **Performance**: TPS, latency, cache hit rates, memory usage
- **Business**: Transactions, users, revenue
- **Security**: Failed logins, suspicious activity, key rotation
- **Consensus**: Block production, validator status, fork detection
- **Storage**: DHT performance, replication, proof generation

## 📈 Metrics Collection

### Prometheus Configuration
```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'ippan-nodes'
    static_configs:
      - targets: ['ippan-service:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
    scrape_timeout: 5s
```

### Key Metrics to Monitor

#### System Metrics
```prometheus
# CPU usage
node_cpu_seconds_total

# Memory usage
node_memory_MemTotal_bytes
node_memory_MemAvailable_bytes

# Disk usage
node_filesystem_size_bytes
node_filesystem_avail_bytes

# Network traffic
node_network_receive_bytes_total
node_network_transmit_bytes_total
```

#### Application Metrics
```prometheus
# Transaction throughput
ippan_transactions_total
rate(ippan_transactions_total[5m])

# Block production
ippan_blocks_total
ippan_block_production_time_seconds

# Network peers
ippan_peers_total
ippan_peer_connections_total

# Storage metrics
ippan_storage_files_total
ippan_storage_size_bytes

# API metrics
http_requests_total
http_request_duration_seconds
```

#### Business Metrics
```prometheus
# User activity
ippan_active_users_total
ippan_new_registrations_total

# Financial metrics
ippan_total_value_locked
ippan_transaction_volume_total

# Domain registrations
ippan_domains_registered_total
ippan_domains_expired_total
```

## 📊 Grafana Dashboards

### Main Dashboard
```json
{
  "dashboard": {
    "title": "IPPAN Main Dashboard",
    "panels": [
      {
        "title": "Transaction Throughput",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(ippan_transactions_total[5m])",
            "legendFormat": "TPS"
          }
        ]
      },
      {
        "title": "Block Production Time",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(ippan_block_production_time_seconds_bucket[5m]))",
            "legendFormat": "95th percentile"
          }
        ]
      },
      {
        "title": "Active Peers",
        "type": "singlestat",
        "targets": [
          {
            "expr": "ippan_peers_total",
            "legendFormat": "Peers"
          }
        ]
      }
    ]
  }
}
```

### Infrastructure Dashboard
```json
{
  "dashboard": {
    "title": "IPPAN Infrastructure",
    "panels": [
      {
        "title": "CPU Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "100 - (avg(rate(node_cpu_seconds_total{mode=\"idle\"}[5m])) * 100)",
            "legendFormat": "CPU %"
          }
        ]
      },
      {
        "title": "Memory Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "(node_memory_MemTotal_bytes - node_memory_MemAvailable_bytes) / node_memory_MemTotal_bytes * 100",
            "legendFormat": "Memory %"
          }
        ]
      },
      {
        "title": "Disk Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "(node_filesystem_size_bytes - node_filesystem_avail_bytes) / node_filesystem_size_bytes * 100",
            "legendFormat": "Disk %"
          }
        ]
      }
    ]
  }
}
```

## 🚨 Alerting Rules

### Critical Alerts
```yaml
# critical_alerts.yml
groups:
- name: critical
  rules:
  - alert: IPPANNodeDown
    expr: up{job="ippan-nodes"} == 0
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "IPPAN node is down"
      description: "IPPAN node {{ $labels.instance }} is down"

  - alert: HighErrorRate
    expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High error rate detected"
      description: "Error rate is {{ $value }} errors per second"

  - alert: LowTPS
    expr: rate(ippan_transactions_total[5m]) < 1000
    for: 10m
    labels:
      severity: critical
    annotations:
      summary: "Low transaction throughput"
      description: "TPS is {{ $value }} transactions per second"
```

### Warning Alerts
```yaml
# warning_alerts.yml
groups:
- name: warning
  rules:
  - alert: HighMemoryUsage
    expr: (node_memory_MemTotal_bytes - node_memory_MemAvailable_bytes) / node_memory_MemTotal_bytes > 0.9
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High memory usage"
      description: "Memory usage is above 90% on {{ $labels.instance }}"

  - alert: HighResponseTime
    expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 1
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High response time detected"
      description: "95th percentile response time is {{ $value }} seconds"
```

## 📝 Logging Strategy

### Structured Logging
```rust
// Rust structured logging
use serde_json::json;
use tracing::{info, warn, error};

// Log transaction
info!(
    transaction_id = %tx_id,
    from = %from_address,
    to = %to_address,
    amount = %amount,
    "Transaction processed"
);

// Log error
error!(
    error = %e,
    transaction_id = %tx_id,
    "Transaction failed"
);
```

### Log Aggregation
```yaml
# Fluentd configuration
<source>
  @type tail
  path /var/log/ippan/*.log
  pos_file /var/log/fluentd/ippan.log.pos
  tag ippan.*
  format json
</source>

<match ippan.**>
  @type elasticsearch
  host elasticsearch
  port 9200
  index_name ippan-logs
</match>
```

### Log Analysis Queries
```json
// Elasticsearch queries
{
  "query": {
    "bool": {
      "must": [
        {
          "range": {
            "@timestamp": {
              "gte": "now-1h"
            }
          }
        },
        {
          "term": {
            "level": "error"
          }
        }
      ]
    }
  }
}
```

## 🔍 Distributed Tracing

### OpenTelemetry Configuration
```rust
// Rust tracing setup
use opentelemetry::{
    global,
    sdk::{
        trace::{self, RandomIdGenerator, Sampler},
        Resource,
    },
    trace::{TraceError, Tracer},
};
use opentelemetry_jaeger::new_agent_pipeline;

fn init_tracer() -> Result<impl Tracer, TraceError> {
    new_agent_pipeline()
        .with_service_name("ippan-node")
        .with_sampler(Sampler::AlwaysOn)
        .install()
}
```

### Trace Analysis
```go
// Jaeger query examples
// Find slow transactions
operation: "process_transaction" AND duration > 1s

// Find error traces
tags.error = true

// Find specific user activity
tags.user_id = "user123"
```

## 📊 Performance Monitoring

### Key Performance Indicators (KPIs)
- **Transaction Throughput**: TPS (Transactions Per Second)
- **Block Production Time**: Average time to produce blocks
- **Network Latency**: P2P communication delays
- **Storage Performance**: Read/write operations per second
- **API Response Time**: 95th percentile response times

### Performance Benchmarks
```bash
# Load testing script
#!/bin/bash
k6 run --vus 1000 --duration 60s load-test.js

# Benchmark results
echo "Target TPS: 1,000,000"
echo "Achieved TPS: $(k6 run --quiet load-test.js | grep 'http_reqs' | awk '{print $2}')"
```

### Performance Optimization
```rust
// Performance monitoring in Rust
use std::time::Instant;

let start = Instant::now();
// ... perform operation ...
let duration = start.elapsed();

// Record metric
metrics::histogram!("operation_duration", duration.as_secs_f64());
```

## 🔐 Security Monitoring

### Security Metrics
```prometheus
# Failed authentication attempts
ippan_security_failed_logins_total

# Rate limited requests
ippan_security_rate_limited_requests_total

# Suspicious activity
ippan_security_suspicious_activity_total

# Encryption errors
ippan_security_encryption_errors_total
```

### Security Alerts
```yaml
- alert: BruteForceAttack
  expr: rate(ippan_security_failed_logins_total[5m]) > 10
  for: 2m
  labels:
    severity: critical
  annotations:
    summary: "Potential brute force attack detected"
    description: "{{ $value }} failed login attempts per second"

- alert: HighRateLimit
  expr: rate(ippan_security_rate_limited_requests_total[5m]) > 100
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High rate limiting activity"
    description: "{{ $value }} rate limited requests per second"
```

## 📱 Alerting Channels

### AlertManager Configuration
```yaml
# alertmanager.yml
global:
  smtp_smarthost: 'localhost:587'
  smtp_from: 'alerts@ippan.network'

route:
  group_by: ['alertname']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'web.hook'

receivers:
- name: 'web.hook'
  webhook_configs:
  - url: 'http://127.0.0.1:5001/'

- name: 'email'
  email_configs:
  - to: 'admin@ippan.network'
    subject: 'IPPAN Alert: {{ .GroupLabels.alertname }}'
    body: |
      {{ range .Alerts }}
      Alert: {{ .Annotations.summary }}
      Description: {{ .Annotations.description }}
      {{ end }}

- name: 'slack'
  slack_configs:
  - api_url: 'https://hooks.slack.com/services/...'
    channel: '#alerts'
    title: 'IPPAN Alert'
    text: '{{ range .Alerts }}{{ .Annotations.summary }}{{ end }}'
```

### Notification Templates
```yaml
# Slack notification template
- name: 'slack'
  slack_configs:
  - api_url: 'https://hooks.slack.com/services/...'
    channel: '#alerts'
    title: 'IPPAN Alert: {{ .GroupLabels.alertname }}'
    text: |
      *Severity:* {{ .GroupLabels.severity }}
      *Instance:* {{ .GroupLabels.instance }}
      *Summary:* {{ .Annotations.summary }}
      *Description:* {{ .Annotations.description }}
      *Time:* {{ .StartsAt }}
```

## 🛠️ Monitoring Tools

### Prometheus Stack
```bash
# Install Prometheus
helm install prometheus prometheus-community/kube-prometheus-stack

# Access Grafana
kubectl port-forward svc/prometheus-grafana 3000:80
```

### ELK Stack
```bash
# Install Elasticsearch
helm install elasticsearch elastic/elasticsearch

# Install Kibana
helm install kibana elastic/kibana

# Install Logstash
helm install logstash elastic/logstash
```

### Jaeger
```bash
# Install Jaeger
helm install jaeger jaegertracing/jaeger

# Access Jaeger UI
kubectl port-forward svc/jaeger-query 16686:80
```

## 📋 Monitoring Checklist

### Pre-Deployment
- [ ] Prometheus configured and running
- [ ] Grafana dashboards created
- [ ] Alert rules defined
- [ ] Log aggregation setup
- [ ] Tracing configured
- [ ] Performance baselines established

### Post-Deployment
- [ ] Metrics collection verified
- [ ] Alerts tested
- [ ] Dashboards populated
- [ ] Log analysis working
- [ ] Performance monitoring active
- [ ] Security monitoring enabled

### Ongoing
- [ ] Regular alert testing
- [ ] Dashboard updates
- [ ] Performance optimization
- [ ] Capacity planning
- [ ] Incident response
- [ ] Documentation updates

## 📚 Additional Resources

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [ELK Stack Guide](https://www.elastic.co/guide/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [OpenTelemetry Guide](https://opentelemetry.io/docs/)
