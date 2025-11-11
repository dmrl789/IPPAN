# Secrets and Environment Variables Documentation

This document provides a comprehensive reference for all secrets and environment variables used in the IPPAN project.

## Table of Contents

- [GitHub Secrets (CI/CD)](#github-secrets-cicd)
- [Node Environment Variables](#node-environment-variables)
- [Gateway Environment Variables](#gateway-environment-variables)
- [UI Environment Variables](#ui-environment-variables)
- [AI Service Environment Variables](#ai-service-environment-variables)
- [Deployment Environment Variables](#deployment-environment-variables)
- [How to Configure](#how-to-configure)
- [Security Best Practices](#security-best-practices)

---

## GitHub Secrets (CI/CD)

These secrets must be configured in GitHub repository settings under **Settings → Secrets and variables → Actions**.

### Required Secrets

| Secret Name | Description | Required | Example/Format |
|-------------|-------------|----------|----------------|
| `GITHUB_TOKEN` | Auto-provided by GitHub Actions | Yes (auto) | `ghp_xxxxxxxxxxxx` |
| `DEPLOY_SSH_KEY` | SSH private key for deployment servers | Yes | `-----BEGIN OPENSSH PRIVATE KEY-----\n...` |
| `SERVER1_HOST` | Primary server hostname/IP | Yes | `188.245.97.41` |
| `SERVER1_USER` | SSH username for Server 1 | Yes | `root` or `ubuntu` |
| `SERVER2_HOST` | Secondary server hostname/IP | Yes | `135.181.145.174` |
| `SERVER2_USER` | SSH username for Server 2 | Yes | `root` or `ubuntu` |

### Optional Secrets

| Secret Name | Description | Default | Example |
|-------------|-------------|---------|---------|
| `DEPLOY_HOST` | Fallback deployment host | N/A | `188.245.97.41` |
| `DEPLOY_USER` | Fallback SSH username | N/A | `ubuntu` |
| `DEPLOY_PORT` | Default SSH port | `22` | `22` |
| `DEPLOY_FINGERPRINT` | SSH fingerprint for host verification | N/A | `SHA256:xxx...` |
| `SERVER1_PORT` | SSH port for Server 1 | `22` | `2222` |
| `SERVER1_SSH_KEY` | Server 1 specific SSH key (overrides DEPLOY_SSH_KEY) | Uses DEPLOY_SSH_KEY | `-----BEGIN...` |
| `SERVER1_FINGERPRINT` | SSH fingerprint for Server 1 | N/A | `SHA256:xxx...` |
| `SERVER2_PORT` | SSH port for Server 2 | `22` | `2222` |
| `SERVER2_SSH_KEY` | Server 2 specific SSH key (overrides DEPLOY_SSH_KEY) | Uses DEPLOY_SSH_KEY | `-----BEGIN...` |
| `SECONDARY_HOST` | Alternative name for SERVER2_HOST | N/A | `135.181.145.174` |
| `SECONDARY_PORT` | Alternative for SERVER2_PORT | `22` | `2222` |
| `SECONDARY_FINGERPRINT` | SSH fingerprint for Server 2 | N/A | `SHA256:xxx...` |
| `NVD_API_KEY` | NIST National Vulnerability Database API key for security scans | N/A | `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` |
| `GH_TOKEN` | Alternative to GITHUB_TOKEN for gh CLI | Uses GITHUB_TOKEN | `ghp_xxxxxxxxxxxx` |

---

## Node Environment Variables

Configuration for IPPAN blockchain nodes.

### Identity & Network

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `NODE_ID` | Unique node identifier | Yes | N/A | `ippan_node_001` |
| `VALIDATOR_ID` | 64-character hex validator ID | Yes | N/A | `0123456789abcdef...` |
| `RPC_HOST` | RPC server bind address | No | `0.0.0.0` | `0.0.0.0` |
| `RPC_PORT` | RPC server port | No | `8080` | `8080` |
| `P2P_HOST` | P2P network bind address | No | `0.0.0.0` | `0.0.0.0` |
| `P2P_PORT` | P2P network port | No | `9000` | `9000` |

### Storage

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `DATA_DIR` | Base data directory | No | `/var/lib/ippan` | `/var/lib/ippan` |
| `DB_PATH` | Database file path | No | `/var/lib/ippan/db` | `/var/lib/ippan/db` |

### Consensus

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `SLOT_DURATION_MS` | Block slot duration in milliseconds | No | `1000` | `1000` |
| `MAX_TRANSACTIONS_PER_BLOCK` | Maximum transactions per block | No | `1000` | `1000` |
| `BLOCK_REWARD` | Block mining reward | No | `10` | `10` |
| `FINALIZATION_INTERVAL_MS` | Block finalization interval | No | `200` | `200` |

### P2P Configuration

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `BOOTSTRAP_NODES` | Comma-separated bootstrap node URLs | No | Empty | `http://188.245.97.41:9000,http://135.181.145.174:9000` |
| `MAX_PEERS` | Maximum peer connections | No | `50` | `50` |
| `P2P_ENABLE_UPNP` | Enable UPnP for NAT traversal | No | `false` | `true` |
| `P2P_PUBLIC_HOST` | Public hostname/IP for NAT | No | Empty | `example.com` |
| `P2P_EXTERNAL_IP_SERVICES` | External IP detection services | No | `https://api.ipify.org,...` | `https://api.ipify.org` |
| `P2P_DISCOVERY_INTERVAL_SECS` | Peer discovery interval | No | `30` | `30` |
| `P2P_ANNOUNCE_INTERVAL_SECS` | Peer announcement interval | No | `60` | `60` |
| `PEER_DISCOVERY_INTERVAL_SECS` | Legacy peer discovery interval | No | `30` | `30` |
| `PEER_ANNOUNCE_INTERVAL_SECS` | Legacy peer announcement interval | No | `60` | `60` |
| `BLOCK_TOPIC` | Gossipsub block topic | No | `ippan-blocks` | `ippan-blocks` |
| `TX_TOPIC` | Gossipsub transaction topic | No | `ippan-transactions` | `ippan-transactions` |

### Layer 2 Configuration

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `L2_MAX_COMMIT_SIZE` | Maximum L2 commit size | No | `16384` | `16384` |
| `L2_MIN_EPOCH_GAP_MS` | Minimum epoch gap in ms | No | `250` | `250` |
| `L2_CHALLENGE_WINDOW_MS` | Challenge window in ms | No | `60000` | `60000` |
| `L2_DA_MODE` | Data availability mode | No | `external` | `external` |
| `L2_MAX_L2_COUNT` | Maximum L2 count | No | `100` | `100` |

### Logging

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `LOG_LEVEL` | Logging level | No | `info` | `debug`, `info`, `warn`, `error` |
| `LOG_FORMAT` | Log output format | No | `json` | `json`, `pretty` |

### Security

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `ENABLE_TLS` | Enable TLS for RPC | No | `false` | `true` |
| `TLS_CERT_PATH` | TLS certificate path | No | Empty | `/etc/ssl/certs/cert.pem` |
| `TLS_KEY_PATH` | TLS private key path | No | Empty | `/etc/ssl/private/key.pem` |
| `ENABLE_SECURITY` | Enable security features | No | `true` | `false` (dev only) |

### Development & Monitoring

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `DEV_MODE` | Enable development mode | No | `false` | `true` |
| `ENABLE_METRICS` | Enable metrics collection | No | `true` | `true` |
| `METRICS_PORT` | Metrics server port | No | `9090` | `9090` |

### Performance

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `MAX_CONNECTIONS` | Maximum connections | No | `100` | `100` |
| `CONNECTION_TIMEOUT_MS` | Connection timeout | No | `30000` | `30000` |
| `REQUEST_TIMEOUT_MS` | Request timeout | No | `10000` | `10000` |

---

## Gateway Environment Variables

Configuration for the IPPAN API Gateway.

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `PORT` | Gateway server port | No | `8080` | `7080` |
| `TARGET_RPC_URL` | Backend node RPC URL | Yes | N/A | `http://localhost:8080` |
| `TARGET_WS_URL` | Backend node WebSocket URL | Yes | N/A | `ws://localhost:8080/ws` |
| `TARGET_HEALTH_PATH` | Health check endpoint path | No | `/health` | `/health` |
| `HEALTH_TIMEOUT_MS` | Health check timeout | No | `5000` | `5000` |
| `API_PREFIX` | API route prefix | No | `/api` | `/api` |
| `WS_PREFIX` | WebSocket route prefix | No | `/ws` | `/ws` |
| `ENABLE_EXPLORER` | Enable explorer API | No | `true` | `true` |
| `EXPLORER_PREFIX` | Explorer API prefix | No | `/explorer/api` | `/explorer/api` |
| `ALLOWED_ORIGINS` | CORS allowed origins | No | `*` | `https://ippan.com,https://ippan.net` |
| `JSON_BODY_LIMIT` | Maximum JSON body size | No | `2mb` | `5mb` |
| `PROXY_LOG_LEVEL` | Proxy log level | No | `warn` | `info`, `warn`, `error` |
| `NODE_ENV` | Node environment | No | `production` | `development`, `production` |

---

## UI Environment Variables

Configuration for the IPPAN Unified UI (Next.js).

### Public Variables (Client-side)

These variables are exposed to the browser and should NOT contain secrets.

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `NEXT_PUBLIC_ENABLE_FULL_UI` | Enable full UI features | No | `1` | `1` or `0` |
| `NEXT_PUBLIC_NETWORK_NAME` | Network display name | No | `IPPAN-Devnet` | `IPPAN Production` |
| `NEXT_PUBLIC_GATEWAY_URL` | Gateway API URL | Yes | N/A | `http://localhost:8081/api` |
| `NEXT_PUBLIC_API_BASE_URL` | API base URL | Yes | N/A | `http://localhost:7080` |
| `NEXT_PUBLIC_WS_URL` | WebSocket URL | Yes | N/A | `ws://localhost:7080/ws` |
| `NEXT_PUBLIC_AI_ENABLED` | Enable AI features | No | `1` | `1` or `0` |
| `NEXT_PUBLIC_ASSET_PREFIX` | CDN/subdirectory prefix | No | Empty | `/static` |

### Server Variables

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `NODE_ENV` | Node environment | No | `production` | `development`, `production` |
| `PORT` | UI server port | No | `3001` | `3000` |
| `ENABLE_FULL_UI` | Server-side full UI toggle | No | `1` | `1` or `0` |

---

## AI Service Environment Variables

Configuration for AI services (AI Core, LLM, Analytics).

### AI Core

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `AI_CORE_HEALTH_ENABLED` | Enable health monitoring | No | `true` | `true`, `false` |
| `AI_CORE_HEALTH_MEMORY_THRESHOLD` | Memory threshold (bytes) | No | `1000000000` | `1073741824` (1GB) |
| `AI_CORE_EXECUTION_MAX_TIME` | Max execution time (seconds) | No | `30` | `60` |
| `AI_CORE_EXECUTION_MAX_MEMORY` | Max memory usage (bytes) | No | `100000000` | `104857600` (100MB) |
| `AI_CORE_LOG_LEVEL` | Logging level | No | `info` | `debug`, `info`, `warn` |
| `AI_CORE_LOG_FILE` | Log file path | No | Empty | `/var/log/ai-core.log` |
| `AI_CORE_SECURITY_RATE_LIMIT` | Rate limit (req/sec) | No | `1000` | `5000` |
| `AI_CORE_PERFORMANCE_METRICS` | Enable metrics | No | `true` | `true`, `false` |

### LLM Service

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `ENABLE_LLM` | Enable LLM features | No | `true` | `true`, `false` |
| `LLM_API_ENDPOINT` | LLM API endpoint | No | `https://api.openai.com/v1` | `https://api.openai.com/v1` |
| `LLM_API_KEY` | LLM API key | **Yes (if LLM enabled)** | N/A | `sk-...` |
| `LLM_MODEL` | LLM model name | No | `gpt-4` | `gpt-4`, `gpt-3.5-turbo` |
| `LLM_MAX_TOKENS` | Maximum tokens per request | No | `4000` | `4000` |
| `LLM_TEMPERATURE` | Temperature (0.0-1.0) | No | `0.7` | `0.7` |
| `LLM_TIMEOUT` | Request timeout (seconds) | No | `30` | `30` |

### Analytics

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `ENABLE_ANALYTICS` | Enable analytics | No | `true` | `true`, `false` |
| `ANALYTICS_REALTIME` | Enable real-time analytics | No | `true` | `true`, `false` |
| `ANALYTICS_RETENTION_DAYS` | Data retention period | No | `30` | `90` |
| `ANALYTICS_INTERVAL` | Analysis interval (seconds) | No | `60` | `60` |
| `ANALYTICS_PREDICTIVE` | Enable predictive analytics | No | `true` | `true`, `false` |

### General AI Service

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `ENABLE_SMART_CONTRACTS` | Enable smart contract features | No | `true` | `true`, `false` |
| `ENABLE_MONITORING` | Enable service monitoring | No | `true` | `true`, `false` |
| `IPPAN_ENV` | Environment name | No | `development` | `development`, `staging`, `production` |
| `ENVIRONMENT` | Alternative env name | No | `development` | `development`, `staging`, `production` |

### Secrets (Prefix Pattern)

Variables prefixed with `IPPAN_SECRET_*` are automatically loaded as secrets:

| Variable | Description | Example |
|----------|-------------|---------|
| `IPPAN_SECRET_API_KEY` | API key secret | Any sensitive value |
| `IPPAN_SECRET_DB_PASSWORD` | Database password | Any sensitive value |
| `IPPAN_SECRET_*` | Any custom secret | Any sensitive value |

---

## Deployment Environment Variables

Global deployment configuration.

| Variable | Description | Required | Default | Example |
|----------|-------------|----------|---------|---------|
| `NETWORK_NAME` | Network name | No | `IPPAN Production` | `IPPAN Production` |
| `CHAIN_ID` | Chain identifier | No | `ippan-mainnet` | `ippan-mainnet` |
| `UI_PORT` | UI service port | No | `3001` | `3001` |
| `RPC_PORT` | RPC service port | No | `8080` | `8080` |
| `P2P_PORT` | P2P service port | No | `4001` | `4001` |
| `GATEWAY_PORT` | Gateway service port | No | `8081` | `8081` |
| `NODE_1_HOST` | Node 1 hostname | No | `188.245.97.41` | `188.245.97.41` |
| `NODE_2_HOST` | Node 2 hostname | No | `135.181.145.174` | `135.181.145.174` |
| `IPPAN_UI_TAG` | UI Docker image tag | No | `latest` | `v1.0.0` |
| `IPPAN_NODE_TAG` | Node Docker image tag | No | `latest` | `v1.0.0` |
| `IPPAN_GATEWAY_TAG` | Gateway Docker image tag | No | `latest` | `v1.0.0` |
| `IMAGE_TAG` | Global Docker image tag | No | `latest` | `main-abc1234` |
| `SSL_CERT_PATH` | SSL certificate path | No | Empty | `/etc/ssl/certs/cert.pem` |
| `SSL_KEY_PATH` | SSL key path | No | Empty | `/etc/ssl/private/key.pem` |
| `DEPLOY_APP_DIR` | Application deployment directory | No | `$HOME/ippan-deploy` | `/opt/ippan` |

---

## How to Configure

### Local Development

1. Copy the example environment files:
   ```bash
   cp config/ippan.env.example config/ippan.env
   cp apps/unified-ui/.env.example apps/unified-ui/.env.local
   cp apps/gateway/.env.example apps/gateway/.env.local
   cp deploy/.env.example deploy/.env
   ```

2. Edit the files with your local configuration:
   ```bash
   nano config/ippan.env
   nano apps/unified-ui/.env.local
   nano apps/gateway/.env.local
   ```

### Production Deployment

1. **Set GitHub Secrets** (Required for CI/CD):
   - Go to: `https://github.com/<owner>/<repo>/settings/secrets/actions`
   - Click "New repository secret"
   - Add all required secrets from the [GitHub Secrets](#github-secrets-cicd) section

2. **Server Environment Files**:
   
   On Server 1 (Full Stack):
   ```bash
   cd /opt/ippan
   cp deploy/.env.example .env
   # Edit with production values
   nano .env
   ```

   On Server 2 (Node Only):
   ```bash
   cd /opt/ippan
   cp config/ippan.env.example config/ippan.env
   # Edit with production values
   nano config/ippan.env
   ```

3. **Docker Compose**:
   Environment variables are automatically loaded from:
   - `.env` file in the same directory as `docker-compose.yml`
   - Environment variables passed via `-e` flag
   - Variables defined in the `docker-compose.yml` itself

### Environment-Specific Configurations

The AI service supports environment-specific config files:

- Development: `config/development.toml`
- Staging: `config/staging.toml`
- Production: `config/production.toml`
- Testing: `config/testing.toml`

Set `IPPAN_ENV` or `ENVIRONMENT` to select the configuration:

```bash
export IPPAN_ENV=production
```

---

## Security Best Practices

### 1. Never Commit Secrets
- Add `.env` files to `.gitignore`
- Use `.env.example` files as templates (with placeholder values)
- Never commit real API keys, passwords, or SSH keys

### 2. Use Strong SSH Keys
```bash
# Generate a secure SSH key for deployment
ssh-keygen -t ed25519 -C "ippan-deployment" -f ~/.ssh/ippan_deploy_key
```

### 3. Rotate Secrets Regularly
- API keys should be rotated every 90 days
- SSH keys should be rotated annually
- Document rotation schedule in your ops runbook

### 4. Principle of Least Privilege
- Use separate SSH keys per server if possible
- Limit SSH user permissions to only what's needed
- Use non-root users when possible

### 5. Secret Storage
- Use GitHub Secrets for CI/CD
- For production servers, consider:
  - HashiCorp Vault
  - AWS Secrets Manager
  - Docker Secrets (if using Docker Swarm)
  - Kubernetes Secrets (if using K8s)

### 6. Validate Environment Variables
```bash
# Check if required variables are set before starting services
[ -z "$LLM_API_KEY" ] && echo "ERROR: LLM_API_KEY is not set" && exit 1
```

### 7. Use HTTPS/TLS in Production
- Always use HTTPS for API endpoints
- Enable TLS for inter-node communication
- Use valid SSL certificates (not self-signed) in production

### 8. Monitor Access
- Enable audit logging for secret access
- Monitor SSH login attempts
- Set up alerts for unauthorized access attempts

### 9. Secure Transmission
- Use encrypted channels (SSH, HTTPS) for secret transmission
- Never send secrets via email or chat
- Use secure password managers for team secret sharing

### 10. Environment Separation
- Use different secrets for dev/staging/production
- Never use production secrets in development
- Isolate test environments completely

---

## Troubleshooting

### Missing Environment Variables

If services fail to start due to missing variables:

1. Check which variables are missing:
   ```bash
   docker-compose config
   ```

2. Verify `.env` file exists and is readable:
   ```bash
   ls -la .env
   cat .env
   ```

3. Check container environment:
   ```bash
   docker exec <container-name> env | grep IPPAN
   ```

### GitHub Actions Failing

If deployment workflows fail due to missing secrets:

1. Verify secrets are set in GitHub:
   ```bash
   gh secret list
   ```

2. Check workflow logs for specific missing variables

3. Ensure secret names match exactly (case-sensitive)

### Connection Issues

If nodes can't connect to each other:

1. Verify `BOOTSTRAP_NODES` is correctly set
2. Check firewall rules for P2P ports
3. Verify `P2P_PUBLIC_HOST` if behind NAT
4. Enable `P2P_ENABLE_UPNP=true` for automatic NAT traversal

---

## Quick Reference Card

### Minimal Required Variables for Local Dev

```bash
# Node
NODE_ID=dev_node_1
VALIDATOR_ID=0000000000000000000000000000000000000000000000000000000000000001
BOOTSTRAP_NODES=

# Gateway
TARGET_RPC_URL=http://localhost:8080
TARGET_WS_URL=ws://localhost:8080/ws

# UI
NEXT_PUBLIC_GATEWAY_URL=http://localhost:7080/api
NEXT_PUBLIC_API_BASE_URL=http://localhost:7080
NEXT_PUBLIC_WS_URL=ws://localhost:7080/ws
```

### Minimal Required Secrets for Production CI/CD

```bash
DEPLOY_SSH_KEY=<ssh-private-key>
SERVER1_HOST=188.245.97.41
SERVER1_USER=root
SERVER2_HOST=135.181.145.174
SERVER2_USER=root
```

---

## Related Documentation

- [Deployment Guide](../DEPLOYMENT_INSTRUCTIONS.md)
- [Full Stack Deployment](../FULL_STACK_DEPLOYMENT_GUIDE.md)
- [Explorer Deployment](../EXPLORER_DEPLOYMENT_GUIDE.md)
- [Contributing Guide](../CONTRIBUTING.md)

---

**Last Updated**: 2025-11-11  
**Maintained By**: Agent-Theta, MetaAgent  
**Review Cycle**: Quarterly or on major changes
