import { useState } from 'react'

export default function StakingForm() {
  const [amount, setAmount] = useState('')
  return (
    <form className="form">
      <label>
        Amount (IPN)
        <input value={amount} onChange={e => setAmount(e.target.value)} placeholder="10" />
      </label>
      <div className="actions">
        <button type="button">Sign & Broadcast</button>
      </div>
    </form>
  )
}
