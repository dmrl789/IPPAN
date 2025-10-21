# Gateway Explorer Analysis for ippan.com/explorer

## Executive Summary

**Status:** ⚠️ **INCOMPLETE CONFIGURATION**

The Kubernetes ingress is configured to route `ippan.com/explorer` and `explorer.ippan.network` to a service named `ippan-explorer:3001`, but this service **does not exist** in any deployment configuration.

---

## 1. Current Configuration

### ✅ What Exists:

1. **Blockchain RPC API Endpoints** (`crates/rpc/src/server.rs`):
   - `/health` - Node health status
   - `/time` - IPPAN time service
   - `/version` - Node version
   - `/metrics` - Prometheus metrics
   - `/tx` (POST) - Submit transaction
   - `/tx/:hash` (GET) - Get transaction by hash
   - `/block/:id` (GET) - Get block by ID
   - `/account/:address` (GET) - Get account info
   - `/peers` (GET) - Network peer info
   - `/l2/*` - Layer 2 endpoints (config, networks, commits, exits)

2. **API Gateway** (`apps/gateway/src/server.mjs`):
   - Proxies `/api` to blockchain node RPC
   - Proxies `/ws` for WebSocket connections
   - CORS configuration
   - Health checks

3. **Kubernetes Ingress** (`deployments/kubernetes/ippan-ingress.yaml`):
   - Routes `explorer.ippan.network` → `ippan-explorer:3001`
   - Routes `ippan.com/explorer` → `ippan-explorer:3001`
   - Includes `https://ippan.com` in CORS origins

### ❌ What's Missing:

1. **No Explorer Service**:
   - `ippan-explorer` service referenced in ingress does not exist
   - No deployment manifest for explorer
   - No explorer UI application

2. **No Nginx Configuration for Explorer**:
   - `deployments/mainnet/nginx/nginx.conf` has no `/explorer` path routing
   - No reverse proxy configuration for explorer subdomain

3. **Gateway Not Configured for Explorer**:
   - Gateway doesn't expose explorer-specific routes
   - No static file serving for explorer UI

---

## 2. Problem Analysis

### Issue 1: Broken Kubernetes Routing
```yaml
# deployments/kubernetes/ippan-ingress.yaml (Lines 66-72)
- host: ippan.com
  http:
    paths:
    - path: /explorer
      pathType: Prefix
      backend:
        service:
          name: ippan-explorer  # ⚠️ SERVICE DOES NOT EXIST
          port:
            number: 3001
```

### Issue 2: Missing Nginx Configuration
The mainnet nginx config has no explorer configuration:
- No `/explorer` location block
- No `explorer.ippan.*` server block
- API-only configuration exists

### Issue 3: No Explorer Application
- No UI application for blockchain exploration
- No dedicated explorer service deployment
- README mentions "https://ippan.com/explorer" but it's not implemented

---

## 3. Solution Options

### Option A: Route Explorer Through Gateway (RECOMMENDED)

**Approach**: Use the gateway as a reverse proxy to serve blockchain data to a frontend explorer.

**Implementation**:

1. **Update Gateway** to add explorer API prefix:
   ```javascript
   // apps/gateway/src/server.mjs
   const explorerApiPrefix = process.env.EXPLORER_API_PREFIX ?? '/explorer/api'
   
   app.use(
     explorerApiPrefix,
     createProxyMiddleware({
       target: targetRpcUrl,
       changeOrigin: true,
       pathRewrite: (path) => {
         return path.replace(new RegExp(`^${explorerApiPrefix}`), '') || '/'
       },
     })
   )
   ```

2. **Update Kubernetes Ingress** to route to gateway:
   ```yaml
   - host: ippan.com
     http:
       paths:
       - path: /explorer
         pathType: Prefix
         backend:
           service:
             name: ippan-api  # Use existing gateway service
             port:
               number: 3000
   ```

3. **Add CORS** for explorer origin:
   ```toml
   # Update CORS in node configs
   cors_origins = [
       "https://ippan.com",
       "https://explorer.ippan.network",
   ]
   ```

**Pros**:
- Uses existing infrastructure
- No new services needed
- Simple to implement
- Minimal resource overhead

**Cons**:
- No dedicated explorer UI (frontend needed separately)
- Gateway becomes multi-purpose

---

### Option B: Deploy Dedicated Explorer Service

**Approach**: Create a standalone blockchain explorer service.

**Implementation**:

1. **Create Explorer Deployment**:
   ```yaml
   # deployments/kubernetes/ippan-explorer.yaml
   apiVersion: apps/v1
   kind: Deployment
   metadata:
     name: ippan-explorer
     namespace: ippan
   spec:
     replicas: 2
     selector:
       matchLabels:
         app: ippan-explorer
     template:
       metadata:
         labels:
           app: ippan-explorer
       spec:
         containers:
         - name: explorer
           image: ghcr.io/ippan/blockchain-explorer:latest
           ports:
           - containerPort: 3001
           env:
           - name: BLOCKCHAIN_RPC_URL
             value: "http://ippan-node-service:8080"
           - name: PORT
             value: "3001"
   ---
   apiVersion: v1
   kind: Service
   metadata:
     name: ippan-explorer
     namespace: ippan
   spec:
     type: ClusterIP
     ports:
     - port: 3001
       targetPort: 3001
     selector:
       app: ippan-explorer
   ```

2. **Choose Explorer Tech Stack**:
   - **BlockScout** - Full-featured blockchain explorer
   - **Subscan** - Substrate-compatible explorer
   - **Custom Next.js App** - Build custom UI using blockchain APIs

**Pros**:
- Full-featured blockchain explorer
- Dedicated service isolation
- Can use established explorer platforms

**Cons**:
- Additional infrastructure complexity
- More resources required
- Maintenance overhead for another service

---

### Option C: Unified UI with Explorer View

**Approach**: Integrate explorer functionality into existing UI.

**Implementation**:

1. **Add Explorer Routes to Unified UI**:
   ```typescript
   // unified-ui/app/explorer/page.tsx
   export default function ExplorerPage() {
     return <BlockchainExplorer />
   }
   ```

2. **Update Ingress** to route to unified UI:
   ```yaml
   - host: ippan.com
     http:
       paths:
       - path: /explorer
         pathType: Prefix
         backend:
           service:
             name: ippan-frontend  # Existing UI service
             port:
               number: 80
   ```

**Pros**:
- Single UI application
- Consistent UX
- No additional services

**Cons**:
- Couples explorer with main UI
- May bloat UI bundle size

---

## 4. Recommended Action Plan

### Phase 1: Immediate Fix (Option A)
1. ✅ Update gateway to expose explorer API routes
2. ✅ Fix Kubernetes ingress to route to gateway
3. ✅ Update CORS configuration
4. ✅ Add explorer endpoints to mainnet nginx config
5. ✅ Test `/explorer` path accessibility

### Phase 2: Explorer UI (Choose One)
- **Short-term**: Deploy BlockScout or similar (Option B)
- **Long-term**: Build custom explorer in unified UI (Option C)

### Phase 3: Production Validation
1. Verify `https://ippan.com/explorer` returns 200
2. Verify blockchain data accessible via explorer API
3. Verify CORS allows cross-origin requests
4. Load test explorer endpoints
5. Monitor performance and errors

---

## 5. Gateway Configuration Changes Needed

### Minimal Changes (Option A - Recommended)

**File**: `apps/gateway/src/server.mjs`

```javascript
// Add after existing imports
const enableExplorer = process.env.ENABLE_EXPLORER !== 'false'

// Add explorer routes (before the 404 handler)
if (enableExplorer) {
  // Proxy explorer API requests to blockchain node
  app.use(
    '/explorer/api',
    createProxyMiddleware({
      target: targetRpcUrl,
      changeOrigin: true,
      pathRewrite: (path) => path.replace(/^\/explorer\/api/, ''),
      logLevel: process.env.PROXY_LOG_LEVEL ?? 'warn',
    })
  )
  
  // Serve explorer UI (if static files provided)
  const explorerUiPath = process.env.EXPLORER_UI_PATH
  if (explorerUiPath) {
    app.use('/explorer', express.static(explorerUiPath))
  } else {
    // Fallback: redirect to API docs or return info
    app.get('/explorer', (req, res) => {
      res.json({
        name: 'IPPAN Blockchain Explorer',
        api: {
          health: '/explorer/api/health',
          time: '/explorer/api/time',
          blocks: '/explorer/api/block/:id',
          transactions: '/explorer/api/tx/:hash',
          accounts: '/explorer/api/account/:address',
        },
        docs: 'https://docs.ippan.com/api',
      })
    })
  }
}
```

### Environment Variables

```env
# Gateway .env additions
ENABLE_EXPLORER=true
EXPLORER_UI_PATH=/usr/share/explorer  # Optional: path to explorer UI static files
```

---

## 6. Testing Checklist

After implementing changes:

- [ ] `curl https://ippan.com/explorer` returns 200 (not 404/502)
- [ ] `curl https://explorer.ippan.network` resolves correctly
- [ ] `curl https://ippan.com/explorer/api/health` returns node health
- [ ] `curl https://ippan.com/explorer/api/block/1` returns block data
- [ ] `curl https://ippan.com/explorer/api/tx/<hash>` returns transaction
- [ ] CORS headers present in responses
- [ ] WebSocket connections work (if needed)
- [ ] Load test handles 100+ concurrent requests

---

## 7. Next Steps

1. **Decision Required**: Choose Option A, B, or C
2. **Implement Gateway Changes**: Update `apps/gateway/src/server.mjs`
3. **Update Ingress**: Fix Kubernetes routing
4. **Deploy & Test**: Validate all explorer endpoints
5. **Document**: Update README with correct explorer URL and capabilities
6. **Monitor**: Add metrics and logging for explorer traffic

---

## Appendix: Available Blockchain API Endpoints

All endpoints available at node RPC (port 8080):

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Node health status |
| `/time` | GET | IPPAN time (microseconds) |
| `/version` | GET | Node version info |
| `/metrics` | GET | Prometheus metrics |
| `/tx` | POST | Submit new transaction |
| `/tx/:hash` | GET | Get transaction by hash |
| `/block/:id` | GET | Get block by ID/height |
| `/account/:address` | GET | Get account balance & state |
| `/peers` | GET | Connected peers info |
| `/l2/config` | GET | Layer 2 configuration |
| `/l2/networks` | GET | List L2 networks |
| `/l2/commits` | GET | List L2 commits |
| `/l2/commits/:l2_id` | GET | L2 commits for specific network |
| `/l2/exits` | GET | List L2 exit records |
| `/l2/exits/:l2_id` | GET | L2 exits for specific network |

These endpoints should be exposed via the gateway at `/explorer/api/*` for public blockchain exploration.

---

**Report Generated**: 2025-10-21  
**Branch**: `cursor/check-gateway-for-blockchain-explorer-1909`  
**Status**: Awaiting implementation decision
