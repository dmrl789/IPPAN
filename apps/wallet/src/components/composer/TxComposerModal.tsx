import { useComposerStore } from '../../stores/composerStore'
import PaymentForm from './forms/PaymentForm'
import DomainForm from './forms/DomainForm'
import StakingForm from './forms/StakingForm'

export default function TxComposerModal({ onClose }: { onClose: () => void }) {
  const { isOpen, type, close, setType } = useComposerStore()
  if (!isOpen) return null
  return (
    <div className="modal-backdrop" onClick={close}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h3>Transaction Composer</h3>
          <select value={type} onChange={e => setType(e.target.value as any)}>
            <option value="payment">Payment (0x01)</option>
            <option value="storage">Storage (0x02)</option>
            <option value="domain">Domain (0x03)</option>
            <option value="staking">Staking (0x04)</option>
            <option value="anchor">Anchor (0x05)</option>
            <option value="m2m">M2M (0x06)</option>
            <option value="l2_settlement">L2 Settlement (0x07)</option>
            <option value="l2_data">L2 Data (0x08)</option>
          </select>
          <button onClick={onClose}>Close</button>
        </div>
        <div className="modal-body">
          {type === 'payment' && <PaymentForm />}
          {type === 'domain' && <DomainForm />}
          {type === 'staking' && <StakingForm />}
          {type !== 'payment' && type !== 'domain' && type !== 'staking' && (
            <div>Form coming soon…</div>
          )}
        </div>
      </div>
    </div>
  )
}
