import { Route, Routes, Navigate } from 'react-router-dom'
import DomainsPage from '../pages/DomainsPage'

export default function OnChainRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/onchain/payments" replace />} />
      <Route path="payments" element={<div style={{padding:16}}>Payments & M2M</div>} />
      <Route path="m2m" element={<div style={{padding:16}}>M2M Channels</div>} />
      <Route path="staking" element={<div style={{padding:16}}>Staking / Validator</div>} />
      <Route path="domains" element={<DomainsPage />} />
      <Route path="anchors" element={<div style={{padding:16}}>Anchors / L2</div>} />
      <Route path="rounds" element={<div style={{padding:16}}>Rounds & Finality</div>} />
    </Routes>
  )
}
