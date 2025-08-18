import { create } from 'zustand'

type TxType = 'payment' | 'storage' | 'domain' | 'staking' | 'anchor' | 'm2m' | 'l2_settlement' | 'l2_data'

type ComposerState = {
  isOpen: boolean
  type: TxType
  openWithType: (type: TxType) => void
  setType: (type: TxType) => void
  close: () => void
}

export const useComposerStore = create<ComposerState>((set) => ({
  isOpen: false,
  type: 'payment',
  openWithType: (type) => set({ isOpen: true, type }),
  setType: (type) => set({ type }),
  close: () => set({ isOpen: false }),
}))
