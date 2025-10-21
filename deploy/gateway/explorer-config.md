# IPPAN Blockchain Explorer Configuration

## Overview
This configuration enables a full blockchain explorer for IPPAN at `ui.ippan.org/explorer` with the following features:

### Available Blockchain Data Endpoints

#### Core Blockchain Data
- `GET /api/health` - Node health and status
- `GET /api/version` - Node version information
- `GET /api/time` - Current blockchain time
- `GET /api/metrics` - Prometheus metrics

#### Block Data
- `GET /api/block/{id}` - Get block by height or hash
  - Example: `/api/block/1` (by height)
  - Example: `/api/block/0x1234...` (by hash)

#### Transaction Data
- `GET /api/tx/{hash}` - Get transaction by hash
- `POST /api/tx` - Submit new transaction

#### Account Data
- `GET /api/account/{address}` - Get account information

#### Network Data
- `GET /api/peers` - List connected peers
- `GET /api/l2/config` - L2 configuration
- `GET /api/l2/networks` - List L2 networks
- `GET /api/l2/commits` - List L2 commits
- `GET /api/l2/exits` - List L2 exits

#### WebSocket Support
- `WS /ws` - Real-time blockchain updates

## Configuration Files

### 1. Docker Compose (`docker-compose.yml`)
- **ippan-node**: Blockchain node with RPC API on port 8080
- **gateway**: API gateway with CORS support on port 8081
- **unified-ui**: Frontend UI on port 3000
- **nginx**: Reverse proxy routing requests

### 2. Nginx Configuration (`nginx.conf`)
- Routes `/api/*` to gateway service
- Routes `/ws` to gateway WebSocket
- Routes `/` to unified-ui
- CORS headers for cross-origin requests

### 3. Environment Variables (`.env`)
- Gateway configuration for API routing
- CORS origins for ui.ippan.org
- Node configuration for blockchain

## Deployment Steps

1. **Deploy the fixed configuration:**
   ```bash
   cd deploy/gateway
   ./fix-gateway.sh
   ```

2. **Verify the deployment:**
   ```bash
   # Check container status
   docker compose ps
   
   # Test API endpoints
   curl http://ui.ippan.org/api/health
   curl http://ui.ippan.org/api/version
   curl http://ui.ippan.org/api/peers
   ```

3. **Access the explorer:**
   - Main UI: http://ui.ippan.org/
   - API Health: http://ui.ippan.org/api/health
   - Blockchain Data: http://ui.ippan.org/api/block/1

## Explorer Features

### Real-time Data
- WebSocket connection for live updates
- Block height monitoring
- Transaction status tracking
- Peer connection status

### Historical Data
- Block browser with pagination
- Transaction history
- Account balance tracking
- L2 network activity

### Network Monitoring
- Peer count and status
- Consensus metrics
- Mempool size
- Network health indicators

## Troubleshooting

### Common Issues

1. **API endpoints return 404:**
   - Check if gateway container is running
   - Verify nginx configuration
   - Check gateway logs: `docker compose logs gateway`

2. **CORS errors:**
   - Verify ALLOWED_ORIGINS in .env
   - Check nginx CORS headers

3. **WebSocket connection fails:**
   - Verify WS_PREFIX configuration
   - Check nginx WebSocket proxy settings

4. **No blockchain data:**
   - Check if ippan-node is running
   - Verify RPC connection
   - Check node logs: `docker compose logs ippan-node`

### Health Checks

```bash
# Container status
docker compose ps

# API health
curl -s http://ui.ippan.org/api/health | jq

# Node health
curl -s http://ui.ippan.org/api/version | jq

# WebSocket test
curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Sec-WebSocket-Key: SGVsbG8sIHdvcmxkIQ==" -H "Sec-WebSocket-Version: 13" http://ui.ippan.org/ws
```

## Security Considerations

- CORS is configured for ui.ippan.org
- API endpoints are rate-limited
- WebSocket connections are properly proxied
- No sensitive data exposed in logs

## Performance

- Nginx reverse proxy for load balancing
- Docker container isolation
- Efficient API routing
- WebSocket connection pooling