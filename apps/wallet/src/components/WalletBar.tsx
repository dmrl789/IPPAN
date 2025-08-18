import { useState } from 'react'
import TxComposerModal from './composer/TxComposerModal'

export default function WalletBar() {
  const [open, setOpen] = useState(false)
  return (
    <header className="wallet-bar">
      <div className="left">IPPAN Wallet</div>
      <div className="right">
        <button onClick={() => setOpen(true)}>New Transaction (Ctrl+K)</button>
      </div>
      {open && <TxComposerModal onClose={() => setOpen(false)} />}
    </header>
  )
}
