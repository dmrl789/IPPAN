import { useEffect, useMemo, useState } from 'react'

function calcYearFee(year: number): number {
  if (year === 1) return 0.20
  if (year === 2) return 0.02
  const decayed = 0.01 - 0.001 * (year - 3)
  return Math.max(decayed, 0.001)
}

export default function DomainForm() {
  const [domain, setDomain] = useState('example.ipn')
  const [years, setYears] = useState(1)
  const [multiplier, setMultiplier] = useState(1)

  // naive tld parse
  const tld = useMemo(() => {
    const parts = domain.split('.')
    return parts.length >= 2 ? parts[parts.length - 1] : 'ipn'
  }, [domain])

  useEffect(() => {
    // simple mapping aligned with PRD: .ai/.m x10, .iot x2, else x1
    if (tld === 'ai' || tld === 'm') setMultiplier(10)
    else if (tld === 'iot') setMultiplier(2)
    else setMultiplier(1)
  }, [tld])

  const total = useMemo(() => {
    let sum = 0
    for (let y = 1; y <= years; y++) sum += calcYearFee(y) * multiplier
    return sum
  }, [years, multiplier])

  return (
    <form className="form">
      <label>
        Domain
        <input value={domain} onChange={e => setDomain(e.target.value)} />
      </label>
      <label>
        Years
        <input type="number" min={1} max={20} value={years} onChange={e => setYears(parseInt(e.target.value||'1',10))} />
      </label>
      <div className="fee">Total fee: {total.toFixed(6)} IPN (multiplier x{multiplier})</div>
      <div className="actions">
        <button type="button">Sign & Broadcast</button>
      </div>
    </form>
  )
}
