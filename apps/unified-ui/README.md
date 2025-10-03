# IPPAN Unified Interface

A comprehensive web interface that combines all IPPAN network functionality into a single, modern React application with enterprise-grade wallet management and blockchain exploration capabilities.

## Features

### 🏦 Wallet & Finance
- **Enhanced Wallet Overview**: 
  - Multi-wallet support (Watch-only, Local, Hardware)
  - Real-time fee estimation with priority controls
  - QR code generation for receiving payments
  - Multi-asset support with fiat conversion (USD/EUR)
  - Advanced transaction preview with nonce override
  - Address book with search and quick pay
  - CSV export for transaction history
  - Security center with spending limits and device management
  - Hardware wallet integration with signature testing
  - Explorer-based recipient validation
- **Payments & M2M**: Send payments and manage machine-to-machine payment channels
- **Staking & Validator**: Stake tokens and participate in network validation
- **Domain Management**: 
  - Register and manage IPPAN domains
  - DNS data management (A/AAAA/CNAME/TXT/MX/SRV)
  - Domain renewal and auto-renewal
  - Proof of ownership verification (DNS TXT, HTML file, META tag, Wallet signature)
  - TLD search and availability checking
- **Storage**: Upload and manage files on the distributed storage network

### 🔍 Blockchain Explorer
- **Live Blocks**: Real-time block monitoring and validation
- **Transactions**: Search, filter, and analyze network transactions
- **Accounts**: View account balances, history, and activity
- **Validators**: Monitor validator performance and network consensus
- **Rounds & Finality**: Track consensus rounds and finality
- **Network Map**: Visualize network topology and node distribution
- **Analytics**: Network metrics, performance, and trends

### 🧠 Neural Network
- **Models**: Register and manage neural network models
- **Datasets**: Upload and manage training datasets
- **Inference**: Run inference jobs on registered models
- **Bids & Winners**: Participate in model auctions and view winners
- **Proofs**: Verify ZK-STARK and SNARK proofs for neural computations

## Tech Stack

- **React 18** with TypeScript
- **Vite** for fast development and building
- **React Router** for navigation
- **TanStack Query** for data fetching and caching
- **Tailwind CSS** for styling
- **Zustand** for state management
- **Axios** for API communication
- **QRCode** for QR code generation
- **Custom UI Components** for consistent design

## Getting Started

### Prerequisites

- Node.js 18+ 
- npm or yarn

### Installation

1. Navigate to the unified-ui directory:
```bash
cd apps/unified-ui
```

2. Install dependencies:
```bash
npm install
```

3. Start the development server:
```bash
npm run dev
```

4. Open your browser and navigate to `http://localhost:5173`

### Building for Production

```bash
npm run build
```

The built files will be in the `dist` directory.

### Server Deployment Checklist

1. **Configure Environment Variables**
   - Copy `.env.production.example` to `.env.production` (or `.env.local` for local overrides) and adjust the URLs for your infrastructure:

     ```bash
     cp .env.production.example .env.production
     ```

   - Update `NEXT_PUBLIC_GATEWAY_URL` (alias: `NEXT_PUBLIC_API_BASE_URL`) and `NEXT_PUBLIC_WS_URL` to point at the HTTP and WebSocket entrypoints browsers should use. These variables are also exposed as `VITE_*` for backwards compatibility.
   - Set `NEXT_PUBLIC_ENABLE_FULL_UI=1` during the build to ensure the full navigation experience is rendered (this flag defaults to enabled).

2. **Build & Serve the UI**
   - Install dependencies and produce a production build:

     ```bash
     npm install
     npm run build
     ```

   - Start the lightweight static server (listens on port `3000` by default and exposes `/api/health`):

     ```bash
     npm run start
     ```

     The server located at `server/serve.js` provides security headers, caches static assets, and returns a 200/`ok` response for health checks.

   - **Serve the build directly from the IPPAN node (optional):**
     - Place the compiled assets where the Rust node can find them (default: `apps/unified-ui/dist`).
     - Or point the node at a custom directory via `IPPAN_UNIFIED_UI_DIST_DIR=/path/to/dist`.
     - When configured, the RPC server will expose the Unified UI at `/` while keeping the `/api` routes available.

3. **Expose the UI Through Your Reverse Proxy**
   - Ensure the domain you will use for the UI is allowed by your reverse proxy. Your previous Envoy configuration returned `Domain forbidden`, which indicates the host header needs to be listed under the allowed domains. The snippets below illustrate the minimal configuration needed for Nginx and Envoy (replace hostnames/IPs with your values):

     **Nginx**

     ```nginx
     server {
       listen 80;
       server_name ui.example.com;

       location / {
        proxy_pass http://127.0.0.1:3001;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        add_header Access-Control-Allow-Origin "https://ui.example.com";
      }

      location /api/health {
        proxy_pass http://127.0.0.1:3001/api/health;
        access_log off;
      }
     }
     ```

     **Envoy**

     ```yaml
     static_resources:
       listeners:
         - name: web_listener
           address:
             socket_address: { address: 0.0.0.0, port_value: 80 }
           filter_chains:
             - filters:
                 - name: envoy.filters.network.http_connection_manager
                   typed_config:
                     "@type": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager
                     stat_prefix: ingress_http
                     route_config:
                       name: local_route
                       virtual_hosts:
                         - name: unified_ui
                           domains: ["ui.example.com", "*"]
                           routes:
                             - match: { prefix: "/api/health" }
                               route: { cluster: unified_ui }
                             - match: { prefix: "/" }
                               route: { cluster: unified_ui }
                     http_filters:
                       - name: envoy.filters.http.router
       clusters:
         - name: unified_ui
           connect_timeout: 1s
           type: logical_dns
           load_assignment:
             cluster_name: unified_ui
             endpoints:
               - lb_endpoints:
                  - endpoint:
                       address:
                       socket_address:
                         address: 127.0.0.1
                         port_value: 3000
     ```

4. **Enable TLS**
   - Terminate HTTPS at your reverse proxy using a trusted certificate (e.g., from Let’s Encrypt). Browsers will block RPC calls to insecure origins when the UI is served over HTTPS but calls an HTTP backend.

5. **Health Checks**
   - Verify that the RPC/API endpoint referenced in your `.env` file is reachable from the UI host:

     ```bash
     curl -f https://rpc.example.com/time
     ```

   - Confirm that the browser’s network tab shows successful responses for the API endpoints listed below.

## Project Structure

```
src/
├── components/
│   └── UI.tsx              # Reusable UI components (Card, Button, Field, Input, Badge, etc.)
├── lib/
│   ├── api.ts              # API client and interfaces
│   └── hex.ts              # Hex utility functions
├── pages/
│   ├── WalletOverview.tsx  # Enhanced wallet management
│   ├── PaymentsPage.tsx    # Payments and M2M
│   ├── StakingPage.tsx     # Staking and validation
│   ├── DomainsPage.tsx     # Domain management with verification
│   ├── StoragePage.tsx     # File storage
│   ├── explorer/           # Blockchain explorer pages
│   │   ├── LiveBlocksPage.tsx
│   │   ├── TransactionsPage.tsx
│   │   ├── AccountsPage.tsx
│   │   ├── ValidatorsPage.tsx
│   │   ├── FinalityPage.tsx
│   │   ├── ContractsPage.tsx
│   │   ├── NetworkMapPage.tsx
│   │   └── AnalyticsPage.tsx
│   ├── ModelsPage.tsx      # Neural models
│   ├── DatasetsPage.tsx    # Datasets
│   ├── InferencePage.tsx   # Inference jobs
│   ├── BidsPage.tsx        # Model auctions
│   └── ProofsPage.tsx      # Proof verification
├── App.tsx                 # Main application component with navigation
├── main.tsx               # Application entry point
└── index.css              # Global styles
```

## Configuration

### Environment Variables

Create a `.env` file in the root directory:

```env
VITE_API_URL=http://localhost:8080
```

### API Endpoints

The application expects the following API endpoints:

#### Wallet & Finance
- `GET /api/wallet/{address}` - Get wallet state (balances, tokens, staking, activities, domains)
- `POST /api/wallet/send` - Send payment with fee and nonce
- `GET /api/fees/estimate` - Estimate transaction fees with priority
- `GET /api/rates` - Get fiat exchange rates
- `GET /api/explorer/address/{addr}/exists` - Check if address exists in explorer

#### Domain Management
- `GET /api/tlds` - Get available TLDs
- `GET /api/domains/check` - Check domain availability
- `POST /api/domains/register` - Register new domain
- `GET /api/domains/my` - Get user's domains
- `POST /api/domains/renew` - Renew domain
- `GET /api/domains/{fqdn}/dns` - Get DNS records
- `POST /api/domains/{fqdn}/dns` - Update DNS records
- `DELETE /api/domains/{fqdn}/dns/{id}` - Delete DNS record
- `GET /api/domains/verify/challenge` - Get verification challenge
- `POST /api/domains/verify/check` - Check verification proof
- `POST /api/domains/verify/submit` - Submit verification

#### Blockchain Explorer
- `GET /api/explorer/blocks/live` - Get live blocks
- `GET /api/explorer/transactions` - Get transactions with filters
- `GET /api/explorer/accounts` - Get accounts
- `GET /api/explorer/validators` - Get validators
- `GET /api/explorer/consensus/rounds` - Get consensus rounds
- `GET /api/explorer/contracts` - Get smart contracts
- `GET /api/explorer/network/map` - Get network topology
- `GET /api/explorer/analytics` - Get network analytics

#### Neural Network
- `GET /api/models` - Get models
- `POST /api/models` - Register model
- `GET /api/datasets` - Get datasets
- `POST /api/datasets` - Register dataset
- `POST /api/inference` - Run inference job
- `GET /api/bids` - Get model bids
- `GET /api/proofs` - Get proofs

#### Storage
- `POST /api/storage/upload` - Upload file

### Wallet Provider Integration

The application supports integration with IPPAN wallet providers:

```typescript
// Wallet connection
window.ippan.connect() → Promise<string> // Returns wallet address

// Hardware wallet connection
window.ippan.connectHardware() → Promise<string> // Returns hardware wallet address

// Message signing
window.ippan.signMessage(message: string) → Promise<string> // Returns signature

// Transaction signing
window.ippan.sendTransaction(tx: Transaction) → Promise<string> // Returns tx hash
```

## Enhanced Features

### Wallet Overview Enhancements

#### Multi-Wallet Support
- **Watch-only Mode**: View balances without signing capability
- **Local Wallets**: Full control with seed phrase/private key
- **Hardware Wallets**: Secure signing with hardware devices

#### Advanced Transaction Features
- **Fee Estimation**: Real-time fee calculation with priority controls
- **Nonce Management**: Automatic and manual nonce override
- **Transaction Preview**: Complete transaction details before sending
- **Recipient Validation**: Explorer-based address verification

#### Security & Privacy
- **Spending Limits**: Configurable daily transaction limits
- **Device Management**: Track and revoke approved devices
- **Session Security**: Secure session management and cleanup
- **Address Book**: Encrypted local storage of contacts

#### User Experience
- **QR Code Generation**: Instant QR codes for receiving payments
- **Multi-Currency Display**: Real-time USD/EUR conversion
- **CSV Export**: Complete transaction history export
- **Responsive Design**: Mobile-friendly interface

### Domain Management Features

#### Registration & Management
- **TLD Search**: Searchable TLD list with availability checking
- **DNS Management**: Full CRUD operations for DNS records
- **Renewal System**: Domain renewal with auto-renewal options
- **Ownership Verification**: Multiple verification methods

#### Verification Methods
- **DNS TXT**: Add verification record to DNS
- **HTML File**: Host verification file at `/.well-known/ippan-verify.txt`
- **META Tag**: Add verification meta tag to website
- **Wallet Signature**: Sign verification message with wallet

### Blockchain Explorer

#### Real-time Monitoring
- **Live Blocks**: Real-time block creation and validation
- **Transaction Tracking**: Search and filter network transactions
- **Account Analysis**: Comprehensive account information and history

#### Network Analytics
- **Validator Performance**: Monitor validator participation and rewards
- **Consensus Tracking**: Track consensus rounds and finality
- **Network Topology**: Visualize node distribution and connections

## Development

### Adding New Pages

1. Create a new page component in `src/pages/`
2. Add the route to `src/App.tsx`
3. Add navigation item to the sidebar

### Styling

The application uses Tailwind CSS for styling. Custom styles can be added to `src/index.css`.

### State Management

- Local component state: Use React's `useState`
- Global state: Use Zustand stores
- Server state: Use TanStack Query
- Local storage: Custom helpers for persistence

### Custom UI Components

The application uses a custom UI component library (`src/components/UI.tsx`) that provides:

- `Card`: Container with title and optional footer
- `Button`: Styled button with variants
- `Field`: Form field with label
- `Input`: Text input with validation
- `Badge`: Status indicators with variants
- `Textarea`: Multi-line text input
- `LoadingSpinner`: Loading indicator

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is part of the IPPAN network and follows the same licensing terms.

## Support

For support and questions, please refer to the main IPPAN documentation or create an issue in the repository.
