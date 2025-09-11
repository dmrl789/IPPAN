# 🚀 IPPAN Developer Quick Start Guide

## Overview

This guide will help you quickly set up and run the enhanced IPPAN frontend application with all the new wallet features and blockchain explorer functionality.

## Prerequisites

- **Node.js 18+** (Recommended: Node.js 20 LTS)
- **npm** or **yarn** package manager
- **Git** for version control
- **Modern browser** (Chrome, Firefox, Safari, Edge)

## Quick Setup

### 1. Clone and Navigate

```bash
# Clone the repository (if not already done)
git clone <repository-url>
cd ippan

# Navigate to the frontend application
cd apps/unified-ui
```

### 2. Install Dependencies

```bash
# Install all dependencies
npm install

# Install additional dependencies for enhanced features
npm install qrcode
npm install --save-dev @types/qrcode
```

### 3. Start Development Server

```bash
# Start the development server
npm run dev
```

The application will be available at `http://localhost:5173`

## 🏦 Enhanced Wallet Features

### Multi-Wallet Support

The enhanced wallet supports three types of wallets:

1. **Watch-only**: View balances without signing capability
2. **Local**: Full control with seed phrase/private key
3. **Hardware**: Secure signing with hardware devices

### Key Features to Test

#### Wallet Connection
- Connect by address (watch-only mode)
- Connect demo wallet (local mode)
- Connect provider (requires `window.ippan.connect()`)
- Connect hardware wallet (requires `window.ippan.connectHardware()`)

#### Advanced Transaction Features
- **Fee Estimation**: Real-time fee calculation with priority slider
- **Nonce Management**: Automatic and manual nonce override
- **Recipient Validation**: Explorer-based address verification
- **Transaction Preview**: Complete transaction details before sending

#### Security Features
- **Spending Limits**: Set daily transaction limits
- **Device Management**: Track and revoke approved devices
- **Session Security**: Secure session management
- **Address Book**: Encrypted local storage of contacts

#### User Experience
- **QR Code Generation**: Instant QR codes for receiving payments
- **Multi-Currency Display**: Real-time USD/EUR conversion
- **CSV Export**: Export transaction history
- **Responsive Design**: Mobile-friendly interface

## 🔍 Blockchain Explorer

### Available Pages

1. **Live Blocks**: Real-time block monitoring
2. **Transactions**: Search and filter transactions
3. **Accounts**: Account information and history
4. **Validators**: Validator performance monitoring
5. **Rounds & Finality**: Consensus tracking
6. **Smart Contracts**: Contract browsing
7. **Network Map**: Network topology visualization
8. **Analytics**: Network metrics and trends

### Navigation

Access the blockchain explorer through the sidebar navigation under "Blockchain Explorer".

## 🌐 Domain Management

### Enhanced Features

1. **TLD Search**: Searchable TLD list with availability checking
2. **DNS Management**: Full CRUD operations for DNS records
3. **Domain Renewal**: Renewal with auto-renewal options
4. **Ownership Verification**: Multiple verification methods

### Verification Methods

1. **DNS TXT**: Add verification record to DNS
2. **HTML File**: Host verification file at `/.well-known/ippan-verify.txt`
3. **META Tag**: Add verification meta tag to website
4. **Wallet Signature**: Sign verification message with wallet

## 🧠 Neural Network Marketplace

### Available Features

1. **Models**: Register and manage neural network models
2. **Datasets**: Upload and manage training datasets
3. **Inference**: Run inference jobs on registered models
4. **Bids & Winners**: Participate in model auctions
5. **Proofs**: Verify ZK-STARK and SNARK proofs

## 🔧 Development Features

### Mock APIs

The application uses comprehensive mock APIs for development:

- **Wallet APIs**: Multi-wallet support and transaction management
- **Domain APIs**: DNS management and verification
- **Explorer APIs**: Blockchain data and analytics
- **Neural APIs**: AI/ML marketplace services

### Local Storage

The application uses local storage for:

- Wallet addresses and types
- Address book entries
- User preferences
- Session data

### Custom UI Components

The application uses a custom UI component library:

- `Card`: Container with title and optional footer
- `Button`: Styled button with variants
- `Field`: Form field with label
- `Input`: Text input with validation
- `Badge`: Status indicators with variants
- `Textarea`: Multi-line text input
- `LoadingSpinner`: Loading indicator

## 🚀 Testing Features

### Wallet Testing

1. **Connect Demo Wallet**: Test with generated demo addresses
2. **Test Transactions**: Send test transactions with mock data
3. **Fee Estimation**: Test fee calculation with different priorities
4. **Address Validation**: Test recipient address verification

### Domain Testing

1. **TLD Search**: Test TLD search and availability checking
2. **DNS Management**: Test DNS record CRUD operations
3. **Verification**: Test domain ownership verification methods

### Explorer Testing

1. **Live Data**: View real-time mock blockchain data
2. **Search & Filter**: Test search and filtering capabilities
3. **Pagination**: Test pagination for large datasets

## 🔒 Security Testing

### Wallet Security

1. **Watch-only Mode**: Verify no signing capability
2. **Hardware Integration**: Test hardware wallet connections
3. **Session Management**: Test secure session handling
4. **Input Validation**: Test comprehensive form validation

### Data Protection

1. **Local Storage**: Verify encrypted storage
2. **Input Sanitization**: Test input validation and sanitization
3. **Error Handling**: Test secure error handling

## 📊 Performance Testing

### Optimization Features

1. **Lazy Loading**: Test efficient component loading
2. **Debounced Inputs**: Test optimized search and validation
3. **Memoization**: Test React.memo and useMemo usage
4. **Bundle Size**: Monitor bundle size optimization

### User Experience

1. **Loading States**: Test comprehensive loading indicators
2. **Error Boundaries**: Test graceful error handling
3. **Responsive Design**: Test mobile-friendly layout
4. **Accessibility**: Test keyboard navigation and screen readers

## 🛠️ Development Tools

### Code Quality

- **TypeScript**: Type-safe development
- **ESLint**: Code quality enforcement
- **Prettier**: Code formatting
- **Vite**: Fast development and building

### Debugging

- **React DevTools**: Component debugging
- **Browser DevTools**: Network and performance debugging
- **Console Logging**: Comprehensive logging for development
- **Error Tracking**: Error boundary and tracking

## 📚 API Documentation

### Wallet APIs

```typescript
// Get wallet state
GET /api/wallet/{address}

// Send payment
POST /api/wallet/send

// Estimate fees
GET /api/fees/estimate

// Get exchange rates
GET /api/rates

// Check address exists
GET /api/explorer/address/{addr}/exists
```

### Domain APIs

```typescript
// Get TLDs
GET /api/tlds

// Check domain availability
GET /api/domains/check

// Register domain
POST /api/domains/register

// Get user's domains
GET /api/domains/my

// Renew domain
POST /api/domains/renew

// Get DNS records
GET /api/domains/{fqdn}/dns

// Update DNS records
POST /api/domains/{fqdn}/dns

// Delete DNS record
DELETE /api/domains/{fqdn}/dns/{id}

// Get verification challenge
GET /api/domains/verify/challenge

// Check verification proof
POST /api/domains/verify/check

// Submit verification
POST /api/domains/verify/submit
```

### Explorer APIs

```typescript
// Get live blocks
GET /api/explorer/blocks/live

// Get transactions
GET /api/explorer/transactions

// Get accounts
GET /api/explorer/accounts

// Get validators
GET /api/explorer/validators

// Get consensus rounds
GET /api/explorer/consensus/rounds

// Get smart contracts
GET /api/explorer/contracts

// Get network map
GET /api/explorer/network/map

// Get analytics
GET /api/explorer/analytics
```

## 🚀 Production Deployment

### Build for Production

```bash
# Build the application
npm run build

# Preview the build
npm run preview
```

### Environment Variables

Create a `.env` file for production configuration:

```env
VITE_API_URL=https://api.ippan.network
VITE_EXPLORER_URL=https://explorer.ippan.network
VITE_NETWORK=mainnet
```

### Deployment

The built files will be in the `dist` directory and can be deployed to any static hosting service:

- **Netlify**: Drag and drop deployment
- **Vercel**: Git-based deployment
- **AWS S3**: Static website hosting
- **Cloudflare Pages**: Global CDN deployment

## 🆘 Troubleshooting

### Common Issues

1. **Port Already in Use**: Change port in `vite.config.ts`
2. **Dependencies Missing**: Run `npm install` again
3. **TypeScript Errors**: Check type definitions and imports
4. **Build Errors**: Check for syntax errors and missing dependencies

### Getting Help

1. **Documentation**: Check the main README and feature documentation
2. **Issues**: Create an issue in the repository
3. **Discussions**: Use GitHub discussions for questions
4. **Support**: Contact the development team

## 🎯 Next Steps

### For Developers

1. **Explore the Codebase**: Familiarize yourself with the project structure
2. **Test Features**: Test all enhanced wallet and explorer features
3. **Contribute**: Submit pull requests for improvements
4. **Documentation**: Help improve documentation

### For Users

1. **Try Features**: Test all wallet and explorer functionality
2. **Provide Feedback**: Report bugs and suggest improvements
3. **Share**: Share the application with others
4. **Support**: Help support the project

---

*This guide is maintained as part of the IPPAN project and reflects the current state of implementation as of 2024.*
