const env = (import.meta as any).env ?? {}

function sanitizeUrl(value: string | undefined): string | undefined {
  if (!value) {
    return undefined
  }

  try {
    const parsed = new URL(value)
    return parsed.toString().replace(/\/$/, '')
  } catch (error) {
    console.warn('Invalid URL provided via environment variable:', value, error)
    return undefined
  }
}

const apiBaseUrl =
  sanitizeUrl(env.NEXT_PUBLIC_API_BASE_URL) ??
  sanitizeUrl(env.VITE_API_URL) ??
  'http://localhost:8080'

const wsUrl =
  sanitizeUrl(env.NEXT_PUBLIC_WS_URL) ??
  sanitizeUrl(env.VITE_WS_URL) ??
  undefined

const networkName = env.NEXT_PUBLIC_NETWORK_NAME || 'IPPAN Devnet'
const explorerBase = sanitizeUrl(env.NEXT_PUBLIC_EXPLORER_BASE) ?? undefined

export const UIConfig = {
  apiBaseUrl,
  wsUrl,
  networkName,
  explorerBase,
}

export type UIConfigType = typeof UIConfig
