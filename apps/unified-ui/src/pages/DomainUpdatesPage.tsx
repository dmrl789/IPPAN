import React, { useState, useEffect } from 'react';
import { Card, Button, Badge, Input, Select } from '../components/UI';

// Types for domain and DNS updates
type UpdateType = 'registration' | 'renewal' | 'transfer' | 'dns_change' | 'expiration' | 'tld_update';

type DomainUpdate = {
  id: string;
  type: UpdateType;
  domain: string;
  description: string;
  timestamp: string;
  status: 'pending' | 'completed' | 'failed';
  tx_hash?: string;
  block_height?: number;
  details?: Record<string, any>;
};

type DNSUpdate = {
  id: string;
  domain: string;
  record_type: string;
  record_name: string;
  old_value?: string;
  new_value: string;
  timestamp: string;
  status: 'pending' | 'completed' | 'failed';
  tx_hash?: string;
  block_height?: number;
};

type TLDUpdate = {
  id: string;
  tld: string;
  type: 'new_tld' | 'price_change' | 'availability_change';
  description: string;
  timestamp: string;
  old_price?: number;
  new_price?: number;
  status: 'active' | 'inactive';
};

// Mock data generation
const generateMockUpdates = (): { domainUpdates: DomainUpdate[], dnsUpdates: DNSUpdate[], tldUpdates: TLDUpdate[] } => {
  const domainUpdates: DomainUpdate[] = [
    {
      id: '1',
      type: 'registration',
      domain: 'alice.ipn',
      description: 'New domain registered',
      timestamp: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
      status: 'completed',
      tx_hash: '0x1234567890abcdef1234567890abcdef12345678',
      block_height: 12345,
      details: { duration_years: 1, fee_paid: 1000000 }
    },
    {
      id: '2',
      type: 'renewal',
      domain: 'bob.ai',
      description: 'Domain renewed for 2 years',
      timestamp: new Date(Date.now() - 4 * 60 * 60 * 1000).toISOString(),
      status: 'completed',
      tx_hash: '0xabcdef1234567890abcdef1234567890abcdef12',
      block_height: 12340,
      details: { duration_years: 2, fee_paid: 2000000 }
    },
    {
      id: '3',
      type: 'transfer',
      domain: 'charlie.iot',
      description: 'Domain ownership transferred',
      timestamp: new Date(Date.now() - 6 * 60 * 60 * 1000).toISOString(),
      status: 'pending',
      tx_hash: '0x9876543210fedcba9876543210fedcba98765432',
      block_height: 12335,
      details: { from: 'i1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa', to: 'i1B2zP2eP6QGefi3DMPTfTL6SLmv8DivfNb' }
    },
    {
      id: '4',
      type: 'expiration',
      domain: 'david.fin',
      description: 'Domain expires in 30 days',
      timestamp: new Date(Date.now() - 8 * 60 * 60 * 1000).toISOString(),
      status: 'pending',
      details: { expires_in_days: 30 }
    }
  ];

  const dnsUpdates: DNSUpdate[] = [
    {
      id: '1',
      domain: 'alice.ipn',
      record_type: 'A',
      record_name: '@',
      old_value: '192.168.1.1',
      new_value: '192.168.1.2',
      timestamp: new Date(Date.now() - 1 * 60 * 60 * 1000).toISOString(),
      status: 'completed',
      tx_hash: '0x1111111111111111111111111111111111111111',
      block_height: 12344
    },
    {
      id: '2',
      domain: 'bob.ai',
      record_type: 'CNAME',
      record_name: 'www',
      new_value: 'bob.ai',
      timestamp: new Date(Date.now() - 3 * 60 * 60 * 1000).toISOString(),
      status: 'completed',
      tx_hash: '0x2222222222222222222222222222222222222222',
      block_height: 12341
    },
    {
      id: '3',
      domain: 'charlie.iot',
      record_type: 'TXT',
      record_name: '@',
      new_value: 'v=spf1 include:_spf.google.com ~all',
      timestamp: new Date(Date.now() - 5 * 60 * 60 * 1000).toISOString(),
      status: 'pending',
      tx_hash: '0x3333333333333333333333333333333333333333',
      block_height: 12336
    }
  ];

  const tldUpdates: TLDUpdate[] = [
    {
      id: '1',
      tld: '.ai',
      type: 'price_change',
      description: 'Premium TLD price updated',
      timestamp: new Date(Date.now() - 12 * 60 * 60 * 1000).toISOString(),
      old_price: 10000000,
      new_price: 12000000,
      status: 'active'
    },
    {
      id: '2',
      tld: '.iot',
      type: 'availability_change',
      description: 'New TLD available for registration',
      timestamp: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(),
      new_price: 2000000,
      status: 'active'
    },
    {
      id: '3',
      tld: '.fin',
      type: 'new_tld',
      description: 'New premium TLD launched',
      timestamp: new Date(Date.now() - 48 * 60 * 60 * 1000).toISOString(),
      new_price: 15000000,
      status: 'active'
    }
  ];

  return { domainUpdates, dnsUpdates, tldUpdates };
};

export default function DomainUpdatesPage() {
  const [activeTab, setActiveTab] = useState<'domain' | 'dns' | 'tld'>('domain');
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());

  // Mock data
  const [domainUpdates, setDomainUpdates] = useState<DomainUpdate[]>([]);
  const [dnsUpdates, setDnsUpdates] = useState<DNSUpdate[]>([]);
  const [tldUpdates, setTldUpdates] = useState<TLDUpdate[]>([]);

  // Load initial data
  useEffect(() => {
    const { domainUpdates: domainData, dnsUpdates: dnsData, tldUpdates: tldData } = generateMockUpdates();
    setDomainUpdates(domainData);
    setDnsUpdates(dnsData);
    setTldUpdates(tldData);
  }, []);

  // Auto-refresh simulation
  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      // Simulate new updates occasionally
      if (Math.random() < 0.1) { // 10% chance of new update
        const { domainUpdates: domainData, dnsUpdates: dnsData, tldUpdates: tldData } = generateMockUpdates();
        setDomainUpdates(domainData);
        setDnsUpdates(dnsData);
        setTldUpdates(tldData);
        setLastUpdate(new Date());
      }
    }, 30000); // Check every 30 seconds

    return () => clearInterval(interval);
  }, [autoRefresh]);

  // Filter functions
  const filterDomainUpdates = (updates: DomainUpdate[]) => {
    return updates.filter(update => {
      const matchesSearch = update.domain.toLowerCase().includes(searchQuery.toLowerCase()) ||
                           update.description.toLowerCase().includes(searchQuery.toLowerCase());
      const matchesStatus = statusFilter === 'all' || update.status === statusFilter;
      const matchesType = typeFilter === 'all' || update.type === typeFilter;
      return matchesSearch && matchesStatus && matchesType;
    });
  };

  const filterDnsUpdates = (updates: DNSUpdate[]) => {
    return updates.filter(update => {
      const matchesSearch = update.domain.toLowerCase().includes(searchQuery.toLowerCase()) ||
                           update.record_name.toLowerCase().includes(searchQuery.toLowerCase());
      const matchesStatus = statusFilter === 'all' || update.status === statusFilter;
      return matchesSearch && matchesStatus;
    });
  };

  const filterTldUpdates = (updates: TLDUpdate[]) => {
    return updates.filter(update => {
      const matchesSearch = update.tld.toLowerCase().includes(searchQuery.toLowerCase()) ||
                           update.description.toLowerCase().includes(searchQuery.toLowerCase());
      const matchesType = typeFilter === 'all' || update.type === typeFilter;
      return matchesSearch && matchesType;
    });
  };

  // Helper functions
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'completed': return 'bg-green-100 text-green-800';
      case 'pending': return 'bg-yellow-100 text-yellow-800';
      case 'failed': return 'bg-red-100 text-red-800';
      case 'active': return 'bg-blue-100 text-blue-800';
      case 'inactive': return 'bg-gray-100 text-gray-800';
      default: return 'bg-gray-100 text-gray-800';
    }
  };

  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'registration': return '🆕';
      case 'renewal': return '🔄';
      case 'transfer': return '↔️';
      case 'dns_change': return '🌐';
      case 'expiration': return '⏰';
      case 'tld_update': return '📋';
      case 'new_tld': return '✨';
      case 'price_change': return '💰';
      case 'availability_change': return '🔓';
      default: return '📄';
    }
  };

  const formatTimeAgo = (timestamp: string) => {
    const now = new Date();
    const time = new Date(timestamp);
    const diffMs = now.getTime() - time.getTime();
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffHours / 24);

    if (diffDays > 0) return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`;
    if (diffHours > 0) return `${diffHours} hour${diffHours > 1 ? 's' : ''} ago`;
    return 'Just now';
  };

  const renderDomainUpdates = () => {
    const filtered = filterDomainUpdates(domainUpdates);
    
    return (
      <div className="space-y-4">
        {filtered.map(update => (
          <div key={update.id} className="border rounded-lg p-4 hover:bg-gray-50">
            <div className="flex items-start justify-between">
              <div className="flex items-start space-x-3">
                <span className="text-2xl">{getTypeIcon(update.type)}</span>
                <div className="flex-1">
                  <div className="flex items-center space-x-2">
                    <h3 className="font-medium text-gray-900">{update.domain}</h3>
                    <Badge className={getStatusColor(update.status)}>
                      {update.status}
                    </Badge>
                  </div>
                  <p className="text-sm text-gray-600 mt-1">{update.description}</p>
                  <div className="flex items-center space-x-4 mt-2 text-xs text-gray-500">
                    <span>{formatTimeAgo(update.timestamp)}</span>
                    {update.tx_hash && (
                      <span>TX: {update.tx_hash.slice(0, 8)}...{update.tx_hash.slice(-8)}</span>
                    )}
                    {update.block_height && (
                      <span>Block: {update.block_height}</span>
                    )}
                  </div>
                  {update.details && (
                    <div className="mt-2 text-xs text-gray-500">
                      {Object.entries(update.details).map(([key, value]) => (
                        <span key={key} className="mr-4">
                          {key}: {String(value)}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        ))}
        {filtered.length === 0 && (
          <div className="text-center py-8 text-gray-500">
            No domain updates found matching your criteria.
          </div>
        )}
      </div>
    );
  };

  const renderDnsUpdates = () => {
    const filtered = filterDnsUpdates(dnsUpdates);
    
    return (
      <div className="space-y-4">
        {filtered.map(update => (
          <div key={update.id} className="border rounded-lg p-4 hover:bg-gray-50">
            <div className="flex items-start justify-between">
              <div className="flex items-start space-x-3">
                <span className="text-2xl">🌐</span>
                <div className="flex-1">
                  <div className="flex items-center space-x-2">
                    <h3 className="font-medium text-gray-900">{update.domain}</h3>
                    <Badge className="bg-blue-100 text-blue-800">
                      {update.record_type}
                    </Badge>
                    <Badge className={getStatusColor(update.status)}>
                      {update.status}
                    </Badge>
                  </div>
                  <p className="text-sm text-gray-600 mt-1">
                    {update.record_name} → {update.new_value}
                  </p>
                  {update.old_value && (
                    <p className="text-xs text-gray-500 mt-1">
                      Previous: {update.old_value}
                    </p>
                  )}
                  <div className="flex items-center space-x-4 mt-2 text-xs text-gray-500">
                    <span>{formatTimeAgo(update.timestamp)}</span>
                    {update.tx_hash && (
                      <span>TX: {update.tx_hash.slice(0, 8)}...{update.tx_hash.slice(-8)}</span>
                    )}
                    {update.block_height && (
                      <span>Block: {update.block_height}</span>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </div>
        ))}
        {filtered.length === 0 && (
          <div className="text-center py-8 text-gray-500">
            No DNS updates found matching your criteria.
          </div>
        )}
      </div>
    );
  };

  const renderTldUpdates = () => {
    const filtered = filterTldUpdates(tldUpdates);
    
    return (
      <div className="space-y-4">
        {filtered.map(update => (
          <div key={update.id} className="border rounded-lg p-4 hover:bg-gray-50">
            <div className="flex items-start justify-between">
              <div className="flex items-start space-x-3">
                <span className="text-2xl">{getTypeIcon(update.type)}</span>
                <div className="flex-1">
                  <div className="flex items-center space-x-2">
                    <h3 className="font-medium text-gray-900">{update.tld}</h3>
                    <Badge className={getStatusColor(update.status)}>
                      {update.status}
                    </Badge>
                  </div>
                  <p className="text-sm text-gray-600 mt-1">{update.description}</p>
                  {update.old_price && update.new_price && (
                    <p className="text-xs text-gray-500 mt-1">
                      Price: {update.old_price / 1000000} IPN → {update.new_price / 1000000} IPN
                    </p>
                  )}
                  {update.new_price && !update.old_price && (
                    <p className="text-xs text-gray-500 mt-1">
                      Price: {update.new_price / 1000000} IPN
                    </p>
                  )}
                  <div className="flex items-center space-x-4 mt-2 text-xs text-gray-500">
                    <span>{formatTimeAgo(update.timestamp)}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        ))}
        {filtered.length === 0 && (
          <div className="text-center py-8 text-gray-500">
            No TLD updates found matching your criteria.
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Domain & DNS Updates</h1>
          <p className="text-sm text-gray-600">
            Track domain registrations, DNS changes, and TLD updates across the IPPAN network.
          </p>
        </div>
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
              className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
            />
            <label className="text-sm text-gray-600">Auto-refresh</label>
          </div>
          <div className="text-xs text-gray-500">
            Last update: {lastUpdate.toLocaleTimeString()}
          </div>
        </div>
      </div>

      {/* Filters */}
      <Card title="Filters">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Search</label>
            <Input
              type="text"
              placeholder="Search domains, records..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Status</label>
            <Select
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value)}
            >
              <option value="all">All Status</option>
              <option value="completed">Completed</option>
              <option value="pending">Pending</option>
              <option value="failed">Failed</option>
              <option value="active">Active</option>
              <option value="inactive">Inactive</option>
            </Select>
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Type</label>
            <Select
              value={typeFilter}
              onChange={(e) => setTypeFilter(e.target.value)}
            >
              <option value="all">All Types</option>
              <option value="registration">Registration</option>
              <option value="renewal">Renewal</option>
              <option value="transfer">Transfer</option>
              <option value="dns_change">DNS Change</option>
              <option value="expiration">Expiration</option>
              <option value="new_tld">New TLD</option>
              <option value="price_change">Price Change</option>
              <option value="availability_change">Availability Change</option>
            </Select>
          </div>
          <div className="flex items-end">
            <Button
              onClick={() => {
                setSearchQuery('');
                setStatusFilter('all');
                setTypeFilter('all');
              }}
              className="w-full bg-gray-600 hover:bg-gray-700"
            >
              Clear Filters
            </Button>
          </div>
        </div>
      </Card>

      {/* Tabs */}
      <div className="border-b border-gray-200">
        <nav className="-mb-px flex space-x-8">
          {[
            { id: 'domain', label: 'Domain Updates', count: domainUpdates.length },
            { id: 'dns', label: 'DNS Changes', count: dnsUpdates.length },
            { id: 'tld', label: 'TLD Updates', count: tldUpdates.length }
          ].map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as any)}
              className={`py-2 px-1 border-b-2 font-medium text-sm ${
                activeTab === tab.id
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              {tab.label}
              <span className="ml-2 bg-gray-100 text-gray-600 py-0.5 px-2 rounded-full text-xs">
                {tab.count}
              </span>
            </button>
          ))}
        </nav>
      </div>

      {/* Content */}
      <Card>
        {activeTab === 'domain' && renderDomainUpdates()}
        {activeTab === 'dns' && renderDnsUpdates()}
        {activeTab === 'tld' && renderTldUpdates()}
      </Card>

      {/* Summary Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card title="Domain Activity">
          <div className="text-center">
            <div className="text-2xl font-bold text-blue-600">{domainUpdates.length}</div>
            <div className="text-sm text-gray-600">Total Updates</div>
            <div className="text-xs text-gray-500 mt-1">
              {domainUpdates.filter(u => u.status === 'pending').length} pending
            </div>
          </div>
        </Card>
        <Card title="DNS Changes">
          <div className="text-center">
            <div className="text-2xl font-bold text-green-600">{dnsUpdates.length}</div>
            <div className="text-sm text-gray-600">Record Updates</div>
            <div className="text-xs text-gray-500 mt-1">
              {dnsUpdates.filter(u => u.status === 'completed').length} completed
            </div>
          </div>
        </Card>
        <Card title="TLD Registry">
          <div className="text-center">
            <div className="text-2xl font-bold text-purple-600">{tldUpdates.length}</div>
            <div className="text-sm text-gray-600">Registry Updates</div>
            <div className="text-xs text-gray-500 mt-1">
              {tldUpdates.filter(u => u.status === 'active').length} active
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}
