# IPPAN Automated Deployment Guide

This guide explains how to use the automated GitHub Actions workflow for deploying the IPPAN blockchain network.

## Overview

The automated deployment workflow (`/.github/workflows/deploy.yml`) provides:

- **Automated Docker Image Building**: Builds and pushes images to GitHub Container Registry (GHCR)
- **Multi-Server Deployment**: Deploys to both Server 1 (full-stack) and Server 2 (node-only)
- **Health Checks**: Verifies service health and P2P connectivity
- **Zero-Downtime Updates**: Uses Docker Compose for seamless updates

## Architecture

### Server 1 (188.245.97.41) - Full Stack
- **IPPAN Node 1**: Blockchain node with RPC API (port 8080) and P2P (port 9000)
- **IPPAN Node 2**: Second blockchain node with RPC API (port 8081) and P2P (port 9001)
- **Gateway**: API gateway service (port 8081)
- **UI**: Web interface (if configured)

### Server 2 (135.181.145.174) - Node Only
- **IPPAN Node 1**: Blockchain node with RPC API (port 8080) and P2P (port 9000)
- **IPPAN Node 2**: Second blockchain node with RPC API (port 8080) and P2P (port 9001)

## Required GitHub Secrets

Configure these secrets in your repository settings (`Settings → Secrets → Actions`):

| Secret Name | Description | Example |
|-------------|-------------|---------|
| `SERVER1_HOST` | IP address of Server 1 | `188.245.97.41` |
| `SERVER1_USER` | SSH username for Server 1 | `root` |
| `SERVER1_SSH_KEY` | Private SSH key for Server 1 | `-----BEGIN OPENSSH PRIVATE KEY-----...` |
| `SERVER2_HOST` | IP address of Server 2 | `135.181.145.174` |
| `SERVER2_USER` | SSH username for Server 2 | `root` |
| `SERVER2_SSH_KEY` | Private SSH key for Server 2 | `-----BEGIN OPENSSH PRIVATE KEY-----...` |

## Workflow Triggers

The deployment workflow runs automatically:

1. **On Push to Main**: Every commit to the `main` branch triggers deployment
2. **Manual Trigger**: Use the "Run workflow" button in GitHub Actions

## Deployment Process

### 1. Image Building
- Builds `ippan-node` image using `Dockerfile.production`
- Builds `ippan-gateway` image using `apps/gateway/Dockerfile`
- Pushes images to `ghcr.io/dmrl789/ippan/`

### 2. Server Deployment
- **Server 1**: Uses `deploy/docker-compose.full-stack.yml`
- **Server 2**: Uses `deploy/docker-compose.production.yml`
- Pulls latest images and restarts services
- Cleans up old Docker images

### 3. Health Verification
- Checks node health endpoints (`/health`)
- Verifies gateway health (if running)
- Tests P2P connectivity between servers

## Docker Images

The workflow builds and uses these images:

- `ghcr.io/dmrl789/ippan/ippan-node:latest` - Blockchain node
- `ghcr.io/dmrl789/ippan/ippan-gateway:latest` - API gateway

## Manual Deployment

If you need to deploy manually:

### Server 1 (Full Stack)
```bash
ssh root@188.245.97.41
cd /opt/ippan
git pull origin main
docker compose -f deploy/docker-compose.full-stack.yml pull
docker compose -f deploy/docker-compose.full-stack.yml up -d
```

### Server 2 (Node Only)
```bash
ssh root@135.181.145.174
cd /opt/ippan
git pull origin main
docker compose -f deploy/docker-compose.production.yml pull
docker compose -f deploy/docker-compose.production.yml up -d
```

## Monitoring and Troubleshooting

### Check Service Status
```bash
# View running containers
docker ps

# Check service logs
docker compose logs -f ippan-node-1
docker compose logs -f gateway

# Check health endpoints
curl http://localhost:8080/health
curl http://localhost:8081/health
```

### Health Check Endpoints

- **Node 1**: `http://188.245.97.41:8080/health`
- **Node 2**: `http://188.245.97.41:8081/health` (Server 1) or `http://135.181.145.174:8080/health` (Server 2)
- **Gateway**: `http://188.245.97.41:8081/health`

### P2P Connectivity

The nodes communicate via P2P on these ports:
- **Server 1 Node 1**: Port 9000
- **Server 1 Node 2**: Port 9001
- **Server 2 Node 1**: Port 9000
- **Server 2 Node 2**: Port 9001

### Common Issues

#### Deployment Fails
1. Check GitHub Secrets are configured correctly
2. Verify SSH keys have proper permissions
3. Ensure servers have Docker and Docker Compose installed

#### Health Checks Fail
1. Check if services are running: `docker ps`
2. Review service logs: `docker compose logs [service-name]`
3. Verify port availability: `netstat -tlnp | grep [port]`

#### P2P Connectivity Issues
1. Check firewall rules: `ufw status`
2. Verify P2P ports are open: `telnet [host] [port]`
3. Review node configuration and bootnode settings

## Environment Variables

### Node Configuration
- `NODE_ID`: Unique identifier for the node
- `VALIDATOR_ID`: Validator identifier
- `RPC_HOST`: RPC server host (default: 0.0.0.0)
- `RPC_PORT`: RPC server port (default: 8080)
- `P2P_HOST`: P2P host (default: 0.0.0.0)
- `P2P_PORT`: P2P port
- `P2P_BOOTNODES`: Comma-separated list of bootnode addresses
- `LOG_LEVEL`: Logging level (info, debug, warn, error)

### Gateway Configuration
- `PORT`: Gateway server port (default: 8081)
- `TARGET_RPC_URL`: Backend RPC URL
- `TARGET_WS_URL`: Backend WebSocket URL
- `ALLOWED_ORIGINS`: CORS allowed origins
- `API_PREFIX`: API route prefix (default: /api)
- `WS_PREFIX`: WebSocket route prefix (default: /ws)

## Security Considerations

1. **SSH Keys**: Use dedicated SSH keys for deployment
2. **Firewall**: Only expose necessary ports
3. **Updates**: Keep Docker images and host systems updated
4. **Monitoring**: Monitor logs for suspicious activity

## Rollback Procedure

If a deployment fails:

1. **Automatic**: The workflow includes health checks that will fail the deployment
2. **Manual Rollback**:
   ```bash
   # On affected server
   docker compose down
   docker pull ghcr.io/dmrl789/ippan/ippan-node:[previous-tag]
   docker compose up -d
   ```

## Performance Monitoring

Monitor these metrics:
- Container resource usage: `docker stats`
- Node synchronization status
- P2P peer count
- Transaction throughput
- Block production rate

## Support

For deployment issues:
1. Check GitHub Actions logs
2. Review server logs via SSH
3. Verify network connectivity
4. Check Docker and system resources

## Next Steps

Consider implementing:
- **Staging Environment**: Add staging deployment workflow
- **Blue-Green Deployment**: Zero-downtime deployment strategy
- **Monitoring Stack**: Prometheus, Grafana, and alerting
- **Backup Strategy**: Automated blockchain data backups