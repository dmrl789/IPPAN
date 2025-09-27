import express from 'express'
import path from 'path'
import { fileURLToPath } from 'url'
import fs from 'fs'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const rootDir = path.resolve(__dirname, '..')
const distDir = path.join(rootDir, 'dist')

const app = express()
const port = Number.parseInt(process.env.PORT || '3000', 10)

app.disable('x-powered-by')

app.use((_, res, next) => {
  res.setHeader('X-Frame-Options', 'SAMEORIGIN')
  res.setHeader('X-Content-Type-Options', 'nosniff')
  res.setHeader('Referrer-Policy', 'strict-origin-when-cross-origin')
  next()
})

app.get(['/api/health', '/healthz', '/_health'], (_req, res) => {
  res.status(200).type('text/plain').send('ok')
})

if (!fs.existsSync(distDir)) {
  console.warn('⚠️  The dist/ directory was not found. Did you run "npm run build"?')
}

app.use(
  express.static(distDir, {
    fallthrough: true,
    index: false,
    setHeaders: (res, filePath) => {
      if (/\.(js|css|ico|png|jpg|jpeg|svg|woff2?)$/i.test(filePath)) {
        res.setHeader('Cache-Control', 'public, max-age=31536000, immutable')
      }
    },
  })
)

app.get('*', (_req, res, next) => {
  const indexPath = path.join(distDir, 'index.html')
  if (!fs.existsSync(indexPath)) {
    return next()
  }
  res.sendFile(indexPath)
})

app.use((_req, res) => {
  res.status(503).json({
    error: 'Unified UI build not found. Run "npm run build" before starting the server.',
  })
})

app.listen(port, '0.0.0.0', () => {
  console.log(`Unified UI ready on http://0.0.0.0:${port}`)
})
