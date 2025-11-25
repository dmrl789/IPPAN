# CORS Conflict Fix - Security Issue Resolution

## Issue Summary

**Priority**: P1 (High)  
**Type**: Security Issue  
**Component**: Nginx configuration for blockchain explorer

## Problem

The original nginx configuration added wildcard CORS headers (`Access-Control-Allow-Origin: *`) in two locations:

1. Explorer subdomain server block (`explorer.ippan.net`)
2. Main website `/explorer` location (`ippan.com/explorer`)

This created a **critical security conflict** because:

### 1. Duplicate CORS Headers
- Nginx adds: `Access-Control-Allow-Origin: *`
- Gateway adds: `Access-Control-Allow-Origin: <specific-origin>`
- **Result**: Browsers reject responses with multiple CORS headers

### 2. Security Bypass
- Wildcard `*` in nginx overrides gateway's origin whitelist
- Exposes blockchain RPC endpoints to **any origin**
- Defeats the purpose of `ALLOWED_ORIGINS` environment variable
- Opens vector for cross-origin attacks

### 3. Configuration Confusion
- CORS policy split between nginx and gateway
- Harder to maintain and audit
- Deployments might have different CORS policies

## Solution

**Remove all CORS headers from nginx; let gateway handle CORS exclusively.**

### Changes Made

#### File: `deployments/mainnet/nginx/nginx.conf`

**Before** (Lines 247-259):
```nginx
# CORS headers for explorer
add_header Access-Control-Allow-Origin "*" always;
add_header Access-Control-Allow-Methods "GET, POST, OPTIONS" always;
add_header Access-Control-Allow-Headers "Authorization, Content-Type, X-Requested-With" always;

if ($request_method = 'OPTIONS') {
    add_header Access-Control-Allow-Origin "*";
    add_header Access-Control-Allow-Methods "GET, POST, OPTIONS";
    add_header Access-Control-Allow-Headers "Authorization, Content-Type, X-Requested-With";
    add_header Content-Length 0;
    add_header Content-Type text/plain;
    return 204;
}
```

**After**:
```nginx
# CORS is handled by the gateway (apps/gateway/src/server.mjs)
# No CORS headers in nginx to avoid conflicts
```

**Before** (Lines 381-393):
```nginx
# CORS headers for explorer
add_header Access-Control-Allow-Origin "*" always;
add_header Access-Control-Allow-Methods "GET, POST, OPTIONS" always;
add_header Access-Control-Allow-Headers "Authorization, Content-Type, X-Requested-With" always;

if ($request_method = 'OPTIONS') {
    add_header Access-Control-Allow-Origin "*";
    add_header Access-Control-Allow-Methods "GET, POST, OPTIONS";
    add_header Access-Control-Allow-Headers "Authorization, Content-Type, X-Requested-With";
    add_header Content-Length 0;
    add_header Content-Type text/plain;
    return 204;
}
```

**After**:
```nginx
# CORS is handled by the gateway (apps/gateway/src/server.mjs)
# No CORS headers in nginx to avoid conflicts
```

## Architecture

### CORS Flow
```
┌─────────┐      ┌───────┐      ┌─────────┐      ┌──────────────┐
│ Browser │─────▶│ Nginx │─────▶│ Gateway │─────▶│ Blockchain   │
│         │      │       │      │         │      │ Node         │
└─────────┘      └───────┘      └─────────┘      └──────────────┘
                     │               │
                     │               └─ Adds CORS headers
                     │                  based on ALLOWED_ORIGINS
                     │
                     └─ NO CORS headers
                        (transparent proxy)
```

### Gateway CORS Configuration

**File**: `apps/gateway/src/server.mjs`

```javascript
const allowedOrigins = (process.env.ALLOWED_ORIGINS ?? '*')
  .split(',')
  .map((value) => value.trim())
  .filter((value) => value.length > 0)

const corsOptions = {
  origin(origin, callback) {
    if (!origin || isOriginAllowed(origin)) {
      callback(null, true)
      return
    }
    console.warn(`Blocked CORS origin: ${origin}`)
    callback(null, false)
  },
  credentials: false,
}

app.use(cors(corsOptions))
```

**Environment Variable**:
```bash
ALLOWED_ORIGINS=https://ippan.com,https://explorer.ippan.network,https://ippan.net
```

## Benefits

✅ **Single Source of Truth**: CORS policy defined in one place (gateway)  
✅ **Security**: Proper origin whitelisting enforced  
✅ **No Duplicates**: Single CORS header per response  
✅ **Configurable**: Change CORS policy via environment variables  
✅ **Maintainable**: Easier to audit and update  
✅ **Consistent**: Same CORS behavior across all deployments  

## Testing

Verify CORS works correctly:

```bash
# Should return proper CORS headers from gateway
curl -I -X OPTIONS https://ippan.com/explorer/api/health \
  -H "Origin: https://ippan.com" \
  -H "Access-Control-Request-Method: GET"

# Expected headers:
# Access-Control-Allow-Origin: https://ippan.com
# (NOT "*")
```

## Verification Checklist

After deploying the fix:

- [ ] No `Access-Control-Allow-Origin` headers in nginx config
- [ ] Gateway CORS middleware is active
- [ ] `ALLOWED_ORIGINS` environment variable is set
- [ ] Browser console shows no CORS errors
- [ ] OPTIONS preflight requests return 200/204
- [ ] Only allowed origins can access the API
- [ ] Wildcard origins blocked (if not in ALLOWED_ORIGINS)

## Production Configuration

### Recommended ALLOWED_ORIGINS

```bash
# Production
ALLOWED_ORIGINS=https://ippan.com,https://explorer.ippan.network,https://ippan.net

# Staging
ALLOWED_ORIGINS=https://staging.ippan.com,https://staging-explorer.ippan.network

# Development (only!)
ALLOWED_ORIGINS=http://localhost:3000,http://localhost:3001
```

⚠️ **NEVER use `ALLOWED_ORIGINS=*` in production!**

## References

- **Bot Review**: @chatgpt-codex-connector[bot] flagged this as P1 issue
- **Root Cause**: Duplicate CORS headers between nginx and gateway
- **Security Impact**: Wildcard CORS bypasses origin whitelist
- **Fix**: Remove nginx CORS, use gateway CORS exclusively

## Related Files

- ✅ `deployments/mainnet/nginx/nginx.conf` - Removed CORS headers
- ✅ `apps/gateway/src/server.mjs` - Gateway CORS implementation
- ✅ `EXPLORER_DEPLOYMENT_GUIDE.md` - Updated with CORS notes
- ✅ `apps/gateway/.env.example` - ALLOWED_ORIGINS example

---

**Date**: 2025-10-21  
**Status**: ✅ Fixed  
**Reviewer**: @chatgpt-codex-connector[bot]
