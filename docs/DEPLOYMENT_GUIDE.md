# IPPAN Deployment Guide

## 🚀 Production Deployment Guide

This guide covers deploying IPPAN to production environments with enterprise-grade infrastructure, comprehensive monitoring, and high-performance optimizations.

## 📋 Prerequisites

### System Requirements
- **CPU**: 8+ cores (16+ recommended for high-throughput)
- **RAM**: 16GB+ (32GB+ recommended for production)
- **Storage**: 1TB+ SSD (10TB+ recommended for full node)
- **Network**: 1Gbps+ bandwidth

### Software Requirements
- **Docker**: 20.10+
- **Kubernetes**: 1.24+
- **kubectl**: Latest version
- **Helm**: 3.8+ (optional)

## 🏗️ Deployment Options

### Option 1: Docker Compose (Recommended for Small-Medium Deployments)

```bash
# Clone repository
git clone https://github.com/ippan/ippan.git
cd ippan

# Quick deployment with automated setup
./scripts/deploy-production.sh

# Or manual deployment
docker build -f Dockerfile.optimized -t ippan:latest .
docker-compose -f docker-compose.production.yml up -d

# Check status
docker-compose -f docker-compose.production.yml ps

# Health check
./scripts/health-check.sh
```

### Option 2: Kubernetes (Recommended for Large-Scale Production)

```bash
# Deploy to Kubernetes
kubectl apply -f deployments/kubernetes/ippan-production.yaml

# Verify deployment
kubectl get pods -l app=ippan-node -n ippan-production
kubectl get services -n ippan-production
kubectl get ingress -n ippan-production

# Check auto-scaling
kubectl get hpa -n ippan-production

# Monitor deployment
kubectl logs -l app=ippan-node -n ippan-production -f
```

### Option 3: Manual Docker Deployment

```bash
# Build image
docker build -f Dockerfile.production -t ippan:latest .

# Run container
docker run -d \
  --name ippan-node \
  -p 80:80 \
  -p 8080:8080 \
  -p 3000:3000 \
  -v ippan-data:/data \
  -v ippan-keys:/keys \
  -v ippan-logs:/logs \
  -e RUST_LOG=info \
  -e NODE_ENV=production \
  ippan:latest
```

## 🔧 Configuration

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `RUST_LOG` | Log level | `info` | No |
| `IPPAN_CONFIG_PATH` | Config file path | `/config/default.toml` | No |
| `IPPAN_DATA_DIR` | Data directory | `/data` | No |
| `IPPAN_KEYS_DIR` | Keys directory | `/keys` | No |
| `IPPAN_LOG_DIR` | Logs directory | `/logs` | No |
| `NODE_ENV` | Environment | `production` | No |

### Configuration File

The main configuration is in `/config/default.toml`:

```toml
[network]
listen_addr = "0.0.0.0:8080"
bootstrap_nodes = []
max_connections = 1000
connection_timeout = 30
enable_nat = true
enable_relay = true

[storage]
db_path = "/data/ippan.db"
max_storage_size = 107374182400  # 100GB
shard_size = 1048576  # 1MB
replication_factor = 3
enable_encryption = true
proof_interval = 3600

[consensus]
block_time = 1000
max_block_size = 1048576  # 1MB
validator_count = 21
stake_threshold = 1000000000  # 10 IPN

[api]
listen_addr = "0.0.0.0:8080"
cors_origins = ["*"]
rate_limit = 1000
timeout = 30

[logging]
level = "info"
format = "json"
output = "stdout"
```

## 📊 Monitoring Setup

### Prometheus Configuration

```yaml
# deployments/monitoring/prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'ippan-nodes'
    static_configs:
      - targets: ['ippan-service:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
```

### Grafana Dashboard

Import the IPPAN dashboard from `deployments/monitoring/grafana-dashboard.json`.

### Alerting Rules

```yaml
# deployments/monitoring/ippan_rules.yml
groups:
- name: ippan_alerts
  rules:
  - alert: HighErrorRate
    expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High error rate detected"
```

## 🔒 Security Configuration

### SSL/TLS Setup

```bash
# Generate SSL certificates
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout /etc/ssl/private/ippan.key \
  -out /etc/ssl/certs/ippan.crt

# Update nginx configuration
# Add SSL configuration to deployments/nginx/nginx.conf
```

### Firewall Configuration

```bash
# Allow required ports
ufw allow 80/tcp
ufw allow 443/tcp
ufw allow 8080/tcp
ufw allow 3000/tcp

# Enable firewall
ufw enable
```

### Rate Limiting

Configure rate limiting in nginx:

```nginx
# deployments/nginx/nginx.conf
limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;

location /api/ {
    limit_req zone=api burst=20 nodelay;
    # ... rest of configuration
}
```

## 📈 Scaling

### Horizontal Scaling (Kubernetes)

```bash
# Scale deployment
kubectl scale deployment ippan-node --replicas=5

# Set up auto-scaling
kubectl autoscale deployment ippan-node --cpu-percent=70 --min=3 --max=10
```

### Vertical Scaling

Update resource limits in Kubernetes deployment:

```yaml
resources:
  requests:
    memory: "1Gi"
    cpu: "500m"
  limits:
    memory: "4Gi"
    cpu: "2000m"
```

## 🔄 Backup & Recovery

### Automated Backups

```bash
# Create backup script
cat > /usr/local/bin/ippan-backup.sh << 'EOF'
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backups/ippan"
mkdir -p $BACKUP_DIR

# Backup database
kubectl exec -it deployment/ippan-node -- tar czf - /data > $BACKUP_DIR/ippan-data-$DATE.tar.gz

# Backup configuration
kubectl get configmap ippan-config -o yaml > $BACKUP_DIR/ippan-config-$DATE.yaml

# Cleanup old backups (keep 30 days)
find $BACKUP_DIR -name "*.tar.gz" -mtime +30 -delete
find $BACKUP_DIR -name "*.yaml" -mtime +30 -delete
EOF

chmod +x /usr/local/bin/ippan-backup.sh

# Schedule daily backups
echo "0 2 * * * /usr/local/bin/ippan-backup.sh" | crontab -
```

### Disaster Recovery

```bash
# Restore from backup
kubectl delete deployment ippan-node
kubectl apply -f deployments/kubernetes/ippan-deployment.yaml

# Restore data
tar xzf /backups/ippan/ippan-data-20240101_020000.tar.gz -C /tmp/
kubectl cp /tmp/data ippan-node-pod:/data
```

## 🚨 Troubleshooting

### Common Issues

#### 1. Pod Not Starting
```bash
# Check pod status
kubectl describe pod <pod-name>

# Check logs
kubectl logs <pod-name>
```

#### 2. High Memory Usage
```bash
# Check resource usage
kubectl top pods -l app=ippan-node

# Adjust resource limits
kubectl edit deployment ippan-node
```

#### 3. Network Issues
```bash
# Check service endpoints
kubectl get endpoints ippan-service

# Test connectivity
kubectl exec -it <pod-name> -- curl http://localhost:8080/health
```

### Performance Tuning

#### Database Optimization
```bash
# Optimize database
kubectl exec -it <pod-name> -- sqlite3 /data/ippan.db "VACUUM;"
kubectl exec -it <pod-name> -- sqlite3 /data/ippan.db "ANALYZE;"
```

#### Memory Optimization
```bash
# Adjust Rust memory settings
export RUST_LOG=info
export MALLOC_ARENA_MAX=2
```

## 📞 Support

### Health Checks

```bash
# Check application health
curl http://localhost:8080/health

# Check metrics
curl http://localhost:8080/metrics
```

### Logs

```bash
# View logs
kubectl logs -l app=ippan-node -f

# View specific pod logs
kubectl logs <pod-name> --tail=100
```

### Monitoring

Access monitoring dashboards:
- **Grafana**: http://localhost:3000
- **Prometheus**: http://localhost:9090
- **AlertManager**: http://localhost:9093

## 🎯 Production Checklist

- [ ] SSL certificates configured
- [ ] Firewall rules applied
- [ ] Monitoring setup complete
- [ ] Backup strategy implemented
- [ ] Load testing completed
- [ ] Security audit passed
- [ ] Documentation updated
- [ ] Support team trained
- [ ] Rollback plan tested
- [ ] Performance benchmarks met

## 📚 Additional Resources

- [Architecture Overview](architecture.md)
- [API Reference](api_reference.md)
- [Security Guide](SECURITY_GUIDE.md)
- [Monitoring Guide](MONITORING_GUIDE.md)
- [Troubleshooting Guide](TROUBLESHOOTING_GUIDE.md)