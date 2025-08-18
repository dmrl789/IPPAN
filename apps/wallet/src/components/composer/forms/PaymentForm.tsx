import { useState, useMemo } from 'react'

export default function PaymentForm() {
  const [to, setTo] = useState('')
  const [amount, setAmount] = useState('')
  const [memo, setMemo] = useState('')

  const fee = useMemo(() => {
    const val = parseFloat(amount || '0')
    if (!isFinite(val) || val <= 0) return 0
    const onePct = Math.floor(val * 0.01 * 1e8) / 1e8
    return Math.max(onePct, 0.00000001)
  }, [amount])

  return (
    <form className="form">
      <label>
        To
        <input value={to} onChange={e => setTo(e.target.value)} placeholder="@alice.ipn or address" />
      </label>
      <label>
        Amount (IPN)
        <input value={amount} onChange={e => setAmount(e.target.value)} placeholder="1.25" />
      </label>
      <label>
        Memo
        <input value={memo} onChange={e => setMemo(e.target.value)} placeholder="optional" />
      </label>
      <div className="fee">
        Fee (1%): {fee.toFixed(8)} IPN → Global Fund
      </div>
      <div className="actions">
        <button type="button">Sign & Broadcast</button>
      </div>
    </form>
  )
}
