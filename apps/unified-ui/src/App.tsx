import { useEffect, useMemo, useState } from 'react'
import { Routes, Route, NavLink, Navigate } from 'react-router-dom'
import DashboardPage from './pages/DashboardPage'
import LiveBlocksPage from './pages/explorer/LiveBlocksPage'
import TransactionsPage from './pages/explorer/TransactionsPage'
import ValidatorsPage from './pages/explorer/ValidatorsPage'
import NodeSelector from './components/NodeSelector'
import { getApiBaseUrl, getHealth } from './lib/api'
import { UIConfig } from './lib/config'

type NavigationGroup = {
  title: string
  items: ReadonlyArray<{ name: string; path: string; icon: string }>
}

const FULL_NAVIGATION: ReadonlyArray<NavigationGroup> = [
  {
    title: 'Overview',
    items: [{ name: 'Node Dashboard', path: '/dashboard', icon: '📊' }],
  },
  {
    title: 'Explorer',
    items: [
      { name: 'Live Blocks', path: '/explorer/live-blocks', icon: '🧱' },
      { name: 'Transactions', path: '/explorer/transactions', icon: '💳' },
      { name: 'Validators', path: '/explorer/validators', icon: '🛡️' },
    ],
  },
  {
    title: 'Operations',
    items: [{ name: 'Node Selector', path: '/operations/node-selector', icon: '🛰️' }],
  },
]

const MINIMAL_NAVIGATION: ReadonlyArray<NavigationGroup> = [
  {
    title: 'Overview',
    items: [
      { name: 'Node Dashboard', path: '/dashboard', icon: '📊' },
      { name: 'Node Selector', path: '/operations/node-selector', icon: '🛰️' },
    ],
  },
]

type HealthStatus = 'checking' | 'online' | 'offline'

export default function App() {
  const [health, setHealth] = useState<HealthStatus>('checking')
  const apiBaseUrl = useMemo(() => getApiBaseUrl(), [])
  const navigation = UIConfig.enableFullUi ? FULL_NAVIGATION : MINIMAL_NAVIGATION

  useEffect(() => {
    let cancelled = false

    const checkHealth = async () => {
      try {
        await getHealth()
        if (!cancelled) {
          setHealth('online')
        }
      } catch (error) {
        console.warn('Health check failed:', error)
        if (!cancelled) {
          setHealth('offline')
        }
      }
    }

    checkHealth()
    const interval = window.setInterval(checkHealth, 15000)

    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [])

  return (
    <div className="app bg-slate-50 text-slate-900">
      <header className="header">
        <div className="flex items-center space-x-4">
          <div>
            <h1 className="text-xl font-bold">IPPAN Operations Console</h1>
            <div className="text-xs uppercase tracking-wide text-white/60">
              {UIConfig.networkName}
            </div>
          </div>
          <span className="text-sm text-white/80" title="Active RPC base URL">
            {apiBaseUrl}
          </span>
        </div>
        <div className="flex items-center space-x-2 text-sm">
          <span
            className={`inline-flex items-center gap-2 rounded-full px-3 py-1 font-medium ${
              health === 'online'
                ? 'bg-emerald-500/20 text-white'
                : health === 'offline'
                ? 'bg-red-500/30 text-white'
                : 'bg-slate-700/50 text-white'
            }`}
          >
            <span className={`h-2.5 w-2.5 rounded-full ${health === 'online' ? 'bg-emerald-400' : health === 'offline' ? 'bg-red-400' : 'bg-amber-300'}`} />
            {health === 'checking' ? 'Checking node health…' : health === 'online' ? 'Node online' : 'Node unreachable'}
          </span>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        <aside className="sidebar">
          <nav className="p-4">
            {navigation.map((group) => (
              <div key={group.title} className="nav-group">
                <div className="nav-group-title">{group.title}</div>
                {group.items.map((item) => (
                  <NavLink
                    key={item.path}
                    to={item.path}
                    end
                    className={({ isActive }) =>
                      `nav-item flex items-center space-x-3 ${isActive ? 'active' : ''}`
                    }
                  >
                    <span className="text-lg" aria-hidden>
                      {item.icon}
                    </span>
                    <span>{item.name}</span>
                  </NavLink>
                ))}
              </div>
            ))}
          </nav>
        </aside>

        <main className="main-content">
          <div className="p-6">
            <Routes>
              <Route path="/" element={<Navigate to="/dashboard" replace />} />
              <Route path="/dashboard" element={<DashboardPage />} />

              <Route path="/explorer/live-blocks" element={<LiveBlocksPage />} />
              <Route path="/explorer/transactions" element={<TransactionsPage />} />
              <Route path="/explorer/validators" element={<ValidatorsPage />} />

              <Route path="/operations/node-selector" element={<NodeSelector />} />
              <Route path="/node-selector" element={<NodeSelector />} />

              <Route path="*" element={<Navigate to="/dashboard" replace />} />
            </Routes>
          </div>
        </main>
      </div>
    </div>
  )
}
