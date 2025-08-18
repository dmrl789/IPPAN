import { Link, Route, Routes, Navigate } from 'react-router-dom'
import WalletBar from './components/WalletBar'
import CommandPalette from './components/CommandPalette'
import OnChainRoutes from './routes/OnChainRoutes'
import StorageRoutes from './routes/StorageRoutes'

export default function App() {
  return (
    <div className="app">
      <WalletBar />
      <nav className="tabs">
        <Link to="/wallet">Wallet</Link>
        <Link to="/onchain/payments">On-Chain</Link>
        <Link to="/storage/upload">Storage</Link>
      </nav>
      <Routes>
        <Route path="/" element={<Navigate to="/wallet" replace />} />
        <Route path="/wallet" element={<div style={{padding:16}}>Balances, keys, activity. Press Ctrl+K to open Composer.</div>} />
        <Route path="/onchain/*" element={<OnChainRoutes />} />
        <Route path="/storage/*" element={<StorageRoutes />} />
      </Routes>
      <CommandPalette />
    </div>
  )
}
