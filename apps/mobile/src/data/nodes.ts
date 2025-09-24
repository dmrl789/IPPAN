export interface NodeOption {
  id: string;
  label: string;
  baseUrl: string;
  region: string;
  latencyMs: number;
  isCommunity?: boolean;
}

export const defaultNodes: NodeOption[] = [
  {
    id: 'validator-eu-1',
    label: 'Validator • EU West',
    baseUrl: 'https://validator.eu-west.ippan.network',
    region: 'Frankfurt, DE',
    latencyMs: 92
  },
  {
    id: 'validator-us-1',
    label: 'Validator • US East',
    baseUrl: 'https://validator.us-east.ippan.network',
    region: 'Ashburn, US',
    latencyMs: 64
  },
  {
    id: 'gateway-apac',
    label: 'Gateway • APAC',
    baseUrl: 'https://gateway.apac.ippan.network',
    region: 'Singapore',
    latencyMs: 138
  },
  {
    id: 'community-latam',
    label: 'Community Node • LATAM',
    baseUrl: 'https://latam.community.ippan.network',
    region: 'São Paulo, BR',
    latencyMs: 182,
    isCommunity: true
  }
];
