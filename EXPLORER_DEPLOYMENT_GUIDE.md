# Blockchain Explorer Deployment Guide

## Overview

This guide explains how to deploy and configure the IPPAN blockchain explorer for `ippan.com/explorer` and `explorer.ippan.network`.

## Changes Made

### 1. Gateway Service (`apps/gateway/src/server.mjs`)

**Added explorer routing capabilities:**
- New environment variables: `ENABLE_EXPLORER`, `EXPLORER_PREFIX`
- Explorer API proxy at `/explorer/api/*` routes to blockchain RPC
- Explorer info endpoint at `/explorer` returns available API endpoints
- Automatic CORS handling for cross-origin requests

### 2. Kubernetes Ingress (`deployments/kubernetes/ippan-ingress.yaml`)

**Fixed broken service references:**
- Changed `ippan-explorer:3001` → `ippan-api:3000` (gateway service)
- Routes `explorer.ippan.network` to gateway
- Routes `ippan.com/explorer` to gateway

### 3. Nginx Configuration (`deployments/mainnet/nginx/nginx.conf`)

**Added explorer support:**
- New server block for `explorer.ippan.net`
- Added `/explorer` location to main website server
- CORS headers for public blockchain data access
- SSL/TLS termination for explorer subdomains

## Environment Variables

### Gateway Service

Add to `.env` or deployment config:

```bash
# Enable blockchain explorer (default: true)
ENABLE_EXPLORER=true

# Explorer API path prefix (default: /explorer/api)
EXPLORER_PREFIX=/explorer/api

# Target blockchain RPC URL (existing)
TARGET_RPC_URL=http://ippan-node:8080

# CORS allowed origins (existing - add explorer domains)
ALLOWED_ORIGINS=https://ippan.com,https://explorer.ippan.network,https://ippan.net
```

### Node Service

Update CORS in node configs (`deployments/mainnet/configs/*.toml`):

```toml
[api]
cors_origins = [
    "https://ippan.net",
    "https://ippan.com",
    "https://explorer.ippan.net",
    "https://explorer.ippan.network",
]
```

## Deployment Steps

### Step 1: Update Gateway

1. **Build new gateway image:**
   ```bash
   cd apps/gateway
   docker build -t ghcr.io/ippan/gateway:latest .
   docker push ghcr.io/ippan/gateway:latest
   ```

2. **Update environment variables:**
   ```bash
   kubectl create configmap gateway-config \
     --from-literal=ENABLE_EXPLORER=true \
     --from-literal=EXPLORER_PREFIX=/explorer/api \
     --from-literal=ALLOWED_ORIGINS=https://ippan.com,https://explorer.ippan.network \
     -n ippan \
     --dry-run=client -o yaml | kubectl apply -f -
   ```

3. **Restart gateway pods:**
   ```bash
   kubectl rollout restart deployment/ippan-gateway -n ippan
   ```

### Step 2: Update Kubernetes Ingress

```bash
kubectl apply -f deployments/kubernetes/ippan-ingress.yaml
```

Verify ingress update:
```bash
kubectl get ingress ippan-ingress -n ippan -o yaml
```

### Step 3: Update Nginx (if using mainnet nginx)

**IMPORTANT**: The nginx configuration does NOT include CORS headers. CORS is handled entirely by the gateway service to ensure proper origin whitelisting and security.

1. **Copy new nginx config:**
   ```bash
   scp deployments/mainnet/nginx/nginx.conf user@server:/etc/nginx/nginx.conf
   ```

2. **Test nginx configuration:**
   ```bash
   ssh user@server "sudo nginx -t"
   ```

3. **Reload nginx:**
   ```bash
   ssh user@server "sudo systemctl reload nginx"
   ```

### Step 4: Update Node CORS Configuration

1. **Update node config maps:**
   ```bash
   kubectl create configmap ippan-config \
     --from-file=deployments/mainnet/configs/bootstrap-1.toml \
     -n ippan \
     --dry-run=client -o yaml | kubectl apply -f -
   ```

2. **Restart nodes:**
   ```bash
   kubectl rollout restart deployment/ippan-node -n ippan
   ```

### Step 5: Verify Deployment

Run these checks to ensure everything is working:

```bash
# 1. Check gateway health
curl https://ippan.com/api/health

# 2. Check explorer info endpoint
curl https://ippan.com/explorer
curl https://explorer.ippan.network/explorer

# 3. Test explorer API endpoints
curl https://ippan.com/explorer/api/health
curl https://ippan.com/explorer/api/time
curl https://ippan.com/explorer/api/block/1

# 4. Test subdomain
curl https://explorer.ippan.network/explorer/api/health

# 5. Verify CORS headers
curl -I -X OPTIONS https://ippan.com/explorer/api/health \
  -H "Origin: https://ippan.com" \
  -H "Access-Control-Request-Method: GET"
```

## Available Explorer API Endpoints

All endpoints accessible at `/explorer/api/*`:

### Node Status
- `GET /explorer/api/health` - Node health check
- `GET /explorer/api/time` - IPPAN time (microseconds)
- `GET /explorer/api/version` - Node version
- `GET /explorer/api/metrics` - Prometheus metrics
- `GET /explorer/api/peers` - Connected peers

### Blockchain Data
- `GET /explorer/api/block/:id` - Get block by height/ID
- `GET /explorer/api/tx/:hash` - Get transaction by hash
- `GET /explorer/api/account/:address` - Get account info
- `POST /explorer/api/tx` - Submit new transaction

### Layer 2
- `GET /explorer/api/l2/config` - L2 configuration
- `GET /explorer/api/l2/networks` - List L2 networks
- `GET /explorer/api/l2/commits` - All L2 commits
- `GET /explorer/api/l2/commits/:l2_id` - L2 commits for specific network
- `GET /explorer/api/l2/exits` - All L2 exit records
- `GET /explorer/api/l2/exits/:l2_id` - L2 exits for specific network

## URL Mappings

| Public URL | Route | Backend Service | Notes |
|------------|-------|-----------------|-------|
| `https://ippan.com/explorer` | Gateway `/explorer` | Info endpoint | Returns API documentation |
| `https://ippan.com/explorer/api/*` | Gateway → Node RPC | Blockchain API | Proxied to node |
| `https://explorer.ippan.network/explorer` | Gateway `/explorer` | Info endpoint | Same as ippan.com |
| `https://explorer.ippan.network/explorer/api/*` | Gateway → Node RPC | Blockchain API | Proxied to node |

## Troubleshooting

### Explorer returns 404

**Check 1: Gateway running?**
```bash
kubectl get pods -n ippan -l app=ippan-gateway
kubectl logs -n ippan -l app=ippan-gateway --tail=50
```

**Check 2: Environment variables set?**
```bash
kubectl exec -n ippan deployment/ippan-gateway -- env | grep EXPLORER
```

**Check 3: Ingress routing correct?**
```bash
kubectl describe ingress ippan-ingress -n ippan
```

### CORS errors in browser

**Check 1: ALLOWED_ORIGINS includes your domain**
```bash
kubectl exec -n ippan deployment/ippan-gateway -- env | grep ALLOWED_ORIGINS
```

**Check 2: Node CORS config**
```bash
kubectl exec -n ippan deployment/ippan-node -- cat /config/default.toml | grep cors
```

**Check 3: Response headers**
```bash
curl -I https://ippan.com/explorer/api/health -H "Origin: https://ippan.com"
```

### 502 Bad Gateway

**Check 1: Node service healthy**
```bash
kubectl get svc -n ippan ippan-node-service
kubectl get endpoints -n ippan ippan-node-service
```

**Check 2: Target RPC URL correct**
```bash
kubectl exec -n ippan deployment/ippan-gateway -- env | grep TARGET_RPC_URL
```

**Check 3: Node API responding**
```bash
kubectl exec -n ippan deployment/ippan-gateway -- wget -O- http://ippan-node-service:8080/health
```

### Slow responses

**Check 1: Gateway timeout settings**
```bash
kubectl exec -n ippan deployment/ippan-gateway -- env | grep TIMEOUT
```

**Check 2: Nginx timeout settings**
```bash
ssh user@server "sudo nginx -T | grep timeout"
```

**Check 3: Node performance**
```bash
curl https://ippan.com/explorer/api/metrics | grep ippan_request_duration
```

## SSL Certificate Setup

### For explorer.ippan.net

If using Let's Encrypt with cert-manager (Kubernetes):

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: explorer-tls
  namespace: ippan
spec:
  secretName: explorer-tls
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  dnsNames:
  - explorer.ippan.net
  - explorer.ippan.network
```

If using standalone nginx with certbot:

```bash
sudo certbot certonly --nginx -d explorer.ippan.net -d explorer.ippan.network
sudo systemctl reload nginx
```

### For ippan.com

Update existing certificate to include explorer path or ensure `ippan.com` cert exists.

## Next Steps

### Phase 1: API Access (COMPLETE)
- ✅ Gateway routes explorer API requests
- ✅ Kubernetes ingress routing fixed
- ✅ Nginx configuration updated
- ✅ CORS properly configured

### Phase 2: Explorer UI (TODO)

Choose one of:

**Option A: Deploy BlockScout**
- Full-featured blockchain explorer
- Requires PostgreSQL database
- Docker deployment available

**Option B: Custom Next.js Explorer**
- Build lightweight explorer UI
- Integrate with unified UI
- Custom branding and features

**Option C: Third-party Explorer Integration**
- Use hosted explorer service
- Configure to use IPPAN API
- Less maintenance overhead

### Phase 3: Monitoring

Add monitoring for explorer traffic:
- Request rate metrics
- Error rate tracking
- Response time percentiles
- Popular endpoint usage

Example Prometheus queries:
```promql
# Explorer request rate
rate(http_requests_total{path=~"/explorer.*"}[5m])

# Explorer error rate
rate(http_requests_total{path=~"/explorer.*",status=~"5.."}[5m])

# Explorer response time
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket{path=~"/explorer.*"}[5m]))
```

## Security Considerations

1. **Rate Limiting**: Explorer endpoints have rate limits (configured in nginx)
2. **CORS**: Handled exclusively by the gateway service
   - ⚠️ **CRITICAL**: Nginx does NOT add CORS headers to avoid conflicts
   - Gateway respects `ALLOWED_ORIGINS` environment variable
   - No wildcard "*" origins in production
   - Prevents CORS header duplication that browsers reject
3. **SSL/TLS**: All explorer traffic uses HTTPS with TLS 1.2+
4. **Input Validation**: All API inputs validated by blockchain node
5. **DDoS Protection**: Nginx rate limiting and connection limits active

### CORS Architecture

```
Request → Nginx (no CORS headers) → Gateway (adds CORS) → Response
```

**Why this matters:**
- Nginx adding CORS headers would create duplicates with gateway headers
- Duplicate `Access-Control-Allow-Origin` headers cause browser errors
- Wildcard CORS in nginx would bypass gateway's origin whitelist
- Gateway CORS is configurable per deployment via environment variables

## Performance Tuning

### Gateway
- Adjust `HEALTH_TIMEOUT_MS` for slower nodes
- Increase `JSON_BODY_LIMIT` if large payloads expected
- Configure `PROXY_LOG_LEVEL=warn` in production

### Nginx
- Tune `limit_req` rate limits based on traffic
- Adjust `proxy_*_timeout` values for slow queries
- Enable caching for frequently accessed blocks/txs

### Node
- Optimize database queries for explorer endpoints
- Add read replicas for heavy explorer traffic
- Cache recent blocks and transactions

## Support

For issues or questions:
- GitHub Issues: https://github.com/dmrl789/IPPAN/issues
- Documentation: https://docs.ippan.com
- Contact: support@ippan.com
