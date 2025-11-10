import cors from 'cors'
import express from 'express'
import morgan from 'morgan'
import { createProxyMiddleware } from 'http-proxy-middleware'
import { URL } from 'url'

const app = express()

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------
const port = Number.parseInt(process.env.PORT ?? '8080', 10)
const targetRpcUrl = process.env.TARGET_RPC_URL ?? 'http://node:8080'
const targetWsUrl = process.env.TARGET_WS_URL ?? targetRpcUrl.replace(/^http/, 'ws')
const targetHealthPath = process.env.TARGET_HEALTH_PATH ?? '/health'
const allowedOrigins = (process.env.ALLOWED_ORIGINS ?? '*')
  .split(',')
  .map((value) => value.trim())
  .filter((value) => value.length > 0)

const healthTimeoutMs = Number.parseInt(process.env.HEALTH_TIMEOUT_MS ?? '5000', 10)
const rewriteApiPrefixRaw = process.env.API_PREFIX ?? '/api'
const rewriteWsPrefixRaw = process.env.WS_PREFIX ?? '/ws'
const enableExplorer = process.env.ENABLE_EXPLORER !== 'false'
const explorerPrefixRaw = process.env.EXPLORER_PREFIX ?? '/explorer/api'

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------
function normalizePrefix(prefix) {
  if (!prefix) return ''
  let normalized = prefix.trim()
  if (normalized === '') return ''
  if (!normalized.startsWith('/')) normalized = `/${normalized}`
  while (normalized.length > 1 && normalized.endsWith('/')) {
    normalized = normalized.slice(0, -1)
  }
  return normalized
}

function stripPathPrefix(path, prefix) {
  if (!prefix || prefix === '/') return path
  if (path === prefix) return '/'
  if (path.startsWith(`${prefix}/`)) {
    const stripped = path.slice(prefix.length)
    return stripped.length === 0 ? '/' : stripped
  }
  return path
}

function isOriginAllowed(origin) {
  if (!origin || allowedOrigins.length === 0) return true
  if (allowedOrigins.includes('*')) return true
  return allowedOrigins.includes(origin)
}

function handleProxyError(err, req, res, target) {
  const upstream = target?.href ?? target ?? 'upstream'
  const path = req?.originalUrl ?? req?.url ?? '<unknown>'
  console.error(`Proxy error for ${path} (${upstream}):`, err.message)
  if (res && !res.headersSent) {
    res.status(502).json({
      error: 'Bad gateway',
      reason: err.message,
      upstream,
    })
  }
}

// ---------------------------------------------------------------------------
// Normalized prefixes
// ---------------------------------------------------------------------------
const rewriteApiPrefix = normalizePrefix(rewriteApiPrefixRaw)
const rewriteWsPrefix = normalizePrefix(rewriteWsPrefixRaw)
const explorerPrefix = normalizePrefix(explorerPrefixRaw)

const apiMountPath = rewriteApiPrefix === '' ? '/' : rewriteApiPrefix
const wsMountPath = rewriteWsPrefix === '' ? '/' : rewriteWsPrefix
const explorerMountPath = explorerPrefix === '' ? '/' : explorerPrefix

// ---------------------------------------------------------------------------
// Middleware and configuration
// ---------------------------------------------------------------------------
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

app.disable('x-powered-by')
app.set('trust proxy', true)
app.use(cors(corsOptions))
app.use(morgan('combined'))

// ---------------------------------------------------------------------------
// Health checks
// ---------------------------------------------------------------------------
async function pingTarget() {
  try {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), healthTimeoutMs)
    const healthUrl = new URL(targetHealthPath, targetRpcUrl)
    const response = await fetch(healthUrl, { signal: controller.signal })
    clearTimeout(timeout)

    if (!response.ok) {
      return { status: 'degraded', upstreamStatus: response.status }
    }

    const payload = await response.json().catch(() => null)
    return { status: 'healthy', upstreamStatus: response.status, payload }
  } catch (error) {
    return { status: 'unreachable', error: error.message }
  }
}

app.get(['/health', '/api/health'], async (_req, res) => {
  const result = await pingTarget()
  const statusCode = result.status === 'healthy' ? 200 : result.status === 'degraded' ? 502 : 503
  res.status(statusCode).json({
    status: result.status,
    upstream: targetRpcUrl,
    details: result,
  })
})

app.get('/api/health/node', async (_req, res) => {
  const result = await pingTarget()
  const statusCode = result.status === 'healthy' ? 200 : result.status === 'degraded' ? 502 : 503
  res.status(statusCode).json(result)
})

// ---------------------------------------------------------------------------
// Proxy setup
// ---------------------------------------------------------------------------
const apiProxy = createProxyMiddleware({
  target: targetRpcUrl,
  changeOrigin: true,
  ws: false,
  logLevel: process.env.PROXY_LOG_LEVEL ?? 'warn',
  pathRewrite: (path) => stripPathPrefix(path, rewriteApiPrefix),
  onError: handleProxyError,
})

const wsProxy = createProxyMiddleware({
  target: targetWsUrl,
  changeOrigin: true,
  ws: true,
  logLevel: process.env.PROXY_LOG_LEVEL ?? 'warn',
  pathRewrite: (path) => stripPathPrefix(path, rewriteWsPrefix),
  onError: handleProxyError,
})

app.use(apiMountPath, apiProxy)
app.use(wsMountPath, wsProxy)

// ---------------------------------------------------------------------------
// Explorer proxy (optional)
// ---------------------------------------------------------------------------
if (enableExplorer) {
  const explorerProxy = createProxyMiddleware({
    target: targetRpcUrl,
    changeOrigin: true,
    ws: false,
    logLevel: process.env.PROXY_LOG_LEVEL ?? 'warn',
    pathRewrite: (path) => {
      const rewritten = stripPathPrefix(path, explorerPrefix)
      if (process.env.PROXY_LOG_LEVEL === 'debug' || process.env.NODE_ENV !== 'production') {
        console.log(`[Explorer] Rewriting ${path} -> ${rewritten}`)
      }
      return rewritten
    },
    onError: handleProxyError,
  })

  app.use(explorerMountPath, explorerProxy)

  app.get('/explorer', (_req, res) => {
    res.json({
      name: 'IPPAN Blockchain Explorer',
      version: '1.0.0',
      endpoints: {
        health: `${explorerPrefix}/health`,
        time: `${explorerPrefix}/time`,
        version: `${explorerPrefix}/version`,
        metrics: `${explorerPrefix}/metrics`,
        blocks: `${explorerPrefix}/block/:id`,
        transactions: `${explorerPrefix}/tx/:hash`,
        accounts: `${explorerPrefix}/account/:address`,
        peers: `${explorerPrefix}/peers`,
        l2: {
          config: `${explorerPrefix}/l2/config`,
          networks: `${explorerPrefix}/l2/networks`,
          commits: `${explorerPrefix}/l2/commits`,
          exits: `${explorerPrefix}/l2/exits`,
        },
      },
      documentation: 'https://docs.ippan.com/api',
    })
  })
}

// ---------------------------------------------------------------------------
// Fallback and server setup
// ---------------------------------------------------------------------------
app.use((req, res) => {
  res.status(404).json({
    error: 'Not found',
    path: req.path,
  })
})

const server = app.listen(port, '0.0.0.0', () => {
  console.log(`✓ Gateway listening on port ${port}`)
  console.log(`✓ Proxying API requests to ${targetRpcUrl}`)
  console.log(`  - API prefix: ${rewriteApiPrefix || '/'} -> ${targetRpcUrl}`)
  console.log(`✓ Proxying websocket requests to ${targetWsUrl}`)
  console.log(`  - WS prefix: ${rewriteWsPrefix || '/'} -> ${targetWsUrl}`)
  if (enableExplorer) {
    console.log(`✓ Blockchain explorer enabled at ${explorerPrefix}`)
  }
  console.log(`✓ CORS origins: ${allowedOrigins.join(', ')}`)
  console.log(`✓ Ready to accept connections`)
})

// ---------------------------------------------------------------------------
// WebSocket upgrade handler
// ---------------------------------------------------------------------------
function isWebsocketUpgrade(url, mountPath) {
  if (!url) return false
  const parsed = new URL(url, 'http://localhost')
  const pathname = parsed.pathname ?? '/'
  if (mountPath === '/' || mountPath === '') return true
  return pathname === mountPath || pathname.startsWith(`${mountPath}/`)
}

server.on('upgrade', (req, socket, head) => {
  if (isWebsocketUpgrade(req.url, wsMountPath)) {
    wsProxy.upgrade(req, socket, head)
    return
  }
  socket.destroy()
})

// ---------------------------------------------------------------------------
// Graceful shutdown
// ---------------------------------------------------------------------------
function shutdown() {
  console.log('Shutting down gateway...')
  server.close(() => {
    console.log('✓ Gateway stopped cleanly')
    process.exit(0)
  })
}

server.on('error', (error) => {
  console.error('Gateway server error:', error)
})

process.on('SIGTERM', shutdown)
process.on('SIGINT', shutdown)
