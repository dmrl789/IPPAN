import { useState, useEffect } from 'react'
import { Card, Button, Field, Input, Badge } from '../components/UI'

interface Domain {
  id: string
  name: string
  tld: string
  status: 'active' | 'expired' | 'pending'
  expiryDate: string
  dnsRecords: Array<{
    type: string
    name: string
    value: string
  }>
}

export default function Domains() {
  const [domains, setDomains] = useState<Domain[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  
  // Domain registration form
  const [registerForm, setRegisterForm] = useState({
    domainName: '',
    tld: 'ippan'
  })

  const [availableTlds] = useState([
    { name: 'ippan', price: '10 IPPAN/year' },
    { name: 'blockchain', price: '25 IPPAN/year' },
    { name: 'crypto', price: '20 IPPAN/year' },
  ])

  useEffect(() => {
    loadDomains()
  }, [])

  const loadDomains = async () => {
    setLoading(true)
    try {
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 1000))
      
      setDomains([
        {
          id: 'domain-1',
          name: 'mycompany',
          tld: 'ippan',
          status: 'active',
          expiryDate: '2024-12-15',
          dnsRecords: [
            { type: 'A', name: '@', value: '192.168.1.1' },
            { type: 'CNAME', name: 'www', value: 'mycompany.ippan' }
          ]
        },
        {
          id: 'domain-2',
          name: 'test',
          tld: 'blockchain',
          status: 'active',
          expiryDate: '2024-11-20',
          dnsRecords: [
            { type: 'A', name: '@', value: '10.0.0.1' }
          ]
        }
      ])
    } catch (err) {
      setError('Failed to load domains')
    } finally {
      setLoading(false)
    }
  }

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError(null)

    try {
      if (!registerForm.domainName || !registerForm.tld) {
        throw new Error('Domain name and TLD are required')
      }

      // Simulate domain registration
      await new Promise(resolve => setTimeout(resolve, 2000))
      
      // Add new domain to list
      const newDomain: Domain = {
        id: `domain-${Date.now()}`,
        name: registerForm.domainName,
        tld: registerForm.tld,
        status: 'pending',
        expiryDate: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString().split('T')[0],
        dnsRecords: []
      }
      
      setDomains(prev => [...prev, newDomain])
      setRegisterForm({ domainName: '', tld: 'ippan' })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Registration failed')
    } finally {
      setLoading(false)
    }
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active': return 'success'
      case 'expired': return 'error'
      case 'pending': return 'warning'
      default: return 'secondary'
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Domain Management</h1>
        <Badge variant="success">{domains.length} Domains</Badge>
      </div>

      {/* Domain Registration */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Register New Domain</h3>
        <form onSubmit={handleRegister} className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <Field label="Domain Name" required>
              <Input
                type="text"
                placeholder="mycompany"
                value={registerForm.domainName}
                onChange={(e) => setRegisterForm(prev => ({ ...prev, domainName: e.target.value }))}
              />
            </Field>

            <Field label="TLD" required>
              <select
                className="w-full p-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                value={registerForm.tld}
                onChange={(e) => setRegisterForm(prev => ({ ...prev, tld: e.target.value }))}
              >
                {availableTlds.map(tld => (
                  <option key={tld.name} value={tld.name}>
                    .{tld.name} - {tld.price}
                  </option>
                ))}
              </select>
            </Field>
          </div>

          <Button 
            type="submit" 
            disabled={loading || !registerForm.domainName}
            className="w-full"
          >
            {loading ? 'Registering...' : 'Register Domain'}
          </Button>
        </form>
      </Card>

      {/* My Domains */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">My Domains</h3>
        {domains.length === 0 ? (
          <div className="text-center py-8 text-gray-500">
            No domains registered yet
          </div>
        ) : (
          <div className="space-y-4">
            {domains.map(domain => (
              <div key={domain.id} className="p-4 border border-gray-200 rounded-lg">
                <div className="flex justify-between items-start mb-3">
                  <div>
                    <div className="font-semibold text-lg">
                      {domain.name}.{domain.tld}
                    </div>
                    <div className="text-sm text-gray-600">
                      Expires: {domain.expiryDate}
                    </div>
                  </div>
                  <Badge variant={getStatusColor(domain.status)}>
                    {domain.status}
                  </Badge>
                </div>
                
                {/* DNS Records */}
                {domain.dnsRecords.length > 0 && (
                  <div className="mt-3">
                    <div className="text-sm font-semibold mb-2">DNS Records:</div>
                    <div className="space-y-1">
                      {domain.dnsRecords.map((record, index) => (
                        <div key={index} className="text-sm font-mono bg-gray-100 p-2 rounded">
                          {record.type} {record.name} {record.value}
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                <div className="flex space-x-2 mt-4">
                  <Button size="sm" variant="outline">
                    Manage DNS
                  </Button>
                  <Button size="sm" variant="outline">
                    Renew
                  </Button>
                  <Button size="sm" variant="outline">
                    Transfer
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Available TLDs */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Available TLDs</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {availableTlds.map(tld => (
            <div key={tld.name} className="p-4 border border-gray-200 rounded-lg text-center">
              <div className="text-2xl font-bold text-blue-600">.{tld.name}</div>
              <div className="text-sm text-gray-600 mt-1">{tld.price}</div>
              <Button size="sm" className="mt-3 w-full">
                Register
              </Button>
            </div>
          ))}
        </div>
      </Card>

      {/* Error Display */}
      {error && (
        <Card className="border-red-200 bg-red-50">
          <div className="text-red-800">
            <strong>Error:</strong> {error}
          </div>
        </Card>
      )}
    </div>
  )
}
