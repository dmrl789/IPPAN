# IPPAN AI Service Deployment Guide

This guide covers deploying the IPPAN AI Service in production environments.

## Prerequisites

- Docker and Docker Compose
- Kubernetes cluster (for K8s deployment)
- LLM API key (OpenAI, Anthropic, etc.)
- Monitoring stack (Prometheus, Grafana)
- Load balancer (Nginx, HAProxy)

## Quick Start

### 1. Clone and Build

```bash
git clone <repository-url>
cd ippan-ai-service
cargo build --release
```

### 2. Configure Environment

```bash
# Copy example configuration
cp secrets/production.env.example secrets/production.env

# Edit configuration
vim secrets/production.env
```

### 3. Deploy with Docker Compose

```bash
# Production deployment
docker-compose -f docker-compose.prod.yml up -d

# Check status
docker-compose -f docker-compose.prod.yml ps
```

## Configuration

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `IPPAN_ENV` | Environment (`development`, `staging`, `production`, `testing`) | `development` | Yes |
| `LLM_API_KEY` | LLM API key (may also be provided as `IPPAN_SECRET_LLM_API_KEY`) | - | Yes |
| `LLM_API_ENDPOINT` | LLM provider URL | `https://api.openai.com/v1` | No |
| `LLM_MODEL` | LLM model name | `gpt-4` | No |
| `LLM_MAX_TOKENS` | Maximum tokens per completion | `4000` | No |
| `LLM_TEMPERATURE` | Sampling temperature | `0.7` | No |
| `LLM_TIMEOUT` | LLM request timeout (seconds) | `30` | No |
| `ENABLE_LLM` | Enable LLM features | `true` | No |
| `ENABLE_ANALYTICS` | Enable analytics | `true` | No |
| `ENABLE_SMART_CONTRACTS` | Enable smart contract analysis | `true` | No |
| `ENABLE_MONITORING` | Enable monitoring | `true` | No |
| `MONITORING_INTERVAL` | Metrics emission interval (seconds) | `30` | No |
| `PROMETHEUS_ENDPOINT` | Prometheus remote write endpoint (required for production metrics) | `http://prometheus:9090/api/v1/write` | Yes (production) |
| `JSON_EXPORTER_ENDPOINT` | JSON exporter URL | `http://localhost:8080/metrics` | No |
| `HEALTH_PORT` | Health server port | `8080` | No |
| `LOG_LEVEL` | Log level | `info` | No |
| `LOG_FORMAT` | Log format (`json`/`pretty`) | `pretty` | No |

For local deployments, copy `.env.example` to `.env` and fill in the required values before starting the service.

### Configuration Files

Create configuration files in the `config/` directory:

- `config/production.toml` - Production settings
- `config/staging.toml` - Staging settings
- `config/development.toml` - Development settings

## Deployment Methods

### Docker Compose

#### Production

```bash
# Deploy production stack
docker-compose -f docker-compose.prod.yml up -d

# View logs
docker-compose -f docker-compose.prod.yml logs -f

# Scale service
docker-compose -f docker-compose.prod.yml up -d --scale ai-service=3
```

#### Staging

```bash
# Deploy staging stack
docker-compose -f docker-compose.staging.yml up -d
```

#### Development

```bash
# Deploy development stack
docker-compose -f docker-compose.dev.yml up -d
```

### Kubernetes

#### Create Namespace

```bash
kubectl create namespace ippan-ai
```

#### Deploy ConfigMap

```bash
kubectl apply -f k8s/configmap.yaml
```

#### Deploy Secrets

```bash
kubectl apply -f k8s/secrets.yaml
```

#### Deploy Service

```bash
kubectl apply -f k8s/service.yaml
```

#### Deploy Deployment

```bash
kubectl apply -f k8s/deployment.yaml
```

#### Deploy Ingress

```bash
kubectl apply -f k8s/ingress.yaml
```

### Manual Deployment

#### Build Binary

```bash
cargo build --release --package ippan-ai-service
```

#### Install Systemd Service

```bash
# Copy binary
sudo cp target/release/ippan-ai-service /usr/local/bin/

# Copy systemd service
sudo cp deploy/ippan-ai-service.service /etc/systemd/system/

# Enable and start service
sudo systemctl enable ippan-ai-service
sudo systemctl start ippan-ai-service
```

## Monitoring

### Health Checks

The service exposes health check endpoints:

- `GET /health` - Service health status
- `GET /metrics` - Prometheus metrics

### Prometheus Metrics

Key metrics to monitor:

- `ippan_ai_requests_total` - Total requests
- `ippan_ai_request_duration_seconds` - Request duration
- `ippan_ai_errors_total` - Total errors
- `ippan_ai_memory_usage_bytes` - Memory usage
- `ippan_ai_cpu_usage_percent` - CPU usage

### Grafana Dashboards

Import the provided dashboard:

```bash
# Import dashboard
curl -X POST \
  http://admin:admin@localhost:3000/api/dashboards/db \
  -H 'Content-Type: application/json' \
  -d @monitoring/grafana/dashboards/ai-service.json
```

## Security

### Authentication

Configure authentication in the service configuration:

```toml
[security]
enable_authentication = true
session_timeout = 3600
```

### Encryption

Enable encryption for sensitive data:

```toml
[security]
enable_encryption = true
```

### Secrets Management

Use environment variables or secret management systems:

```bash
# Using environment variables
export LLM_API_KEY=your-api-key

# Using Docker secrets
echo "your-api-key" | docker secret create llm_api_key -
```

## Performance Tuning

### Resource Limits

Set appropriate resource limits:

```yaml
resources:
  limits:
    memory: 2Gi
    cpu: 1000m
  requests:
    memory: 1Gi
    cpu: 500m
```

### Connection Pooling

Configure connection pooling:

```toml
[database]
max_connections = 100
min_connections = 10
connection_timeout = 30
```

### Caching

Enable caching for better performance:

```toml
[cache]
enable = true
ttl_seconds = 3600
max_size = 1000
```

## Troubleshooting

### Common Issues

1. **Service won't start**
   - Check configuration files
   - Verify environment variables
   - Check logs for errors

2. **LLM requests failing**
   - Verify API key is correct
   - Check network connectivity
   - Verify API endpoint URL

3. **High memory usage**
   - Adjust memory limits
   - Check for memory leaks
   - Monitor garbage collection

4. **Slow response times**
   - Check CPU usage
   - Verify network latency
   - Review database queries

### Logs

View service logs:

```bash
# Docker Compose
docker-compose -f docker-compose.prod.yml logs -f ai-service

# Kubernetes
kubectl logs -f deployment/ippan-ai-service -n ippan-ai

# Systemd
journalctl -u ippan-ai-service -f
```

### Debugging

Enable debug logging:

```bash
export RUST_LOG=debug
export LOG_LEVEL=debug
```

## Maintenance

### Updates

Update the service:

```bash
# Pull latest image
docker-compose -f docker-compose.prod.yml pull

# Restart service
docker-compose -f docker-compose.prod.yml up -d
```

### Backups

Backup configuration and data:

```bash
# Backup configuration
tar -czf config-backup.tar.gz config/

# Backup data
tar -czf data-backup.tar.gz data/
```

### Scaling

Scale the service:

```bash
# Docker Compose
docker-compose -f docker-compose.prod.yml up -d --scale ai-service=3

# Kubernetes
kubectl scale deployment ippan-ai-service --replicas=3 -n ippan-ai
```

## Support

For support and questions:

- Create an issue on GitHub
- Join our Discord community
- Check the documentation
- Review the troubleshooting guide