import { useEffect, useMemo, useState } from 'react'
import { Routes, Route, NavLink, Navigate } from 'react-router-dom'
import WalletOverview from './pages/WalletOverview'
import WalletPage from './pages/WalletPage'
import PaymentsPage from './pages/PaymentsPage'
import PaymentsM2M from './pages/PaymentsM2M'
import DashboardPage from './pages/DashboardPage'
import StakingPage from './pages/StakingPage'
import StakingValidator from './pages/StakingValidator'
import DomainsPage from './pages/DomainsPage'
import DomainUpdatesPage from './pages/DomainUpdatesPage'
import InteroperabilityPage from './pages/InteroperabilityPage'
import StoragePage from './pages/StoragePage'
import FileAvailabilityPage from './pages/FileAvailabilityPage'
import NeuralModels from './pages/NeuralModels'
import ModelsPage from './pages/ModelsPage'
import DatasetsPage from './pages/DatasetsPage'
import InferencePage from './pages/InferencePage'
import BidsPage from './pages/BidsPage'
import ProofsPage from './pages/ProofsPage'
import LiveBlocksPage from './pages/explorer/LiveBlocksPage'
import TransactionsPage from './pages/explorer/TransactionsPage'
import AccountsPage from './pages/explorer/AccountsPage'
import ContractsPage from './pages/explorer/ContractsPage'
import ValidatorsPage from './pages/explorer/ValidatorsPage'
import NetworkMapPage from './pages/explorer/NetworkMapPage'
import AnalyticsPage from './pages/explorer/AnalyticsPage'
import NodeSelector from './components/NodeSelector'
import { getApiBaseUrl, getHealth } from './lib/api'
import { UIConfig } from './lib/config'

const navigation = [
  {
    title: 'Overview',
    items: [{ name: 'Node Dashboard', path: '/dashboard', icon: '📊' }],
  },
  {
    title: 'Wallet & Payments',
    items: [
      { name: 'Wallet Control Center', path: '/wallet', icon: '💼' },
      { name: 'Wallet Playground', path: '/wallet/legacy', icon: '🧪' },
      { name: 'Payments', path: '/wallet/payments', icon: '💸' },
      { name: 'Machine Payments', path: '/wallet/m2m', icon: '🤖' },
    ],
  },
  {
    title: 'Staking & Governance',
    items: [
      { name: 'Staking', path: '/staking', icon: '🪙' },
      { name: 'Validator Ops', path: '/staking/validators', icon: '🛠️' },
    ],
  },
  {
    title: 'Domains & Interop',
    items: [
      { name: 'Domain Manager', path: '/domains', icon: '🌐' },
      { name: 'Domain Updates', path: '/domains/updates', icon: '📰' },
      { name: 'Interoperability', path: '/domains/interoperability', icon: '🔗' },
    ],
  },
  {
    title: 'Storage & Data',
    items: [
      { name: 'Storage Control', path: '/storage', icon: '🗄️' },
      { name: 'File Availability', path: '/storage/availability', icon: '🧾' },
    ],
  },
  {
    title: 'Neural Network',
    items: [
      { name: 'Control Center', path: '/neural', icon: '🧠' },
      { name: 'Models API', path: '/neural/models', icon: '📚' },
      { name: 'Datasets', path: '/neural/datasets', icon: '🧬' },
      { name: 'Inference', path: '/neural/inference', icon: '⚙️' },
      { name: 'Bids & Auctions', path: '/neural/bids', icon: '🏅' },
      { name: 'Proofs', path: '/neural/proofs', icon: '✅' },
    ],
  },
  {
    title: 'Explorer',
    items: [
      { name: 'Live Blocks', path: '/explorer/live-blocks', icon: '🧱' },
      { name: 'Transactions', path: '/explorer/transactions', icon: '💳' },
      { name: 'Accounts', path: '/explorer/accounts', icon: '👤' },
      { name: 'Contracts', path: '/explorer/contracts', icon: '📜' },
      { name: 'Validators', path: '/explorer/validators', icon: '🛡️' },
      { name: 'Network Map', path: '/explorer/network-map', icon: '🗺️' },
      { name: 'Analytics', path: '/explorer/analytics', icon: '📈' },
    ],
  },
  {
    title: 'Operations',
    items: [{ name: 'Node Selector', path: '/operations/node-selector', icon: '🛰️' }],
  },
] as const

type HealthStatus = 'checking' | 'online' | 'offline'

export default function App() {
  const [health, setHealth] = useState<HealthStatus>('checking')
  const apiBaseUrl = useMemo(() => getApiBaseUrl(), [])

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

              <Route path="/wallet" element={<WalletOverview />} />
              <Route path="/wallet/legacy" element={<WalletPage />} />
              <Route path="/wallet/payments" element={<PaymentsPage />} />
              <Route path="/wallet/m2m" element={<PaymentsM2M />} />

              <Route path="/staking" element={<StakingPage />} />
              <Route path="/staking/validators" element={<StakingValidator />} />

              <Route path="/domains" element={<DomainsPage />} />
              <Route path="/domains/updates" element={<DomainUpdatesPage />} />
              <Route path="/domains/interoperability" element={<InteroperabilityPage />} />

              <Route path="/storage" element={<StoragePage />} />
              <Route path="/storage/availability" element={<FileAvailabilityPage />} />

              <Route path="/neural" element={<NeuralModels />} />
              <Route path="/neural/models" element={<ModelsPage />} />
              <Route path="/neural/datasets" element={<DatasetsPage />} />
              <Route path="/neural/inference" element={<InferencePage />} />
              <Route path="/neural/bids" element={<BidsPage />} />
              <Route path="/neural/proofs" element={<ProofsPage />} />

              <Route path="/explorer/live-blocks" element={<LiveBlocksPage />} />
              <Route path="/explorer/transactions" element={<TransactionsPage />} />
              <Route path="/explorer/accounts" element={<AccountsPage />} />
              <Route path="/explorer/contracts" element={<ContractsPage />} />
              <Route path="/explorer/validators" element={<ValidatorsPage />} />
              <Route path="/explorer/network-map" element={<NetworkMapPage />} />
              <Route path="/explorer/analytics" element={<AnalyticsPage />} />

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
