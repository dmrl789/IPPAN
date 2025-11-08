# IPPAN Unified UI

A modern Next.js-based UI for the IPPAN blockchain with AI integration.

## Features

- 🤖 **AI-Powered Dashboard** - Real-time AI insights and analytics
- 🔗 **Blockchain Explorer** - Browse blocks, transactions, and network stats
- 📝 **Smart Contract Studio** - Develop and deploy smart contracts
- 📊 **Analytics Panel** - Network performance and metrics
- 🔍 **Monitoring Center** - System health and security monitoring
- 🎯 **MetaAgent Dashboard** - Agent orchestration and governance

## Technology Stack

- **Framework:** Next.js 14 (App Router)
- **Language:** TypeScript
- **Styling:** Tailwind CSS
- **UI Components:** Headless UI, Heroicons, Lucide React
- **Animation:** Framer Motion
- **Charts:** Recharts
- **Code Editor:** Monaco Editor
- **API Communication:** Axios, Native Fetch
- **WebSocket:** Native WebSocket API

## Getting Started

### Prerequisites

- Node.js 18+ and npm
- IPPAN backend services running (API server, gateway)

### Installation

```bash
# Install dependencies
npm install

# Copy environment variables
cp .env.example .env.local

# Start development server
npm run dev
```

The application will be available at `http://localhost:3000`

## Environment Variables

All environment variables must be prefixed with `NEXT_PUBLIC_` to be accessible in the browser.

### Required Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `NEXT_PUBLIC_ENABLE_FULL_UI` | Enable full UI features | `1` | `1` |
| `NEXT_PUBLIC_NETWORK_NAME` | Network identifier | `IPPAN-Devnet` | `IPPAN-Mainnet` |
| `NEXT_PUBLIC_API_BASE_URL` | Backend API base URL | `http://localhost:7080` | `https://api.ippan.network` |
| `NEXT_PUBLIC_GATEWAY_URL` | Gateway API URL | `http://localhost:8081/api` | `https://gateway.ippan.network/api` |
| `NEXT_PUBLIC_WS_URL` | WebSocket connection URL | `ws://localhost:7080/ws` | `wss://api.ippan.network/ws` |
| `NEXT_PUBLIC_AI_ENABLED` | Enable AI features | `1` | `1` or `0` |

### Optional Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `NEXT_PUBLIC_ASSET_PREFIX` | CDN or subdirectory path | `/ippan-ui` or `https://cdn.example.com` |

### Environment Files

- `.env.development` - Development environment (loaded by default with `npm run dev`)
- `.env.production` - Production environment (loaded with `npm run build`)
- `.env.local` - Local overrides (gitignored, highest priority)
- `.env.example` - Template for required variables

## Build & Deploy

### Static Export (Recommended)

The application is configured for static export, generating a fully static site:

```bash
# Build static export
npm run build

# Output directory: ./out
# Deploy the ./out directory to any static hosting service
```

### Deployment Options

#### 1. **Nginx / Apache**
```bash
# Copy the out directory to your web server
cp -r out/* /var/www/html/
```

#### 2. **Vercel**
```bash
vercel deploy
```

#### 3. **Netlify**
```bash
netlify deploy --dir=out --prod
```

#### 4. **AWS S3 + CloudFront**
```bash
aws s3 sync out/ s3://your-bucket-name/
```

#### 5. **Docker**
```bash
# Build image
docker build -t ippan-ui .

# Run container
docker run -p 80:80 ippan-ui
```

### Build Configuration

The `next.config.js` is configured for static export with the following settings:

- `output: 'export'` - Generates static HTML/CSS/JS files
- `images.unoptimized: true` - Disables Next.js image optimization (required for static export)
- `trailingSlash: true` - Adds trailing slashes to URLs for better hosting compatibility
- Asset prefix support for CDN deployment

## Development

### Available Scripts

```bash
# Start development server
npm run dev

# Build for production
npm run build

# Run production build locally
npm run start

# Lint code
npm run lint

# Type check
npm run type-check
```

### Project Structure

```
src/
├── app/                    # Next.js App Router
│   ├── globals.css        # Global styles
│   ├── layout.tsx         # Root layout
│   └── page.tsx           # Home page
├── components/            # React components
│   ├── ai/               # AI-related components
│   ├── analytics/        # Analytics components
│   ├── blockchain/       # Blockchain explorer components
│   ├── layout/           # Layout components (Header, Navigation)
│   ├── metaagent/        # MetaAgent components
│   ├── monitoring/       # Monitoring components
│   └── smart-contracts/  # Smart contract components
└── contexts/             # React contexts
    ├── AIContext.tsx     # AI state management
    ├── ThemeContext.tsx  # Theme management
    └── WebSocketContext.tsx # WebSocket connection
```

## API Integration

The UI connects to backend services via:

### REST API
- **Base URL:** `NEXT_PUBLIC_API_BASE_URL`
- **Endpoints:**
  - `/api/ai/status` - Get AI service status
  - `/api/ai/toggle` - Toggle AI service
  - `/api/blockchain/*` - Blockchain queries
  - `/api/contracts/*` - Smart contract operations

### WebSocket
- **URL:** `NEXT_PUBLIC_WS_URL`
- **Features:**
  - Real-time blockchain updates
  - Live transaction monitoring
  - Network status notifications
  - AI prediction streams

### Gateway API
- **Base URL:** `NEXT_PUBLIC_GATEWAY_URL`
- **Purpose:** Aggregated API for cross-service queries

## Static Export Considerations

Since this app uses static export (`output: 'export'`), the following Next.js features are **not available**:

- ❌ API Routes (`/app/api/*`)
- ❌ Server-Side Rendering (SSR)
- ❌ Incremental Static Regeneration (ISR)
- ❌ Image Optimization
- ❌ Rewrites/Redirects

Instead, the app:

- ✅ Connects directly to backend APIs via fetch
- ✅ Uses client-side routing
- ✅ Handles all data fetching on the client
- ✅ Deploys as pure static files

## Troubleshooting

### Build Errors

**Error: API routes are not supported with static export**
- Solution: Remove API routes from `src/app/api/` directory. Use direct backend API calls instead.

**Error: Image optimization requires a server**
- Solution: Add `images: { unoptimized: true }` to `next.config.js`

### Runtime Issues

**CORS errors when connecting to backend**
- Ensure backend API has CORS headers configured
- Check that `NEXT_PUBLIC_API_BASE_URL` is correct
- Verify backend is running and accessible

**WebSocket connection fails**
- Check `NEXT_PUBLIC_WS_URL` format (must start with `ws://` or `wss://`)
- Ensure WebSocket port is not blocked by firewall
- Verify backend WebSocket server is running

**Environment variables not working**
- All browser-accessible variables must have `NEXT_PUBLIC_` prefix
- Rebuild after changing `.env` files: `npm run build`
- Clear Next.js cache: `rm -rf .next`

## Contributing

See the main project [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

Apache-2.0 - See [LICENSE](../../LICENSE) for details.

## Agent Assignment

**Scope:** `/apps/unified-ui`  
**Agent:** Agent-Lambda (Unified UI & Tauri frontend)  
**Maintainer:** Desirée Verga

---

*Part of the IPPAN blockchain ecosystem*
