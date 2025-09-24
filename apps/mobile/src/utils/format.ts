export function formatNumber(value: number | null | undefined, options: Intl.NumberFormatOptions = {}): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return '—';
  }
  return new Intl.NumberFormat('en-US', {
    maximumFractionDigits: value < 1 ? 4 : 2,
    minimumFractionDigits: value < 1 ? 2 : 0,
    ...options
  }).format(value);
}

export function formatCurrency(value: number | null | undefined, currency = 'USD'): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return '—';
  }
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency,
    maximumFractionDigits: 2
  }).format(value);
}

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || Number.isNaN(bytes)) {
    return '—';
  }
  if (bytes === 0) {
    return '0 B';
  }
  const k = 1024;
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const size = bytes / Math.pow(k, i);
  return `${size.toFixed(size >= 10 || i === 0 ? 0 : 1)} ${units[i]}`;
}

export function formatDateTime(input: string | number | Date | null | undefined): string {
  if (!input) {
    return '—';
  }
  const date = input instanceof Date ? input : new Date(input);
  if (Number.isNaN(date.getTime())) {
    return '—';
  }
  return `${date.toLocaleDateString()} ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
}

export function truncateAddress(address: string | null | undefined, size = 6): string {
  if (!address) {
    return '—';
  }
  if (address.length <= size * 2) {
    return address;
  }
  return `${address.slice(0, size)}…${address.slice(-size)}`;
}

export function formatPercent(value: number | null | undefined, fractionDigits = 1): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return '—';
  }
  return `${value.toFixed(fractionDigits)}%`;
}
