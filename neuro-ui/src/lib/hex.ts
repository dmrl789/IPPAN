export function strip0x(h: string) {
  return h.startsWith("0x") ? h.slice(2) : h;
}
export function hexToBytes(hex: string): number[] {
  const clean = strip0x(hex).toLowerCase();
  if (clean.length % 2 !== 0) throw new Error("Hex must have even length");
  const out: number[] = [];
  for (let i=0;i<clean.length;i+=2) out.push(parseInt(clean.slice(i,i+2),16));
  return out;
}
export function ensureLen(bytes: number[], n: number, label="field") {
  if (bytes.length !== n) throw new Error(`${label} must be ${n} bytes`);
  return bytes;
}
