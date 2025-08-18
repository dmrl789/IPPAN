import { useState } from 'react'
import NamePicker from '../components/NamePicker'

type TabType = 'domains' | 'handles'

export default function DomainsPage() {
  const [activeTab, setActiveTab] = useState<TabType>('domains')

  return (
    <div style={{ padding: 16 }}>
      <div className="tabs">
        <button 
          className={activeTab === 'domains' ? 'active' : ''} 
          onClick={() => setActiveTab('domains')}
        >
          Domains
        </button>
        <button 
          className={activeTab === 'handles' ? 'active' : ''} 
          onClick={() => setActiveTab('handles')}
        >
          Handles
        </button>
      </div>
      
      <div style={{ marginTop: 16 }}>
        <NamePicker type={activeTab} />
      </div>
    </div>
  )
}
