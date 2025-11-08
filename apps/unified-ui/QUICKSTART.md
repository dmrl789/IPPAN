# Quick Start Guide - IPPAN Unified UI

Get the IPPAN Unified UI running in under 5 minutes!

## 🚀 Option 1: Local Development (Fastest)

```bash
# Navigate to the UI directory
cd /workspace/apps/unified-ui

# Install dependencies (first time only)
npm install

# Start development server
npm run dev
```

Visit **http://localhost:3000** in your browser.

---

## 📦 Option 2: Build & Serve Static Site

```bash
# Build static export
npm run build

# Serve locally
npm run serve
```

Visit **http://localhost:3000** in your browser.

---

## 🐳 Option 3: Docker (Production)

```bash
# Build Docker image
npm run docker:build

# Run container
npm run docker:run
```

Visit **http://localhost:3000** in your browser.

---

## ⚙️ Environment Configuration

**For local development**, copy and edit `.env.local`:

```bash
cp .env.local.example .env.local
```

**Required variables:**
- `NEXT_PUBLIC_API_BASE_URL` - Backend API URL (default: http://localhost:7080)
- `NEXT_PUBLIC_WS_URL` - WebSocket URL (default: ws://localhost:7080/ws)

See [README.md](./README.md) for full environment variable documentation.

---

## 🔗 Backend Services

The UI requires these backend services to be running:

1. **IPPAN Node** (port 7080)
   - Blockchain API
   - WebSocket server
   - AI services

2. **API Gateway** (port 8081) - Optional
   - Aggregated API endpoints

---

## 📖 Full Documentation

- [README.md](./README.md) - Complete feature list, setup, API docs
- [DEPLOYMENT.md](./DEPLOYMENT.md) - Production deployment guide
- [FIX_SUMMARY.md](./FIX_SUMMARY.md) - Recent changes and improvements

---

## ❓ Troubleshooting

### Issue: Build fails

**Solution:**
```bash
# Clean Next.js cache
rm -rf .next

# Reinstall dependencies
rm -rf node_modules package-lock.json
npm install

# Try building again
npm run build
```

### Issue: API connections fail

**Check:**
1. Backend services are running
2. `NEXT_PUBLIC_API_BASE_URL` is correct
3. CORS is enabled on backend
4. Firewall allows connections

### Issue: WebSocket won't connect

**Check:**
1. WebSocket server is running
2. `NEXT_PUBLIC_WS_URL` starts with `ws://` or `wss://`
3. Port 7080 is not blocked

---

## 🎯 What's Included

- ✅ **AI Dashboard** - Real-time AI insights
- ✅ **Blockchain Explorer** - Browse blocks and transactions
- ✅ **Smart Contract Studio** - Develop contracts
- ✅ **Analytics Panel** - Network metrics
- ✅ **Monitoring Center** - System health
- ✅ **MetaAgent Dashboard** - Agent governance

---

**Need help?** Check the [README.md](./README.md) or contact the maintainers.

**Agent Scope:** `/apps/unified-ui` (Agent-Lambda)  
**Maintainer:** Desirée Verga
