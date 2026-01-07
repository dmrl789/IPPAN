export function formatTimestamp(value: string) {
  return new Date(value).toLocaleString();
}

export function formatAmount(value: number) {
  return `${value.toLocaleString(undefined, { maximumFractionDigits: 4 })} IPN`;
}

export function shortenHash(hash: string, size = 10) {
  if (hash.length <= size * 2) return hash;
  return `${hash.slice(0, size)}…${hash.slice(-size)}`;
}
