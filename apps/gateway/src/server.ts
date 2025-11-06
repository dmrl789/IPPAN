import cors, { type CorsOptions } from 'cors'
import express, { type Request, type Response } from 'express'
import morgan from 'morgan'
import { createProxyMiddleware, type Options as HttpProxyOptions } from 'http-proxy-middleware'
import { URL } from 'node:url'

type HealthyResult = {
  status: 'healthy'
  upstreamStatus: number
  payload: unknown
}

type DegradedResult = {
  status: 'degraded'
  upstreamStatus: number
}

type UnreachableResult = {
  status: 'unreachable'
  error: string
}

type PingResult = HealthyResult | DegradedResult | UnreachableResult

const app = express()

const LOG_LEVELS = ['debug', 'info', 'warn', 'error', 'silent'] as const
type ProxyLogLevel = (typeof LOG_LEVELS)[number]
type PathRewriter = (path: string) => string

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
const enableExplorer = process.env.ENABLE_EXPLORER !== 'false'
const explorerPrefix = process.env.EXPLORER_PREFIX ?? '/explorer/api'
const proxyLogLevel: ProxyLogLevel = resolveLogLevel(process.env.PROXY_LOG_LEVEL)

function resolveLogLevel(value?: string | null): ProxyLogLevel {
  if (!value) {
    return 'warn'
  }

  const normalized = value.toLowerCase()
  return isProxyLogLevel(normalized) ? normalized : 'warn'
}

function isOriginAllowed(origin: string | undefined): boolean {
  if (!origin || allowedOrigins.length === 0) {
    return true
  }

  if (allowedOrigins.includes('*')) {
    return true
  }

  return allowedOrigins.includes(origin)
}

const corsOptions: CorsOptions = {
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
app.use(cors(corsOptions))
app.use(express.json({ limit: process.env.JSON_BODY_LIMIT ?? '2mb' }))
app.use(morgan('combined'))

async function pingTarget(): Promise<PingResult> {
  try {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), healthTimeoutMs)
    const healthUrl = new URL(targetHealthPath, targetRpcUrl)
    const response = await fetch(healthUrl, { signal: controller.signal })
    clearTimeout(timeout)

    if (!response.ok) {
      return {
        status: 'degraded',
        upstreamStatus: response.status,
      }
    }

    const payload = await response
      .json()
      .catch(() => null)

    return {
      status: 'healthy',
      upstreamStatus: response.status,
      payload,
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error'
    return {
      status: 'unreachable',
      error: message,
    }
  }
}

app.get(['/health', '/api/health'], async (_req: Request, res: Response) => {
  const result = await pingTarget()
  const statusCode =
    result.status === 'healthy' ? 200 : result.status === 'degraded' ? 502 : 503

  res.status(statusCode).json({
    status: result.status,
    upstream: targetRpcUrl,
    details: result,
  })
})

app.get('/api/health/node', async (_req: Request, res: Response) => {
  const result = await pingTarget()
  const statusCode =
    result.status === 'healthy' ? 200 : result.status === 'degraded' ? 502 : 503

  res.status(statusCode).json(result)
})

const apiRewrite = createPathRewriter(rewriteApiPrefix)
app.use(
  rewriteApiPrefix,
  createProxyMiddleware(
    createProxyOptions({ target: targetRpcUrl, ws: false, rewrite: apiRewrite }),
  ),
)

const wsRewrite = createPathRewriter(rewriteWsPrefix)
app.use(
  rewriteWsPrefix,
  createProxyMiddleware(
    createProxyOptions({ target: targetWsUrl, ws: true, rewrite: wsRewrite }),
  ),
)

if (enableExplorer) {
  const explorerRewrite = createPathRewriter(explorerPrefix)

  app.use(
    explorerPrefix,
    createProxyMiddleware(
      createProxyOptions({ target: targetRpcUrl, ws: false, rewrite: explorerRewrite }),
    ),
  )

  app.get('/explorer', (_req: Request, res: Response) => {
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

app.use((req: Request, res: Response) => {
  res.status(404).json({
    error: 'Not found',
    path: req.path,
  })
})

const server = app.listen(port, '0.0.0.0', () => {
  console.log(`Gateway listening on port ${port}`)
  console.log(`Proxying API requests to ${targetRpcUrl}`)
  console.log(`Proxying websocket requests to ${targetWsUrl}`)
  if (enableExplorer) {
    console.log(`Blockchain explorer enabled at ${explorerPrefix}`)
  }
})

function createProxyOptions({
  target,
  ws,
  rewrite,
}: {
  target: string
  ws: boolean
  rewrite: PathRewriter
}): HttpProxyOptions {
  return {
    target,
    changeOrigin: true,
    ws,
    pathRewrite: (path) => rewrite(path),
    logLevel: proxyLogLevel,
  }
}

function createPathRewriter(prefix: string | undefined): PathRewriter {
  if (!prefix || prefix === '/') {
    return (path: string) => path
  }

  const matcher = new RegExp(`^${escapeRegExp(prefix)}`)
  return (path: string) => path.replace(matcher, '') || '/'
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function isProxyLogLevel(value: string): value is ProxyLogLevel {
  return (LOG_LEVELS as readonly string[]).includes(value)
}

function shutdown(signal: NodeJS.Signals) {
  console.log(`Received ${signal}. Shutting down gateway...`)
  server.close((error) => {
    if (error) {
      console.error('Error while closing the server', error)
      process.exitCode = 1
    }
    process.exit()
  })
}

process.on('SIGTERM', () => shutdown('SIGTERM'))
process.on('SIGINT', () => shutdown('SIGINT'))
