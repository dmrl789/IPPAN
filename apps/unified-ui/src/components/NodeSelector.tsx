import { useEffect, useMemo, useState } from 'react'
import { Card, Button, Field, Input, Badge } from './UI'
import { getApiBaseUrl, setApiBaseUrl } from '../lib/api'
import { UIConfig } from '../lib/config'

type NodeStatus = 'connected' | 'disconnected' | 'checking'

type SavedNode = {
  name: string
  url: string
}

type DisplayNode = SavedNode & {
  status: NodeStatus
  preset?: boolean
}

const PRESET_NODE_CANDIDATES: Array<SavedNode | null> = [
  UIConfig.apiBaseUrl
    ? {
        name: `${UIConfig.networkName} RPC`,
        url: UIConfig.apiBaseUrl,
      }
    : null,
  { name: 'Local node', url: 'http://localhost:8080' },
  { name: 'Primary server', url: 'http://188.245.97.41:8080' },
  { name: 'Secondary server', url: 'http://135.181.145.174:8080' },
]

const PRESET_NODES: SavedNode[] = PRESET_NODE_CANDIDATES
  .filter((node): node is SavedNode => Boolean(node))
  .filter((node, index, array) => array.findIndex((item) => item.url === node.url) === index)

const STORAGE_KEY = 'ippan.ui.customNodes'

function readCustomNodes(): SavedNode[] {
  if (typeof window === 'undefined') {
    return []
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) {
      return []
    }
    const parsed = JSON.parse(raw) as SavedNode[]
    return Array.isArray(parsed) ? parsed : []
  } catch (error) {
    console.warn('Unable to parse stored nodes:', error)
    return []
  }
}

function persistCustomNodes(nodes: SavedNode[]) {
  if (typeof window === 'undefined') {
    return
  }

  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(nodes))
  } catch (error) {
    console.warn('Unable to persist nodes:', error)
  }
}

function normalizeUrl(url: string): string | null {
  const trimmed = url.trim()
  if (!trimmed) {
    return null
  }

  try {
    const withProtocol = /^https?:\/\//i.test(trimmed) ? trimmed : `http://${trimmed}`
    const parsed = new URL(withProtocol)
    parsed.pathname = ''
    parsed.search = ''
    parsed.hash = ''
    return parsed.toString().replace(/\/$/, '')
  } catch (error) {
    console.warn('Invalid node URL provided:', error)
    return null
  }
}

async function probeNode(url: string): Promise<NodeStatus> {
  const normalized = url.replace(/\/$/, '')
  const healthPaths = ['/api/health', '/health']

  for (const path of healthPaths) {
    const controller = new AbortController()
    const timeout = window.setTimeout(() => controller.abort(), 4000)
    try {
      const response = await fetch(`${normalized}${path}`, {
        method: 'GET',
        signal: controller.signal,
      })
      if (response.ok) {
        return 'connected'
      }
    } catch (error) {
      // Try the next path
      continue
    } finally {
      window.clearTimeout(timeout)
    }
  }
  return 'disconnected'
}

export default function NodeSelector() {
  const [customNodes, setCustomNodes] = useState<SavedNode[]>(() => readCustomNodes())
  const [selected, setSelected] = useState(() => normalizeUrl(getApiBaseUrl()) || getApiBaseUrl())
  const [statuses, setStatuses] = useState<Record<string, NodeStatus>>({})
  const [newNodeUrl, setNewNodeUrl] = useState('')
  const [newNodeName, setNewNodeName] = useState('')
  const [formError, setFormError] = useState('')
  const [showReloadHint, setShowReloadHint] = useState(false)

  const nodes = useMemo<DisplayNode[]>(() => {
    const custom = customNodes.map<DisplayNode>((node) => ({
      ...node,
      status: statuses[node.url] || 'checking',
    }))

    const preset = PRESET_NODES.map<DisplayNode>((node) => ({
      ...node,
      status: statuses[node.url] || 'checking',
      preset: true,
    }))

    const merged = [...preset]
    custom.forEach((node) => {
      if (!merged.find((item) => item.url === node.url)) {
        merged.push(node)
      }
    })

    return merged
  }, [customNodes, statuses])

  useEffect(() => {
    let cancelled = false

    const checkAll = async () => {
      const results = await Promise.all(
        nodes.map(async (node) => {
          const status = await probeNode(node.url)
          return [node.url, status] as const
        })
      )

      if (!cancelled) {
        setStatuses((prev) => {
          const next = { ...prev }
          results.forEach(([url, status]) => {
            next[url] = status
          })
          return next
        })
      }
    }

    checkAll()
    const interval = window.setInterval(checkAll, 15000)
    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [nodes])

  const handleSelect = (node: DisplayNode) => {
    setSelected(node.url)
    setApiBaseUrl(node.url)
    setShowReloadHint(true)
    window.setTimeout(() => {
      window.location.reload()
    }, 250)
  }

  const handleRemove = (node: DisplayNode) => {
    if (node.preset) {
      return
    }

    const next = customNodes.filter((item) => item.url !== node.url)
    setCustomNodes(next)
    persistCustomNodes(next)
    setStatuses((prev) => {
      const copy = { ...prev }
      delete copy[node.url]
      return copy
    })
  }

  const handleSave = () => {
    setFormError('')

    const normalized = normalizeUrl(newNodeUrl)
    if (!normalized) {
      setFormError('Enter a valid HTTP or HTTPS URL for the node.')
      return
    }

    const label = newNodeName.trim() || normalized
    const candidate: SavedNode = { name: label, url: normalized }

    const alreadyExists = nodes.some((node) => node.url === candidate.url)
    if (alreadyExists) {
      setFormError('This node is already in the list.')
      return
    }

    const next = [...customNodes, candidate]
    setCustomNodes(next)
    persistCustomNodes(next)
    setNewNodeUrl('')
    setNewNodeName('')
    setFormError('')
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-slate-900">Node selection</h1>
      <p className="text-sm text-slate-600">
        Choose which IPPAN node this interface should query. The selection is saved locally so you can tailor the dashboard to
        your deployment.
      </p>

      <Card title="Available nodes">
        <div className="space-y-3">
          {nodes.map((node) => {
            const isActive = selected === node.url
            const status = node.status || 'checking'
            return (
              <div
                key={node.url}
                className={`flex flex-col gap-3 rounded-lg border p-4 transition-colors ${
                  isActive ? 'border-blue-500 bg-blue-50' : 'border-slate-200 hover:border-slate-300'
                }`}
              >
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <div className="text-base font-semibold text-slate-800">{node.name}</div>
                    <div className="text-sm text-slate-500">{node.url}</div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge
                      variant={
                        status === 'connected' ? 'success' : status === 'disconnected' ? 'error' : 'warning'
                      }
                    >
                      {status}
                    </Badge>
                    {isActive && <Badge variant="blue">Active</Badge>}
                  </div>
                </div>
                <div className="flex flex-wrap items-center gap-3">
                  <Button onClick={() => handleSelect(node)} disabled={isActive}>
                    {isActive ? 'Selected' : 'Use this node'}
                  </Button>
                  {!node.preset && (
                    <Button variant="secondary" onClick={() => handleRemove(node)}>
                      Remove
                    </Button>
                  )}
                </div>
              </div>
            )
          })}
          {nodes.length === 0 && <p className="text-sm text-slate-600">No nodes are configured yet.</p>}
        </div>
      </Card>

      <Card title="Add custom node">
        <div className="space-y-4 text-sm">
          <Field label="Node URL">
            <Input
              placeholder="https://node.example.com:8080"
              value={newNodeUrl}
              onChange={(event) => setNewNodeUrl(event.target.value)}
            />
          </Field>
          <Field label="Display name (optional)">
            <Input placeholder="e.g. Frankfurt validator" value={newNodeName} onChange={(event) => setNewNodeName(event.target.value)} />
          </Field>
          {formError && <p className="text-sm text-red-600">{formError}</p>}
          <Button onClick={handleSave} disabled={!newNodeUrl.trim()}>
            Save node
          </Button>
        </div>
      </Card>

      {showReloadHint && (
        <p className="text-sm text-amber-600">
          Reloading the application to use the updated API endpoint…
        </p>
      )}
    </div>
  )
}
