import { create } from 'zustand'

export const WALLET_STORAGE_KEY = 'ippan.wallet.address'
export const WALLET_ADDRESS_REGEX = /^i[0-9a-fA-F]{64}$/

type WalletState = {
  address: string
  setAddress: (address: string) => void
  clearAddress: () => void
}

function readStoredAddress(): string {
  if (typeof window === 'undefined') {
    return ''
  }

  try {
    return window.localStorage.getItem(WALLET_STORAGE_KEY) ?? ''
  } catch (error) {
    console.warn('Unable to read wallet address from storage:', error)
    return ''
  }
}

export const useWalletStore = create<WalletState>((set) => ({
  address: readStoredAddress(),
  setAddress: (address) => {
    set({ address })

    if (typeof window !== 'undefined') {
      try {
        window.localStorage.setItem(WALLET_STORAGE_KEY, address)
      } catch (error) {
        console.warn('Unable to persist wallet address:', error)
      }
    }
  },
  clearAddress: () => {
    set({ address: '' })

    if (typeof window !== 'undefined') {
      try {
        window.localStorage.removeItem(WALLET_STORAGE_KEY)
      } catch (error) {
        console.warn('Unable to clear wallet address:', error)
      }
    }
  },
}))

if (typeof window !== 'undefined') {
  window.addEventListener('storage', (event) => {
    if (event.key === WALLET_STORAGE_KEY) {
      useWalletStore.setState({ address: event.newValue ?? '' })
    }
  })
}

export function isValidWalletAddress(address: string | null | undefined): boolean {
  if (!address) {
    return false
  }

  return WALLET_ADDRESS_REGEX.test(address)
}
