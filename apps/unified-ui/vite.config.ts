import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  envPrefix: ['VITE_', 'NEXT_PUBLIC_'],
  server: {
    port: 3000,
    host: '0.0.0.0',
    strictPort: true,
    hmr: {
      port: 3000
    }
  },
  preview: {
    port: 4000,
    host: '0.0.0.0'
  }
})
