import express from 'express'
import cors from 'cors'
import morgan from 'morgan'
import { createProxyMiddleware } from 'http-proxy-middleware'
import { URL } from 'url'

const app = express()

const port = Number.parseInt(process.env.PORT ?? '8080', 10)
const targetRpcUrl = process.env.TARGET_RPC_URL ?? 'http://node:8080'
const targetWsUrl = process.env.TARGET_WS_URL ?? targetRpcUrl.replace(/^http/, 'ws')
const targetHealthPath = process.env.TARGET_HEALTH_PATH ?? '/health'
const allowedOrigins = (process.env.ALLOWED_ORIGINS ?? '*')
  .split(',')
  .map((value) => value.trim())
  .filter((value) => value.length > 0)

const healthTimeoutMs = Number.parseInt(process.env.HEALTH_TIMEOUT_MS ?? '5000', 10)
const rewriteApiPrefix = process.env.API_PREFIX ?? '/api'
const rewriteWsPrefix = process.env.WS_PREFIX ?? '/ws'

function isOriginAllowed(origin) {
  if (!origin || allowedOrigins.length === 0) {
    return true
  }

  if (allowedOrigins.includes('*')) {
    return true
  }

  return allowedOrigins.includes(origin)
}

const corsOptions = {
  origin(origin, callback) {
    if (isOriginAllowed(origin)) {
      callback(null, true)
    } else {
      callback(new Error('Not allowed by CORS'))
    }
  },
  credentials: false,
}

app.disable('x-powered-by')
app.use(cors(corsOptions))
app.use(express.json({ limit: process.env.JSON_BODY_LIMIT ?? '2mb' }))
app.use(morgan('combined'))

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

app.use(
  rewriteApiPrefix,
  createProxyMiddleware({
    target: targetRpcUrl,
    changeOrigin: true,
    ws: false,
    pathRewrite: (path, req) => {
      if (!rewriteApiPrefix || rewriteApiPrefix === '/') {
        return path
      }
      return path.replace(new RegExp(`^${rewriteApiPrefix}`), '') || '/'
    },
    logLevel: process.env.PROXY_LOG_LEVEL ?? 'warn',
  }),
)

app.use(
  rewriteWsPrefix,
  createProxyMiddleware({
    target: targetWsUrl,
    changeOrigin: true,
    ws: true,
    logLevel: process.env.PROXY_LOG_LEVEL ?? 'warn',
    pathRewrite: (path, req) => {
      if (!rewriteWsPrefix || rewriteWsPrefix === '/') {
        return path
      }
      return path.replace(new RegExp(`^${rewriteWsPrefix}`), '') || '/'
    },
  }),
)

app.use((req, res) => {
  res.status(404).json({
    error: 'Not found',
    path: req.path,
  })
})

const server = app.listen(port, '0.0.0.0', () => {
  console.log(`Gateway listening on port ${port}`)
  console.log(`Proxying API requests to ${targetRpcUrl}`)
  console.log(`Proxying websocket requests to ${targetWsUrl}`)
})

function shutdown() {
  console.log('Shutting down gateway...')
  server.close(() => {
    process.exit(0)
  })
}

process.on('SIGTERM', shutdown)
process.on('SIGINT', shutdown)
