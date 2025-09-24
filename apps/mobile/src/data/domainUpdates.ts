export type DomainUpdateType = 'registration' | 'renewal' | 'transfer' | 'dns_change' | 'expiration' | 'tld_update';

export interface DomainUpdate {
  id: string;
  type: DomainUpdateType;
  domain: string;
  description: string;
  timestamp: string;
  status: 'pending' | 'completed' | 'failed';
  txHash?: string;
  blockHeight?: number;
  details?: Record<string, unknown>;
}

export interface DnsUpdate {
  id: string;
  domain: string;
  recordType: string;
  recordName: string;
  oldValue?: string;
  newValue: string;
  timestamp: string;
  status: 'pending' | 'completed' | 'failed';
  txHash?: string;
  blockHeight?: number;
}

export interface TldUpdate {
  id: string;
  tld: string;
  type: 'new_tld' | 'price_change' | 'availability_change';
  description: string;
  timestamp: string;
  oldPrice?: number;
  newPrice?: number;
  status: 'active' | 'inactive';
}

export const mockDomainUpdates: DomainUpdate[] = [
  {
    id: '1',
    type: 'registration',
    domain: 'alice.ipn',
    description: 'New domain registered',
    timestamp: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
    status: 'completed',
    txHash: '0x1234567890abcdef1234567890abcdef12345678',
    blockHeight: 12345,
    details: { duration_years: 1, fee_paid: 1_000_000 }
  },
  {
    id: '2',
    type: 'renewal',
    domain: 'bob.ai',
    description: 'Domain renewed for 2 years',
    timestamp: new Date(Date.now() - 4 * 60 * 60 * 1000).toISOString(),
    status: 'completed',
    txHash: '0xabcdef1234567890abcdef1234567890abcdef12',
    blockHeight: 12340,
    details: { duration_years: 2, fee_paid: 2_000_000 }
  },
  {
    id: '3',
    type: 'transfer',
    domain: 'charlie.iot',
    description: 'Domain ownership transferred',
    timestamp: new Date(Date.now() - 6 * 60 * 60 * 1000).toISOString(),
    status: 'pending',
    txHash: '0x9876543210fedcba9876543210fedcba98765432',
    blockHeight: 12335,
    details: {
      from: 'i1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa',
      to: 'i1B2zP2eP6QGefi3DMPTfTL6SLmv8DivfNb'
    }
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

export const mockDnsUpdates: DnsUpdate[] = [
  {
    id: '1',
    domain: 'alice.ipn',
    recordType: 'A',
    recordName: '@',
    oldValue: '192.168.1.1',
    newValue: '192.168.1.2',
    timestamp: new Date(Date.now() - 1 * 60 * 60 * 1000).toISOString(),
    status: 'completed',
    txHash: '0x1111111111111111111111111111111111111111',
    blockHeight: 12344
  },
  {
    id: '2',
    domain: 'bob.ai',
    recordType: 'CNAME',
    recordName: 'www',
    newValue: 'bob.ai',
    timestamp: new Date(Date.now() - 3 * 60 * 60 * 1000).toISOString(),
    status: 'completed',
    txHash: '0x2222222222222222222222222222222222222222',
    blockHeight: 12341
  },
  {
    id: '3',
    domain: 'charlie.iot',
    recordType: 'TXT',
    recordName: '@',
    newValue: 'v=spf1 include:_spf.google.com ~all',
    timestamp: new Date(Date.now() - 5 * 60 * 60 * 1000).toISOString(),
    status: 'pending',
    txHash: '0x3333333333333333333333333333333333333333',
    blockHeight: 12336
  }
];

export const mockTldUpdates: TldUpdate[] = [
  {
    id: '1',
    tld: '.ai',
    type: 'price_change',
    description: 'Premium TLD price updated',
    timestamp: new Date(Date.now() - 12 * 60 * 60 * 1000).toISOString(),
    oldPrice: 10_000_000,
    newPrice: 12_000_000,
    status: 'active'
  },
  {
    id: '2',
    tld: '.iot',
    type: 'availability_change',
    description: 'New TLD available for registration',
    timestamp: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(),
    newPrice: 2_000_000,
    status: 'active'
  },
  {
    id: '3',
    tld: '.fin',
    type: 'new_tld',
    description: 'New premium TLD launched',
    timestamp: new Date(Date.now() - 48 * 60 * 60 * 1000).toISOString(),
    newPrice: 15_000_000,
    status: 'active'
  }
];
