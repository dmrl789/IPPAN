# IPPAN Production Deployment Guide

This guide provides comprehensive instructions for deploying IPPAN to production with high availability, security, and performance optimizations.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Architecture Overview](#architecture-overview)
3. [Deployment Options](#deployment-options)
4. [Docker Compose Deployment](#docker-compose-deployment)
5. [Kubernetes Deployment](#kubernetes-deployment)
6. [Configuration](#configuration)
7. [Monitoring and Observability](#monitoring-and-observability)
8. [Security Hardening](#security-hardening)
9. [Backup and Recovery](#backup-and-recovery)
10. [Maintenance and Updates](#maintenance-and-updates)
11. [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

- **CPU**: 8+ cores (16+ recommended for high-throughput)
- **RAM**: 16GB+ (32GB+ recommended)
- **Storage**: 1TB+ SSD (10TB+ recommended for full node)
- **Network**: 1Gbps+ bandwidth
- **OS**: Linux (Ubuntu 20.04+ recommended) or Windows Server 2019+

### Software Requirements

- Docker 20.10+
- Docker Compose 2.0+
- Kubernetes 1.21+ (for K8s deployment)
- kubectl (for K8s deployment)
- OpenSSL (for certificate generation)
- curl (for health checks)

### Network Requirements

- **Ports**:
  - 8080: P2P network communication
  - 3000: API endpoints
  - 80/443: HTTP/HTTPS frontend
  - 9090: Prometheus metrics
  - 3001: Grafana dashboard
  - 9093: Alertmanager
  - 6379: Redis cache
  - 9200: Elasticsearch
  - 5601: Kibana

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    IPPAN Production Stack                   │
├─────────────────────────────────────────────────────────────┤
│  Load Balancer (Nginx)                                      │
│  ├── SSL Termination                                        │
│  ├── Rate Limiting                                          │
│  └── Health Checks                                          │
├─────────────────────────────────────────────────────────────┤
│  IPPAN Node Cluster                                         │
│  ├── Consensus Engine                                       │
│  ├── P2P Network                                            │
│  ├── Storage Layer                                          │
│  ├── API Gateway                                            │
│  └── Performance Optimizations                              │
├─────────────────────────────────────────────────────────────┤
│  Monitoring Stack                                           │
│  ├── Prometheus (Metrics)                                   │
│  ├── Grafana (Dashboards)                                   │
│  ├── Alertmanager (Alerts)                                  │
│  └── ELK Stack (Logs)                                       │
├─────────────────────────────────────────────────────────────┤
│  Supporting Services                                        │
│  ├── Redis (Caching)                                        │
│  ├── Backup Service                                         │
│  └── Security Services                                      │
└─────────────────────────────────────────────────────────────┘
```

## Deployment Options

### Option 1: Docker Compose (Recommended for Small-Medium Deployments)

Best for:
- Single-node deployments
- Development and testing
- Small to medium production environments
- Quick setup and management

### Option 2: Kubernetes (Recommended for Large-Scale Deployments)

Best for:
- Multi-node clusters
- High availability requirements
- Auto-scaling needs
- Enterprise deployments

## Docker Compose Deployment

### Quick Start

1. **Clone the repository**:
   ```bash
   git clone https://github.com/ippan/ippan.git
   cd ippan
   ```

2. **Set environment variables**:
   ```bash
   export BACKUP_ENCRYPTION_KEY="your-secure-backup-key"
   export DEPLOYMENT_ENV="production"
   ```

3. **Run the deployment script**:
   ```bash
   # Linux/macOS
   ./scripts/deploy-production.sh
   
   # Windows
   scripts\deploy-production.bat
   ```

### Manual Deployment

1. **Build the images**:
   ```bash
   docker build -f Dockerfile.optimized -t ippan/ippan:latest .
   ```

2. **Generate SSL certificates**:
   ```bash
   mkdir -p ssl
   openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
     -keyout ssl/ippan.key \
     -out ssl/ippan.crt \
     -subj "/C=US/ST=State/L=City/O=IPPAN/CN=ippan.network"
   ```

3. **Start the services**:
   ```bash
   docker-compose -f docker-compose.production.yml up -d
   ```

4. **Verify deployment**:
   ```bash
   docker-compose -f docker-compose.production.yml ps
   curl http://localhost:80/health
   ```

### Configuration

The production configuration is located in `config/production.toml`. Key settings:

```toml
[network]
listen_addr = "0.0.0.0:8080"
max_connections = 1000
enable_tls = true

[storage]
max_storage_size = 1099511627776  # 1TB
replication_factor = 3
enable_encryption = true

[consensus]
block_time = 1000  # 1 second
validator_count = 21
stake_threshold = 1000000000  # 10 IPN

[performance]
enable_lockfree = true
enable_memory_pool = true
memory_pool_size = 1073741824  # 1GB
```

## Kubernetes Deployment

### Prerequisites

- Kubernetes cluster (1.21+)
- kubectl configured
- Persistent volume support
- Load balancer or ingress controller

### Deployment Steps

1. **Create namespace**:
   ```bash
   kubectl create namespace ippan-production
   ```

2. **Apply the deployment**:
   ```bash
   kubectl apply -f deployments/kubernetes/ippan-production.yaml
   ```

3. **Wait for deployment**:
   ```bash
   kubectl wait --for=condition=available --timeout=300s deployment/ippan-node -n ippan-production
   ```

4. **Verify deployment**:
   ```bash
   kubectl get pods -n ippan-production
   kubectl get services -n ippan-production
   ```

### Scaling

```bash
# Scale to 5 replicas
kubectl scale deployment ippan-node --replicas=5 -n ippan-production

# Enable auto-scaling
kubectl autoscale deployment ippan-node --cpu-percent=70 --min=3 --max=10 -n ippan-production
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level | `info` |
| `IPPAN_CONFIG_PATH` | Config file path | `/config/default.toml` |
| `IPPAN_DATA_DIR` | Data directory | `/data` |
| `IPPAN_KEYS_DIR` | Keys directory | `/keys` |
| `IPPAN_LOG_DIR` | Logs directory | `/logs` |
| `NODE_ENV` | Environment | `production` |
| `BACKUP_ENCRYPTION_KEY` | Backup encryption key | Required |

### Performance Tuning

#### Memory Optimization
```toml
[performance]
memory_pool_size = 2147483648  # 2GB
cache_size = 4294967296        # 4GB
thread_pool_size = 16          # Adjust based on CPU cores
```

#### Network Optimization
```toml
[network]
max_connections = 2000         # Increase for high-traffic
connection_timeout = 60        # Increase for slow networks
enable_compression = true      # Enable for bandwidth optimization
```

#### Storage Optimization
```toml
[storage]
shard_size = 2097152          # 2MB for better throughput
replication_factor = 5        # Increase for higher availability
enable_compression = true     # Enable for space optimization
```

## Monitoring and Observability

### Prometheus Metrics

Access Prometheus at `http://localhost:9090`

Key metrics to monitor:
- `ippan_consensus_blocks_per_second`
- `ippan_network_active_connections`
- `ippan_storage_used_bytes`
- `ippan_performance_request_duration_seconds`
- `ippan_validator_status`

### Grafana Dashboards

Access Grafana at `http://localhost:3001` (admin/admin123)

Pre-configured dashboards:
- IPPAN Node Overview
- Network Performance
- Storage Metrics
- Consensus Health
- Security Alerts

### Alerting

Alertmanager is configured with rules for:
- Node downtime
- High CPU/memory usage
- Network issues
- Storage problems
- Security threats

### Log Aggregation

ELK Stack (Elasticsearch, Logstash, Kibana) for centralized logging:
- Access Kibana at `http://localhost:5601`
- Structured JSON logs
- Real-time log analysis
- Log retention policies

## Security Hardening

### SSL/TLS Configuration

1. **Generate certificates**:
   ```bash
   # Self-signed (development)
   openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
     -keyout ssl/ippan.key -out ssl/ippan.crt
   
   # Let's Encrypt (production)
   certbot certonly --standalone -d ippan.network
   ```

2. **Configure TLS**:
   ```toml
   [security]
   enable_tls = true
   certificate_path = "/ssl/ippan.crt"
   private_key_path = "/ssl/ippan.key"
   ```

### Network Security

- Enable firewall rules
- Use VPN for management access
- Implement rate limiting
- Enable DDoS protection
- Use secure communication protocols

### Key Management

- Store keys in secure locations
- Enable key rotation
- Use hardware security modules (HSM)
- Implement key escrow for recovery
- Enable audit logging

### Access Control

- Implement RBAC
- Use strong authentication
- Enable session management
- Implement API rate limiting
- Enable audit trails

## Backup and Recovery

### Automated Backups

Backups are configured to run daily at 2 AM:

```bash
# Manual backup
docker exec ippan-backup /scripts/backup.sh

# Restore from backup
docker exec ippan-backup /scripts/restore.sh backup_file.tar.gz.enc
```

### Backup Configuration

```toml
[backup]
enable_backup = true
backup_interval = 86400        # 24 hours
backup_retention_days = 30
backup_encryption = true
backup_compression = true
```

### Disaster Recovery

1. **Full system backup**:
   ```bash
   docker-compose -f docker-compose.production.yml down
   tar -czf ippan-full-backup-$(date +%Y%m%d).tar.gz data/ keys/ config/
   ```

2. **Restore procedure**:
   ```bash
   tar -xzf ippan-full-backup-YYYYMMDD.tar.gz
   docker-compose -f docker-compose.production.yml up -d
   ```

## Maintenance and Updates

### Rolling Updates

#### Docker Compose
```bash
# Update image
docker pull ippan/ippan:latest
docker-compose -f docker-compose.production.yml up -d --force-recreate
```

#### Kubernetes
```bash
# Update deployment
kubectl set image deployment/ippan-node ippan=ippan/ippan:latest -n ippan-production
kubectl rollout status deployment/ippan-node -n ippan-production
```

### Database Maintenance

```bash
# Optimize database
docker exec ippan-node ippan db optimize

# Rebuild indexes
docker exec ippan-node ippan db rebuild-indexes

# Clean up old data
docker exec ippan-node ippan db cleanup --older-than 30d
```

### Log Management

```bash
# Rotate logs
docker exec ippan-node logrotate /etc/logrotate.conf

# Clean old logs
find /logs -name "*.log.*" -mtime +30 -delete
```

## Troubleshooting

### Common Issues

#### Node Won't Start
```bash
# Check logs
docker logs ippan-node

# Check configuration
docker exec ippan-node ippan config validate

# Check resources
docker stats ippan-node
```

#### High Memory Usage
```bash
# Check memory usage
docker exec ippan-node ippan metrics memory

# Adjust memory pool size
# Edit config/production.toml
memory_pool_size = 1073741824  # 1GB
```

#### Network Connectivity Issues
```bash
# Check network status
docker exec ippan-node ippan network status

# Check peer connections
docker exec ippan-node ippan network peers

# Test connectivity
docker exec ippan-node ping bootstrap.ippan.network
```

#### Storage Issues
```bash
# Check storage usage
docker exec ippan-node ippan storage status

# Check disk space
df -h

# Clean up old data
docker exec ippan-node ippan storage cleanup
```

### Performance Issues

#### Slow Block Processing
- Increase `thread_pool_size`
- Enable `parallel_validation`
- Optimize `batch_size`

#### High Network Latency
- Increase `connection_timeout`
- Enable `compression`
- Optimize `max_connections`

#### Memory Leaks
- Enable `memory_pool`
- Adjust `cache_size`
- Monitor `memory_usage`

### Monitoring Issues

#### Prometheus Not Collecting Metrics
```bash
# Check Prometheus status
curl http://localhost:9090/-/healthy

# Check metrics endpoint
curl http://localhost:8080/metrics

# Restart Prometheus
docker-compose -f docker-compose.production.yml restart prometheus
```

#### Grafana Dashboard Issues
```bash
# Check Grafana status
curl http://localhost:3001/api/health

# Check datasource
curl http://localhost:3001/api/datasources

# Restart Grafana
docker-compose -f docker-compose.production.yml restart grafana
```

### Security Issues

#### SSL Certificate Problems
```bash
# Check certificate validity
openssl x509 -in ssl/ippan.crt -text -noout

# Renew certificate
certbot renew

# Restart services
docker-compose -f docker-compose.production.yml restart ippan-node
```

#### Authentication Failures
```bash
# Check authentication logs
docker logs ippan-node | grep auth

# Reset user passwords
docker exec ippan-node ippan auth reset-password username

# Check JWT configuration
docker exec ippan-node ippan auth status
```

## Support and Resources

### Documentation
- [API Documentation](API_DOCUMENTATION.md)
- [Security Guide](SECURITY_GUIDE.md)
- [Monitoring Guide](MONITORING_GUIDE.md)
- [Developer Guide](developer_guide.md)

### Community
- [GitHub Issues](https://github.com/ippan/ippan/issues)
- [Discord Community](https://discord.gg/ippan)
- [Telegram Group](https://t.me/ippan_network)

### Professional Support
- Enterprise support available
- 24/7 monitoring and alerting
- Custom deployment assistance
- Performance optimization consulting

---

For additional help, please refer to the troubleshooting section or contact the IPPAN support team.
