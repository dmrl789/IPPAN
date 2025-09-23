import { useEffect, useState } from 'react'
import { Routes, Route, NavLink, Navigate } from 'react-router-dom'
import { Button } from './components/UI'
import DomainsPage from './pages/DomainsPage'
import StoragePage from './pages/StoragePage'
import WalletOverview from './pages/WalletOverview'
import StakingValidator from './pages/StakingValidator'
import NeuralModels from './pages/NeuralModels'
import PaymentsM2M from './pages/PaymentsM2M'
import BidsPage from './pages/BidsPage'
import DatasetsPage from './pages/DatasetsPage'
import InferencePage from './pages/InferencePage'
import ProofsPage from './pages/ProofsPage'
import AccountsPage from './pages/explorer/AccountsPage'
import AnalyticsPage from './pages/explorer/AnalyticsPage'
import ContractsPage from './pages/explorer/ContractsPage'
import LiveBlocksPage from './pages/explorer/LiveBlocksPage'
import NetworkMapPage from './pages/explorer/NetworkMapPage'
import TransactionsPage from './pages/explorer/TransactionsPage'
import ValidatorsPage from './pages/explorer/ValidatorsPage'
import WalletPage from './pages/WalletPage'
import InteroperabilityPage from './pages/InteroperabilityPage'
import FileAvailabilityPage from './pages/FileAvailabilityPage'
import DomainUpdatesPage from './pages/DomainUpdatesPage'
import NodeSelector from './components/NodeSelector'

// Local storage helpers (same as WalletOverview)
const LS_ADDR = "ippan.wallet.address";
const LS_TYPE = "ippan.wallet.type";

function loadAddress(): string | null { 
  return localStorage.getItem(LS_ADDR);
}

function loadType(): "watch-only" | "local" { 
  return (localStorage.getItem(LS_TYPE) as "watch-only" | "local") || "watch-only";
}

export default function App() {
  // Wallet connection state - read from localStorage like WalletOverview
  const [walletAddress, setWalletAddress] = useState<string | null>(loadAddress());
  const [walletType, setWalletType] = useState<"watch-only" | "local">(loadType());
  const [walletBalance, setWalletBalance] = useState<string | null>(null);
  
  // Derived state
  const walletConnected = !!walletAddress;

  // Listen for localStorage changes to sync wallet state
  useEffect(() => {
    const handleStorageChange = () => {
      const newAddress = loadAddress();
      const newType = loadType();
      setWalletAddress(newAddress);
      setWalletType(newType);
    };

    // Listen for storage events (when localStorage changes in other tabs/components)
    window.addEventListener('storage', handleStorageChange);
    
    // Also listen for custom events (for same-tab updates)
    window.addEventListener('walletStateChanged', handleStorageChange);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
      window.removeEventListener('walletStateChanged', handleStorageChange);
    };
  }, []);

  // Real wallet provider setup
  useEffect(() => {
    // Create real wallet provider if not exists
    if (!window.ippan) {
      window.ippan = {
        connect: async () => {
          // For now, return a default address - in a real app this would connect to a wallet
          return {
            address: "i0000000000000000000000000000000000000000000000000000000000000000",
            balance: "0.0" // Will be fetched from blockchain
          };
        },
        signMessage: async (message: string) => {
          // In a real implementation, this would sign with the user's private key
          // For now, return a placeholder
          return 'real_signature_' + btoa(message).slice(0, 32);
        },
        getAddress: async () => {
          return "i0000000000000000000000000000000000000000000000000000000000000000";
        }
      };
    }
  }, []);

  // Wallet connection functions (now handled by WalletOverview component)
  const connectWallet = async () => {
    // This function is no longer used as wallet connection is handled by WalletOverview
    console.log('Wallet connection is handled by the Wallet Overview page');
  };

  const disconnectWallet = () => {
    // This function is no longer used as wallet disconnection is handled by WalletOverview
    console.log('Wallet disconnection is handled by the Wallet Overview page');
  };

  const navigation = [
    {
      title: "Wallet & Finance",
      items: [
        { name: "Wallet Overview", path: "/wallet", icon: "💰" },
        { name: "Payments & M2M", path: "/payments", icon: "💳" },
        { name: "Staking & Validator", path: "/staking", icon: "🔒" },
        { name: "Domain Management", path: "/domains", icon: "🌐" },
        { name: "Domain & DNS Updates", path: "/domain-updates", icon: "📋" },
        { name: "Storage", path: "/storage", icon: "📁" },
        { name: "File Availability", path: "/availability", icon: "📊" },
      ]
    },
    {
      title: "Blockchain Explorer",
      items: [
        { name: "Live Blocks", path: "/explorer/live-blocks", icon: "⛓️" },
        { name: "Transactions", path: "/explorer/transactions", icon: "📜" },
        { name: "Accounts", path: "/explorer/accounts", icon: "👤" },
        { name: "Validators", path: "/explorer/validators", icon: "🛡️" },
        { name: "Interoperability", path: "/interoperability", icon: "🔗" },
        { name: "Network Map", path: "/explorer/network-map", icon: "🌍" },
        { name: "Analytics", path: "/explorer/analytics", icon: "📊" },
      ]
    },
    {
      title: "Neural Network",
      items: [
        { name: "Models", path: "/models", icon: "🧠" },
        { name: "Datasets", path: "/datasets", icon: "📊" },
        { name: "Inference", path: "/inference", icon: "⚡" },
        { name: "Bids & Winners", path: "/bids", icon: "🏆" },
        { name: "Proofs", path: "/proofs", icon: "🔍" },
      ]
    },
    {
      title: "Node Management",
      items: [
        { name: "Node Selection", path: "/node-selector", icon: "🔧" },
      ]
    }
  ]

  return (
    <div className="app">
      {/* Header */}
      <header className="header">
        <div className="flex items-center space-x-4">
          <h1 className="text-xl font-bold">IPPAN Unified Interface</h1>
        </div>
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-1">
            <div className={`w-4 h-4 rounded-full ${walletConnected ? 'bg-green-500' : 'bg-gray-400'}`}></div>
            <div className={`w-4 h-4 rounded-full ${!walletConnected ? 'bg-red-500' : 'bg-gray-400'}`}></div>
          </div>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <aside className="sidebar w-72">
          <nav className="p-4">
            {navigation.map((group) => (
              <div key={group.title} className="nav-group">
                <div className="nav-group-title">{group.title}</div>
                {group.items.map((item) => (
                  <NavLink
                    key={item.path}
                    to={item.path}
                    className={({ isActive }) =>
                      `nav-item flex items-center space-x-3 ${isActive ? 'active' : ''}`
                    }
                  >
                    <span className="text-lg">{item.icon}</span>
                    <span>{item.name}</span>
                  </NavLink>
                ))}
              </div>
            ))}
          </nav>
        </aside>

        {/* Main Content */}
        <main className="main-content">
          <div className="p-6">
            <Routes>
              <Route path="/" element={<Navigate to="/wallet" replace />} />
              
              {/* Wallet & Finance Routes */}
              <Route path="/wallet" element={<WalletOverview />} />
              <Route path="/payments" element={<PaymentsM2M walletAddress={walletAddress} walletConnected={walletConnected} />} />
              <Route path="/staking" element={<StakingValidator />} />
              <Route path="/domains" element={<DomainsPage />} />
              <Route path="/domain-updates" element={<DomainUpdatesPage />} />
              <Route path="/storage" element={<StoragePage />} />
              
              {/* Blockchain Explorer Routes */}
              <Route path="/explorer/live-blocks" element={<LiveBlocksPage />} />
              <Route path="/explorer/transactions" element={<TransactionsPage />} />
              <Route path="/explorer/accounts" element={<AccountsPage />} />
              <Route path="/explorer/validators" element={<ValidatorsPage />} />
              <Route path="/explorer/contracts" element={<ContractsPage />} />
              <Route path="/explorer/network-map" element={<NetworkMapPage />} />
              <Route path="/explorer/analytics" element={<AnalyticsPage />} />
              
              {/* Interoperability Route */}
              <Route path="/interoperability" element={<InteroperabilityPage />} />
              
              {/* File Availability Route */}
              <Route path="/availability" element={<FileAvailabilityPage />} />
              
              {/* Neural Network Routes */}
              <Route path="/models" element={<NeuralModels />} />
              <Route path="/datasets" element={<DatasetsPage />} />
              <Route path="/inference" element={<InferencePage />} />
              <Route path="/bids" element={<BidsPage />} />
              <Route path="/proofs" element={<ProofsPage />} />
              
              {/* Node Management Routes */}
              <Route path="/node-selector" element={<NodeSelector />} />
            </Routes>
          </div>
        </main>
      </div>
    </div>
  )
}
