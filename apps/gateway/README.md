# IPPAN Gateway

API Gateway and reverse proxy service for the IPPAN blockchain network. Routes HTTP/WebSocket requests to the blockchain node with CORS support and optional explorer endpoints.

## Features

- **API Proxying**: Routes `/api/*` requests to the blockchain node RPC
- **WebSocket Support**: Proxies `/ws` connections for real-time blockchain updates
- **Explorer API**: Optional `/explorer/api/*` endpoints for blockchain exploration
- **CORS Configuration**: Flexible origin whitelist for cross-origin requests
- **Health Checks**: Built-in health monitoring at `/health` endpoint
- **Path Rewriting**: Configurable prefix stripping for clean routing

## Quick Start

### Local Development

```bash
# Install dependencies
npm install

# Start development server with auto-reload
npm run dev

# Start production server
npm start
```

### Docker Build

```bash
# Build the image
docker build -t ippan-gateway .

# Run container
docker run -p 8080:8080 \
  -e TARGET_RPC_URL=http://node:8080 \
  -e ALLOWED_ORIGINS=* \
  ippan-gateway
```

## Configuration

All configuration is done via environment variables:

### Server Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | Port to listen on |
| `NODE_ENV` | `production` | Node environment |

### Upstream Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `TARGET_RPC_URL` | `http://node:8080` | Blockchain node RPC endpoint |
| `TARGET_WS_URL` | Auto-derived from `TARGET_RPC_URL` | WebSocket endpoint |
| `TARGET_HEALTH_PATH` | `/health` | Health check endpoint path |
| `HEALTH_TIMEOUT_MS` | `5000` | Health check timeout in milliseconds |

### Routing Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `API_PREFIX` | `/api` | Prefix for API routes (stripped before forwarding) |
| `WS_PREFIX` | `/ws` | Prefix for WebSocket routes |
| `EXPLORER_PREFIX` | `/explorer/api` | Prefix for explorer API routes |
| `ENABLE_EXPLORER` | `true` | Enable blockchain explorer endpoints |

### CORS Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `ALLOWED_ORIGINS` | `*` | Comma-separated list of allowed origins, or `*` for all |

### Limits & Logging

| Variable | Default | Description |
|----------|---------|-------------|
| `JSON_BODY_LIMIT` | `2mb` | Maximum JSON body size |
| `PROXY_LOG_LEVEL` | `warn` | Proxy logging level: `silent`, `error`, `warn`, `info`, `debug` |

## API Endpoints

### Gateway Endpoints

- `GET /health` - Gateway and upstream health status
- `GET /api/health` - Alias for health check
- `GET /api/health/node` - Direct node health check

### Proxied Blockchain Endpoints

All requests to `/api/*` are proxied to the blockchain node:

- `GET /api/block/:id` - Get block by height or hash
- `GET /api/tx/:hash` - Get transaction by hash
- `POST /api/tx` - Submit transaction
- `GET /api/account/:address` - Get account information
- `GET /api/peers` - List network peers
- `GET /api/time` - Current blockchain time
- `GET /api/version` - Node version
- `GET /api/metrics` - Prometheus metrics

### Explorer Endpoints

When `ENABLE_EXPLORER=true`, additional endpoints are available:

- `GET /explorer` - Explorer API info and endpoint list
- `GET /explorer/api/health` - Node health via explorer route
- `GET /explorer/api/block/:id` - Get block (explorer route)
- `GET /explorer/api/tx/:hash` - Get transaction (explorer route)
- `GET /explorer/api/account/:address` - Get account (explorer route)
- `GET /explorer/api/peers` - Network peers (explorer route)
- `GET /explorer/api/l2/config` - Layer 2 configuration
- `GET /explorer/api/l2/networks` - List L2 networks
- `GET /explorer/api/l2/commits` - List L2 commits
- `GET /explorer/api/l2/exits` - List L2 exits

### WebSocket Endpoint

- `WS /ws` - Real-time blockchain updates via WebSocket

## Docker Configuration

### Multi-stage Build

The Dockerfile uses a multi-stage build for optimization:

1. **deps stage**: Installs production dependencies
2. **runner stage**: Copies dependencies and application code

### Security Features

- Runs as non-root user (`nodejs:nodejs`)
- Minimal Alpine Linux base image
- Health check built into the image
- Only essential files copied to final image

### Health Check

The container includes a health check that runs every 30 seconds:

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD node -e "require('http').get('http://localhost:8080/health', (r) => { process.exit(r.statusCode === 200 ? 0 : 1); }).on('error', () => process.exit(1));"
```

## Deployment

### Docker Compose

See `/deploy/gateway/docker-compose.yml` for a complete deployment example with:

- IPPAN blockchain node
- Gateway service
- Unified UI
- Nginx reverse proxy

### Environment File

Create a `.env` file based on `.env.example`:

```bash
cp .env.example .env
# Edit .env with your configuration
```

### Production Deployment

```bash
cd /workspace/deploy/gateway
docker compose up -d
```

## Route Configuration Examples

### Example 1: Default Configuration

```bash
export API_PREFIX="/api"
export WS_PREFIX="/ws"
export EXPLORER_PREFIX="/explorer/api"
```

Routes:
- `/api/block/1` → `http://node:8080/block/1`
- `/ws` → `ws://node:8080/`
- `/explorer/api/block/1` → `http://node:8080/block/1`

### Example 2: No Prefix

```bash
export API_PREFIX="/"
export WS_PREFIX="/websocket"
```

Routes:
- `/block/1` → `http://node:8080/block/1`
- `/websocket` → `ws://node:8080/`

### Example 3: Custom Prefixes

```bash
export API_PREFIX="/blockchain/api"
export EXPLORER_PREFIX="/blockchain/explorer"
```

Routes:
- `/blockchain/api/tx/abc123` → `http://node:8080/tx/abc123`
- `/blockchain/explorer/block/1` → `http://node:8080/block/1`

## Path Rewriting Logic

The gateway uses intelligent path rewriting:

1. Strips the configured prefix from incoming requests
2. Forwards the remainder to the upstream node
3. Preserves query parameters and URL encoding

Example:
```
Request: GET /api/block/1?verbose=true
Rewritten: GET /block/1?verbose=true (to node)
```

## CORS Configuration

### Allow All Origins

```bash
export ALLOWED_ORIGINS="*"
```

### Specific Origins

```bash
export ALLOWED_ORIGINS="https://ippan.com,https://explorer.ippan.network,http://localhost:3000"
```

The gateway will:
- Accept requests from listed origins
- Reject others with CORS errors (logged as warnings)
- Support credentials if needed

## Troubleshooting

### Gateway Won't Start

```bash
# Check if port is already in use
lsof -i :8080

# Check logs
docker compose logs gateway

# Test connection to node
curl http://node:8080/health
```

### Upstream Connection Failed

```bash
# Verify node is running
docker compose ps ippan-node

# Check TARGET_RPC_URL
echo $TARGET_RPC_URL

# Test direct connection
curl $TARGET_RPC_URL/health
```

### CORS Errors

```bash
# Check allowed origins
echo $ALLOWED_ORIGINS

# Enable all origins for testing
export ALLOWED_ORIGINS="*"

# Check browser console for specific error
```

### WebSocket Connection Issues

```bash
# Verify WS target
echo $TARGET_WS_URL

# Check nginx WebSocket config
docker compose exec nginx cat /etc/nginx/nginx.conf | grep -A 20 "location /ws"

# Test WebSocket upgrade
curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Key: test" -H "Sec-WebSocket-Version: 13" \
  http://localhost:8080/ws
```

## Development

### Watch Mode

```bash
npm run dev
```

This starts the server with Node's built-in watch mode, automatically restarting on file changes.

### Testing Routes

```bash
# Health check
curl http://localhost:8080/health

# API endpoint
curl http://localhost:8080/api/version

# Explorer info
curl http://localhost:8080/explorer

# Explorer API
curl http://localhost:8080/explorer/api/peers
```

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Client    │────▶│   Gateway   │────▶│    Node     │
│  (Browser)  │     │  (Express)  │     │   (Axum)    │
└─────────────┘     └─────────────┘     └─────────────┘
      │                    │                    │
      │                    │                    │
      ▼                    ▼                    ▼
   /api/tx    ───────▶  /api/tx   ──────▶    /tx
   /ws        ───────▶   /ws      ──────▶    /
   /explorer  ───────▶  /explorer ──────▶    /
```

## Security Considerations

1. **Non-root User**: Container runs as `nodejs:nodejs` (UID 1001)
2. **CORS Protection**: Configurable origin whitelist
3. **Rate Limiting**: Implement at reverse proxy level (nginx)
4. **Request Size Limits**: JSON body limited to 2MB by default
5. **Health Checks**: Automatic container health monitoring
6. **Connection Limits**: Configure via environment variables

## Performance

- **Stateless**: No session storage, easy horizontal scaling
- **Connection Pooling**: Efficient upstream connection reuse
- **Buffering**: Disabled for API routes to reduce latency
- **Compression**: Enable at nginx level if needed

## Monitoring

### Health Check Response

```json
{
  "status": "healthy",
  "upstream": "http://node:8080",
  "details": {
    "status": "healthy",
    "upstreamStatus": 200,
    "payload": { ... }
  }
}
```

Status codes:
- `200` - Healthy: upstream responding normally
- `502` - Degraded: upstream reachable but returning errors
- `503` - Unreachable: cannot connect to upstream

### Metrics

The gateway logs all requests using Morgan combined format:

```
:remote-addr - :remote-user [:date[clf]] ":method :url HTTP/:http-version" :status :res[content-length] ":referrer" ":user-agent"
```

## License

Part of the IPPAN blockchain project. See main repository LICENSE.

## Links

- Main repo: https://github.com/dmrl789/ippan
- Documentation: https://docs.ippan.com
- Explorer: https://explorer.ippan.network
