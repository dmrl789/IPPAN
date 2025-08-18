import { useEffect } from 'react'
import { useComposerStore } from '../stores/composerStore'

export default function CommandPalette() {
  const openWithType = useComposerStore(s => s.openWithType)

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const isMac = navigator.platform.toUpperCase().includes('MAC')
      const combo = (isMac && e.metaKey && e.key.toLowerCase() === 'k') || (!isMac && e.ctrlKey && e.key.toLowerCase() === 'k')
      if (combo) {
        e.preventDefault()
        openWithType('payment')
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [openWithType])

  return null
}
