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
const explorerCacheTtlMs = Number.parseInt(process.env.EXPLORER_CACHE_TTL_MS ?? '2000', 10)
const explorerFetchTimeoutMs = Number.parseInt(process.env.EXPLORER_FETCH_TIMEOUT_MS ?? '5000', 10)
const explorerMaxBlocks = Number.parseInt(process.env.EXPLORER_MAX_BLOCKS ?? '50', 10)
const explorerMaxTransactions = Number.parseInt(
  process.env.EXPLORER_MAX_TRANSACTIONS ?? '100',
  10,
)
const explorerDefaultBlockSummary = Number.parseInt(
  process.env.EXPLORER_DEFAULT_BLOCKS ?? '10',
  10,
)
const explorerDefaultTransactionSummary = Number.parseInt(
  process.env.EXPLORER_DEFAULT_TRANSACTIONS ?? '20',
  10,
)
const explorerDefaultBlockLimit = Math.min(
  Math.max(1, explorerDefaultBlockSummary),
  explorerMaxBlocks,
)
const explorerDefaultTransactionLimit = Math.min(
  Math.max(1, explorerDefaultTransactionSummary),
  explorerMaxTransactions,
)

const apiMountPath = normalizeMountPath(rewriteApiPrefix)
const wsMountPath = normalizeMountPath(rewriteWsPrefix)
const explorerBaseMounts = new Set([''])
if (apiMountPath !== '/' && apiMountPath !== '') {
  explorerBaseMounts.add(apiMountPath)
}

function normalizeMountPath(prefix) {
  if (!prefix || prefix === '/') {
    return '/'
  }

  let normalized = prefix.trim()
  if (!normalized.startsWith('/')) {
    normalized = `/${normalized}`
  }
  if (normalized.length > 1 && normalized.endsWith('/')) {
    normalized = normalized.replace(/\/+/g, '/').replace(/\/$/, '')
  }
  return normalized
}

function clampNumber(value, { min, max, fallback }) {
  const numeric = Number.parseInt(value ?? '', 10)
  if (Number.isNaN(numeric)) {
    return fallback
  }
  return Math.min(Math.max(numeric, min), max)
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function ensureDateFromMicros(micros) {
  if (typeof micros !== 'number' || Number.isNaN(micros) || !Number.isFinite(micros)) {
    return null
  }
  try {
    return new Date(Math.floor(micros / 1000))
  } catch (error) {
    return null
  }
}

function toUint8Array(value) {
  if (!value) {
    return null
  }
  if (value instanceof Uint8Array) {
    return value
  }
  if (Array.isArray(value)) {
    return Uint8Array.from(value)
  }
  if (typeof value === 'string') {
    const hex = value.startsWith('0x') ? value.slice(2) : value
    if (hex.length % 2 !== 0) {
      return null
    }
    return Uint8Array.from(Buffer.from(hex, 'hex'))
  }
  if (typeof value === 'object' && Array.isArray(value.data)) {
    return Uint8Array.from(value.data)
  }
  return null
}

function bytesToHex(value, { withPrefix = true } = {}) {
  const bytes = toUint8Array(value)
  if (!bytes) {
    return null
  }
  const hex = Buffer.from(bytes).toString('hex')
  return withPrefix ? `0x${hex}` : hex
}

function hashTimerToHex(hashTimer) {
  if (!hashTimer) {
    return null
  }
  const timePrefix = toUint8Array(hashTimer.time_prefix ?? hashTimer.timePrefix)
  const hashSuffix = toUint8Array(hashTimer.hash_suffix ?? hashTimer.hashSuffix)
  if (!timePrefix || !hashSuffix) {
    return null
  }
  return `${Buffer.from(timePrefix).toString('hex')}${Buffer.from(hashSuffix).toString('hex')}`
}

function hashTimerToMicros(hashTimer) {
  if (!hashTimer) {
    return null
  }
  const prefix = toUint8Array(hashTimer.time_prefix ?? hashTimer.timePrefix)
  if (!prefix || prefix.length !== 7) {
    return null
  }
  const buffer = Buffer.alloc(8)
  Buffer.from(prefix).copy(buffer, 1)
  return Number(buffer.readBigUInt64BE())
}

function toHashTimerView(hashTimer) {
  if (!hashTimer) {
    return null
  }
  const micros = hashTimerToMicros(hashTimer)
  const hex = hashTimerToHex(hashTimer)
  const date = ensureDateFromMicros(micros)
  return {
    hex,
    micros,
    iso: date?.toISOString() ?? null,
  }
}

function toIppanMicros(value) {
  if (value == null) {
    return null
  }
  if (typeof value === 'number') {
    return value
  }
  if (typeof value === 'string') {
    const parsed = Number.parseInt(value, 10)
    return Number.isNaN(parsed) ? null : parsed
  }
  if (typeof value === 'object') {
    if (typeof value.micros === 'number') {
      return value.micros
    }
    if (typeof value[0] === 'number') {
      return value[0]
    }
  }
  return null
}

class ExplorerError extends Error {
  constructor(message, { status = 502, upstream, cause } = {}) {
    super(message)
    this.name = 'ExplorerError'
    this.status = status
    this.upstream = upstream
    if (cause) {
      this.cause = cause
    }
  }
}

function createExplorerService({
  rpcUrl,
  fetchTimeoutMs,
  cacheTtlMs,
  maxBlocks,
  maxTransactions,
  defaultBlockLimit,
  defaultTransactionLimit,
}) {
  const cache = new Map()
  const fallbackBlockLimit = Math.min(
    Math.max(1, defaultBlockLimit ?? maxBlocks),
    maxBlocks,
  )
  const fallbackTransactionLimit = Math.min(
    Math.max(1, defaultTransactionLimit ?? maxTransactions),
    maxTransactions,
  )

  function setCache(key, value, ttl = cacheTtlMs) {
    if (ttl <= 0) {
      return value
    }
    cache.set(key, { value, expiresAt: Date.now() + ttl })
    return value
  }

  function getCache(key) {
    const entry = cache.get(key)
    if (!entry) {
      return null
    }
    if (entry.expiresAt < Date.now()) {
      cache.delete(key)
      return null
    }
    return entry.value
  }

  async function requestJson(path, { allowNotFound = false } = {}) {
    const url = new URL(path, rpcUrl)
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), fetchTimeoutMs)

    try {
      const response = await fetch(url, {
        signal: controller.signal,
        headers: {
          accept: 'application/json',
        },
      })

      if (!response.ok) {
        if (allowNotFound && response.status === 404) {
          return null
        }
        const body = await response.text().catch(() => '')
        throw new ExplorerError(`Upstream ${response.status} ${response.statusText}`, {
          status: response.status === 404 ? 404 : 502,
          upstream: {
            url: url.toString(),
            status: response.status,
            statusText: response.statusText,
            body: body?.slice(0, 4096) ?? '',
          },
        })
      }

      const data = await response.json()
      return data
    } catch (error) {
      if (error instanceof ExplorerError) {
        throw error
      }
      if (error.name === 'AbortError') {
        throw new ExplorerError(`Request to ${url.toString()} timed out`, {
          cause: error,
          upstream: { url: url.toString() },
        })
      }
      throw new ExplorerError(`Failed to reach upstream ${url.toString()}: ${error.message}`, {
        cause: error,
        upstream: { url: url.toString() },
      })
    } finally {
      clearTimeout(timer)
    }
  }

  async function getHealth() {
    const cacheKey = 'health'
    const cached = getCache(cacheKey)
    if (cached) {
      return cached
    }
    const result = await requestJson('/health')
    return setCache(cacheKey, result)
  }

  async function getMetrics() {
    const cacheKey = 'metrics'
    const cached = getCache(cacheKey)
    if (cached) {
      return cached
    }
    const metricsUrl = new URL('/metrics', rpcUrl)
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), fetchTimeoutMs)
    try {
      const response = await fetch(metricsUrl, {
        signal: controller.signal,
        headers: {
          accept: 'text/plain',
        },
      })
      if (!response.ok) {
        throw new ExplorerError(`Upstream ${response.status} ${response.statusText}`, {
          upstream: {
            url: metricsUrl.toString(),
            status: response.status,
            statusText: response.statusText,
          },
        })
      }
      const text = await response.text()
      return setCache(cacheKey, text)
    } catch (error) {
      if (error instanceof ExplorerError) {
        throw error
      }
      if (error.name === 'AbortError') {
        throw new ExplorerError(`Request to ${metricsUrl.toString()} timed out`, {
          cause: error,
          upstream: { url: metricsUrl.toString() },
        })
      }
      throw new ExplorerError(`Failed to reach upstream ${metricsUrl.toString()}: ${error.message}`, {
        cause: error,
        upstream: { url: metricsUrl.toString() },
      })
    } finally {
      clearTimeout(timer)
    }
  }

  async function getBlock(identifier) {
    const cacheKey = `block:${identifier}`
    const cached = getCache(cacheKey)
    if (cached) {
      return cached
    }
    const block = await requestJson(`/block/${identifier}`, { allowNotFound: true })
    if (!block) {
      return null
    }
    return setCache(cacheKey, block)
  }

  async function getTransaction(hash) {
    return requestJson(`/tx/${hash}`, { allowNotFound: true })
  }

  async function getAccount(address) {
    return requestJson(`/account/${address}`, { allowNotFound: true })
  }

  function toBlockSummary(block) {
    if (!block || !block.header) {
      return null
    }

    const txs = Array.isArray(block.transactions) ? block.transactions : []
    const parentIds = Array.isArray(block.header.parent_ids)
      ? block.header.parent_ids.map((value) => bytesToHex(value))
      : []
    const payloadIds = Array.isArray(block.header.payload_ids)
      ? block.header.payload_ids.map((value) => bytesToHex(value))
      : []

    const blockId = bytesToHex(block.header.id)
    const creator = bytesToHex(block.header.creator)
    const signature = bytesToHex(block.signature)
    const hashTimer = toHashTimerView(block.header.hashtimer)
    const prevHashes = Array.isArray(block.prev_hashes)
      ? block.prev_hashes
      : Array.isArray(block.header.prev_hashes)
      ? block.header.prev_hashes
      : []

    return {
      id: blockId,
      height: block.header.round ?? null,
      creator,
      txCount: txs.length,
      hashtimer: hashTimer,
      signature,
      parentIds,
      payloadIds,
      prevHashes,
      raw: block,
    }
  }

  function toTransactionSummary(tx, metadata = {}) {
    if (!tx) {
      return null
    }

    const micros = toIppanMicros(tx.timestamp)
    const date = ensureDateFromMicros(micros)

    return {
      id: bytesToHex(tx.id),
      from: bytesToHex(tx.from),
      to: bytesToHex(tx.to),
      amount: tx.amount ?? null,
      nonce: tx.nonce ?? null,
      visibility: tx.visibility ?? null,
      topics: Array.isArray(tx.topics) ? tx.topics : [],
      hashtimer: toHashTimerView(tx.hashtimer),
      timestamp: {
        micros,
        iso: date?.toISOString() ?? null,
      },
      blockId: metadata.block?.id ?? null,
      blockHeight: metadata.block?.height ?? null,
      index: metadata.index ?? null,
      raw: tx,
    }
  }

  function toNetworkSummary(health) {
    if (!health) {
      return null
    }
    return {
      status: health.status ?? null,
      nodeId: health.node_id ?? null,
      uptimeSecs: health.uptime_secs ?? null,
      peerCount: health.peer_count ?? null,
      mempoolSize: health.mempool_size ?? null,
      reqTotal: health.req_total ?? null,
      timeUs: health.time_us ?? null,
      consensus: health.consensus ?? null,
      l2: health.l2_config ?? null,
    }
  }

  async function getRecentBlocks(limit, { startHeight } = {}) {
    const health = await getHealth()
    const consensusHeight = health?.consensus?.latest_block_height
    const latestHeight =
      typeof startHeight === 'number'
        ? Math.min(startHeight, consensusHeight ?? startHeight)
        : typeof consensusHeight === 'number'
        ? consensusHeight
        : null

    if (latestHeight == null || Number.isNaN(latestHeight)) {
      return { latestHeight: null, blocks: [] }
    }

    const normalizedLimit = clampNumber(limit, {
      min: 1,
      max: maxBlocks,
      fallback: fallbackBlockLimit,
    })

    const blocks = []
    let height = latestHeight
    let attempts = 0
    const maxAttempts = normalizedLimit * 5

    while (height >= 0 && blocks.length < normalizedLimit && attempts < maxAttempts) {
      const block = await getBlock(height.toString())
      if (block) {
        blocks.push({ raw: block, summary: toBlockSummary(block) })
      }
      height -= 1
      attempts += 1
    }

    return { latestHeight, blocks }
  }

  async function getRecentTransactions(limit, { prefetchedBlocks, latestHeight } = {}) {
    const normalizedLimit = clampNumber(limit, {
      min: 1,
      max: maxTransactions,
      fallback: fallbackTransactionLimit,
    })

    let blocksData
    if (Array.isArray(prefetchedBlocks) && prefetchedBlocks.length > 0) {
      blocksData = prefetchedBlocks
    } else {
      const blockFetch = await getRecentBlocks(Math.min(normalizedLimit * 2, maxBlocks))
      blocksData = blockFetch.blocks
      latestHeight = blockFetch.latestHeight
    }

    const transactions = []
    for (const block of blocksData) {
      const txs = Array.isArray(block.raw?.transactions) ? block.raw.transactions : []
      for (let index = 0; index < txs.length; index += 1) {
        const summary = toTransactionSummary(txs[index], {
          block: block.summary,
          index,
        })
        if (summary) {
          transactions.push(summary)
        }
        if (transactions.length >= normalizedLimit) {
          break
        }
      }
      if (transactions.length >= normalizedLimit) {
        break
      }
    }

    return { latestHeight: latestHeight ?? null, transactions }
  }

  async function search(query) {
    if (!query || typeof query !== 'string') {
      return { query: query ?? '', matches: [] }
    }

    const trimmed = query.trim()
    if (!trimmed) {
      return { query: '', matches: [] }
    }

    const matches = []

    const isNumeric = /^\d+$/.test(trimmed)
    const normalizedHex = trimmed.startsWith('0x') ? trimmed.slice(2) : trimmed
    const isHex = /^[0-9a-fA-F]{64}$/.test(normalizedHex)

    if (isNumeric) {
      const block = await getBlock(trimmed)
      if (block) {
        matches.push({
          type: 'block',
          lookup: trimmed,
          summary: toBlockSummary(block),
        })
      }
    }

    if (isHex) {
      const prefixed = trimmed.startsWith('0x') ? trimmed : `0x${normalizedHex}`
      const block = await getBlock(prefixed)
      if (block) {
        matches.push({
          type: 'block',
          lookup: prefixed,
          summary: toBlockSummary(block),
        })
      }

      const tx = await getTransaction(prefixed)
      if (tx) {
        matches.push({
          type: 'transaction',
          lookup: prefixed,
          summary: toTransactionSummary(tx),
        })
      }

      const account = await getAccount(prefixed)
      if (account) {
        matches.push({
          type: 'account',
          lookup: prefixed,
          account,
        })
      }
    }

    return {
      query: trimmed,
      matches,
    }
  }

  return {
    getHealth,
    getMetrics,
    getBlock,
    getTransaction,
    getAccount,
    getRecentBlocks,
    getRecentTransactions,
    search,
    toBlockSummary,
    toTransactionSummary,
    toNetworkSummary,
    maxBlocks,
    maxTransactions,
  }
}

const explorerService = createExplorerService({
  rpcUrl: targetRpcUrl,
  fetchTimeoutMs: explorerFetchTimeoutMs,
  cacheTtlMs: explorerCacheTtlMs,
  maxBlocks: explorerMaxBlocks,
  maxTransactions: explorerMaxTransactions,
  defaultBlockLimit: explorerDefaultBlockLimit,
  defaultTransactionLimit: explorerDefaultTransactionLimit,
})

function normalizeExplorerBase(base) {
  if (!base || base === '/' || base === '') {
    return ''
  }
  let normalized = base.trim()
  if (!normalized.startsWith('/')) {
    normalized = `/${normalized}`
  }
  if (normalized.endsWith('/')) {
    normalized = normalized.slice(0, -1)
  }
  return normalized
}

function buildExplorerPath(prefix, suffix) {
  if (!suffix.startsWith('/')) {
    throw new Error('Explorer route suffix must start with "/"')
  }
  if (!prefix) {
    return suffix
  }
  const trimmedPrefix = prefix.endsWith('/') ? prefix.slice(0, -1) : prefix
  if (trimmedPrefix === '') {
    return suffix
  }
  return `${trimmedPrefix}${suffix}`
}

function asyncHandler(handler) {
  return (req, res, next) => {
    Promise.resolve(handler(req, res, next)).catch(next)
  }
}

function createExplorerRouteDefinitions(service) {
  return [
    {
      method: 'get',
      path: '/explorer/summary',
      handler: asyncHandler(async (req, res) => {
        const blockLimit = clampNumber(req.query.blocks, {
          min: 1,
          max: explorerMaxBlocks,
          fallback: explorerDefaultBlockLimit,
        })
        const transactionLimit = clampNumber(req.query.transactions, {
          min: 1,
          max: explorerMaxTransactions,
          fallback: explorerDefaultTransactionLimit,
        })

        const blockData = await service.getRecentBlocks(blockLimit)
        const [health, transactionData] = await Promise.all([
          service.getHealth(),
          service.getRecentTransactions(transactionLimit, {
            prefetchedBlocks: blockData.blocks,
            latestHeight: blockData.latestHeight,
          }),
        ])

        res.json({
          status: 'ok',
          fetchedAt: new Date().toISOString(),
          latestHeight: blockData.latestHeight ?? transactionData.latestHeight ?? null,
          network: service.toNetworkSummary(health),
          blocks: blockData.blocks
            .map((entry) => entry.summary)
            .filter((summary) => Boolean(summary)),
          transactions: transactionData.transactions,
        })
      }),
    },
    {
      method: 'get',
      path: '/explorer/health',
      handler: asyncHandler(async (_req, res) => {
        const health = await service.getHealth()
        res.json({
          status: 'ok',
          fetchedAt: new Date().toISOString(),
          network: service.toNetworkSummary(health),
        })
      }),
    },
    {
      method: 'get',
      path: '/explorer/metrics',
      handler: asyncHandler(async (_req, res) => {
        const metrics = await service.getMetrics()
        res.type('text/plain').send(metrics ?? '')
      }),
    },
    {
      method: 'get',
      path: '/explorer/blocks',
      handler: asyncHandler(async (req, res) => {
        const limit = clampNumber(req.query.limit ?? req.query.count, {
          min: 1,
          max: explorerMaxBlocks,
          fallback: explorerDefaultBlockLimit,
        })
        const startRaw = req.query.start ?? req.query.from ?? req.query.height
        const startHeight = Number.parseInt(startRaw ?? '', 10)
        const options = {}
        if (!Number.isNaN(startHeight)) {
          options.startHeight = startHeight
        }

        const data = await service.getRecentBlocks(limit, options)
        res.json({
          latestHeight: data.latestHeight,
          blocks: data.blocks
            .map((entry) => entry.summary)
            .filter((summary) => Boolean(summary)),
        })
      }),
    },
    {
      method: 'get',
      path: '/explorer/blocks/:id',
      handler: asyncHandler(async (req, res) => {
        const { id } = req.params
        const block = await service.getBlock(id)
        if (!block) {
          res.status(404).json({ error: 'Block not found', id })
          return
        }
        res.json({ summary: service.toBlockSummary(block) })
      }),
    },
    {
      method: 'get',
      path: '/explorer/transactions',
      handler: asyncHandler(async (req, res) => {
        const limit = clampNumber(req.query.limit ?? req.query.count, {
          min: 1,
          max: explorerMaxTransactions,
          fallback: explorerDefaultTransactionLimit,
        })
        const data = await service.getRecentTransactions(limit)
        res.json({
          latestHeight: data.latestHeight,
          transactions: data.transactions,
        })
      }),
    },
    {
      method: 'get',
      path: '/explorer/transactions/:hash',
      handler: asyncHandler(async (req, res) => {
        const { hash } = req.params
        const tx = await service.getTransaction(hash)
        if (!tx) {
          res.status(404).json({ error: 'Transaction not found', hash })
          return
        }
        res.json({ summary: service.toTransactionSummary(tx), transaction: tx })
      }),
    },
    {
      method: 'get',
      path: '/explorer/accounts/:address',
      handler: asyncHandler(async (req, res) => {
        const { address } = req.params
        const account = await service.getAccount(address)
        if (!account) {
          res.status(404).json({ error: 'Account not found', address })
          return
        }
        res.json({ account })
      }),
    },
    {
      method: 'get',
      path: '/explorer/search',
      handler: asyncHandler(async (req, res) => {
        const query = req.query.q ?? req.query.query ?? ''
        const result = await service.search(String(query))
        res.json(result)
      }),
    },
  ]
}

function registerExplorerRoutes(app, service, baseMounts) {
  const mounts = Array.from(baseMounts ?? [''])
  const routes = createExplorerRouteDefinitions(service)
  const registered = new Set()

  for (const base of mounts) {
    const normalizedBase = normalizeExplorerBase(base)
    for (const route of routes) {
      const fullPath = buildExplorerPath(normalizedBase, route.path)
      const key = `${route.method}:${fullPath}`
      if (registered.has(key)) {
        continue
      }
      registered.add(key)
      app[route.method](fullPath, route.handler)
    }
  }
}

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

registerExplorerRoutes(app, explorerService, explorerBaseMounts)

const healthPaths = Array.from(
  new Set(
    [
      '/health',
      '/api/health',
      apiMountPath !== '/' && apiMountPath !== '' ? `${apiMountPath}/health` : null,
    ].filter(Boolean),
  ),
)

const nodeHealthPaths = Array.from(
  new Set(
    [
      '/api/health/node',
      apiMountPath === '/' ? '/health/node' : null,
      apiMountPath !== '/' && apiMountPath !== '' ? `${apiMountPath}/health/node` : null,
    ].filter(Boolean),
  ),
)

const apiRewritePattern = apiMountPath === '/' ? null : new RegExp(`^${escapeRegex(apiMountPath)}`)
const wsRewritePattern = wsMountPath === '/' ? null : new RegExp(`^${escapeRegex(wsMountPath)}`)

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

app.get(healthPaths, async (_req, res) => {
  const result = await pingTarget()
  const statusCode = result.status === 'healthy' ? 200 : result.status === 'degraded' ? 502 : 503
  res.status(statusCode).json({
    status: result.status,
    upstream: targetRpcUrl,
    details: result,
  })
})

app.get(nodeHealthPaths, async (_req, res) => {
  const result = await pingTarget()
  const statusCode = result.status === 'healthy' ? 200 : result.status === 'degraded' ? 502 : 503
  res.status(statusCode).json(result)
})

app.use(
  apiMountPath,
  createProxyMiddleware({
    target: targetRpcUrl,
    changeOrigin: true,
    ws: false,
    pathRewrite: (path) => {
      if (!apiRewritePattern) {
        return path
      }
      return path.replace(apiRewritePattern, '') || '/'
    },
    logLevel: process.env.PROXY_LOG_LEVEL ?? 'warn',
  }),
)

app.use(
  wsMountPath,
  createProxyMiddleware({
    target: targetWsUrl,
    changeOrigin: true,
    ws: true,
    logLevel: process.env.PROXY_LOG_LEVEL ?? 'warn',
    pathRewrite: (path) => {
      if (!wsRewritePattern) {
        return path
      }
      return path.replace(wsRewritePattern, '') || '/'
    },
  }),
)

function gatewayErrorHandler(err, _req, res, next) {
  if (res.headersSent) {
    next(err)
    return
  }
  const status = err instanceof ExplorerError ? err.status ?? 502 : err.status ?? 500
  const response = {
    error: err?.message ?? 'Internal server error',
  }
  if (err instanceof ExplorerError && err.upstream) {
    response.upstream = err.upstream
  }
  if (process.env.NODE_ENV !== 'production' && err?.stack) {
    response.stack = err.stack.split('\n')
  }
  console.error('Gateway error:', err)
  res.status(status).json(response)
}

app.use(gatewayErrorHandler)

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
