export function hexToBytes(hex: string): Uint8Array {
  if (hex.startsWith('0x')) {
    hex = hex.slice(2);
  }
  
  if (hex.length % 2 !== 0) {
    throw new Error('Hex string must have an even number of characters');
  }
  
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substr(i, 2), 16);
  }
  
  return bytes;
}

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

export function ensureLen(bytes: Uint8Array, length: number, name: string): Uint8Array {
  if (bytes.length !== length) {
    throw new Error(`${name} must be exactly ${length} bytes, got ${bytes.length}`);
  }
  return bytes;
}

export function padHex(hex: string, length: number): string {
  if (hex.startsWith('0x')) {
    hex = hex.slice(2);
  }
  
  if (hex.length > length * 2) {
    throw new Error(`Hex string too long: ${hex.length} chars, max ${length * 2}`);
  }
  
  return '0x' + hex.padStart(length * 2, '0');
}
