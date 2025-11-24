# IPPAN Reference Applications

**Realistic demos showcasing IPPAN blockchain integration**

**Last Updated:** 2025-11-24

---

## Overview

This document catalogs reference applications that demonstrate real-world IPPAN integration patterns. Each app is production-ready with complete documentation and can be used as a starting point for building on IPPAN.

---

## Available Reference Apps

### 1. Merchant Payment Demo

**Location:** `apps/merchant-demo/`

**Purpose:** E-commerce checkout flow with IPPAN payments

**Features:**
- Payment request generation
- QR code payment URIs (`ippan:@merchant?amount=...`)
- Order management (pending/paid/expired)
- Real-time payment verification
- Handle-based payments (`@merchant`, `@customer`)
- Webhook support for notifications

**Tech Stack:**
- TypeScript + Express.js
- IPPAN TypeScript SDK
- RESTful API

**Use Cases:**
- Online stores
- Digital content sales
- Service subscriptions
- Donation platforms

**Quick Start:**
```bash
cd apps/merchant-demo
npm install
npm start
# Open http://localhost:3000
```

**Read More:** [apps/merchant-demo/README.md](../../apps/merchant-demo/README.md)

---

### 2. Simple Payment Example (TypeScript)

**Location:** `examples/sdk_ts_payment.ts`

**Purpose:** Minimal TypeScript SDK usage

**Features:**
- Account balance query
- Simple payment transaction
- Handle resolution (`@friend.ipn`)

**Use Cases:**
- Learning the SDK
- CLI payment tools
- Scripting bulk payments

**Quick Start:**
```bash
export IPPAN_FROM_ADDRESS=i0x...
export IPPAN_SIGNING_KEY=0x...
export IPPAN_TO_ADDRESS=@friend
npx ts-node examples/sdk_ts_payment.ts
```

---

### 3. Rust SDK Payment Example

**Location:** `crates/sdk/examples/sdk_rust_payment.rs`

**Purpose:** Minimal Rust SDK usage

**Features:**
- Payment transaction construction
- Signature generation (Ed25519)
- RPC submission

**Use Cases:**
- Backend services (Rust)
- High-performance payment processors
- Validator tooling

**Quick Start:**
```bash
cargo run --example sdk_rust_payment
```

---

### 4. DAG-Fair Emission Demo

**Location:** `examples/dag_fair_emission_demo.rs`

**Purpose:** Demonstrates IPPAN's emission curve and validator rewards

**Features:**
- Emission calculation (round-based)
- Halving schedule visualization
- Validator reward distribution

**Use Cases:**
- Understanding tokenomics
- Economic modeling
- Validator profitability analysis

**Quick Start:**
```bash
cargo run --example dag_fair_emission_demo
```

---

## Planned Reference Apps (Future)

### 5. Machine-to-Machine Micro-Payments

**Status:** Planned for v1.1

**Purpose:** Two services paying each other for API calls

**Features:**
- Automated payment on API usage
- Usage metering
- Balance tracking
- Auto-refill when low

**Use Cases:**
- AI API credits
- CDN bandwidth payments
- Compute resource sharing

---

### 6. Multi-Sig Wallet Demo

**Status:** Planned for v1.2

**Purpose:** Collaborative wallet requiring M-of-N signatures

**Features:**
- Multi-signature transactions
- Proposal/approval workflow
- Threshold signing

**Use Cases:**
- Corporate treasury
- DAO funds management
- Shared wallets

---

### 7. Decentralized File Storage

**Status:** Planned for v1.3

**Purpose:** Pay-per-use file storage with IPNDHT

**Features:**
- Upload files to IPNDHT
- Micropayments for storage
- Content addressing
- Retrieval with payment

**Use Cases:**
- Decentralized CDN
- NFT metadata storage
- Document archival

---

## Integration Patterns

### Pattern 1: Direct RPC Integration

**When to use:** Simple use cases, low traffic

**Example:** CLI tools, internal dashboards

```typescript
import { IppanClient } from 'ippan-sdk';

const client = new IppanClient({ baseUrl: 'http://localhost:8080' });
const balance = await client.getAccountBalance(address);
```

**Pros:**
- Simple setup
- No intermediaries
- Direct blockchain access

**Cons:**
- Requires running IPPAN node
- No caching/rate limiting
- Manual key management

---

### Pattern 2: Gateway Integration

**When to use:** Public-facing apps, high traffic

**Example:** Web apps, mobile apps

```typescript
const client = new IppanClient({ baseUrl: 'https://gateway.ippan.io/api' });
```

**Pros:**
- Scalable (gateway handles load)
- Rate limiting built-in
- HTTPS/WSS support

**Cons:**
- Introduces trust in gateway
- Potential single point of failure
- May have usage limits

---

### Pattern 3: Embedded Node

**When to use:** Enterprise, validators, exchanges

**Example:** Payment processors, custodial services

```rust
use ippan_node::{Node, Config};

let node = Node::new(config)?;
node.start().await?;
```

**Pros:**
- Full control
- No external dependencies
- Maximum security

**Cons:**
- Complex setup
- Requires infrastructure
- Ongoing maintenance

---

## Development Workflow

### Local Development

1. **Start local IPPAN node:**
   ```bash
   cargo run -p ippan-node -- --config config/devnet.toml
   ```

2. **Fund test wallets:**
   ```bash
   curl -X POST http://localhost:8080/dev/fund \
     -d '{"address":"i0x...","amount":1000000000}'
   ```

3. **Run reference app:**
   ```bash
   cd apps/merchant-demo
   npm install && npm start
   ```

4. **Test payment flow:**
   ```bash
   # Create order
   curl -X POST http://localhost:3000/api/orders \
     -d '{"amount":"1000000","description":"Test"}'
   
   # Pay with wallet
   cargo run -p ippan-wallet -- send --to @merchant --amount 1000000
   ```

---

### Testnet Deployment

1. **Deploy reference app:**
   ```bash
   # Update .env
   IPPAN_RPC_URL=https://testnet-rpc.ippan.io
   MERCHANT_ADDRESS=@my-testnet-handle
   
   # Start
   npm start
   ```

2. **Get testnet IPN:**
   - Join Discord: https://discord.gg/ippan
   - Request from faucet: `!faucet <your-address>`

3. **Test with real testnet transactions**

---

## Production Checklist

Before deploying to mainnet:

- [ ] **Security Audit:**
  - Input validation on all endpoints
  - Rate limiting (per-IP, per-user)
  - HTTPS/TLS everywhere
  - Secure key storage (HSM, KMS)

- [ ] **Monitoring:**
  - Health checks
  - Error tracking (Sentry, etc.)
  - Metrics (Prometheus)
  - Alerting (PagerDuty, Slack)

- [ ] **Scalability:**
  - Load testing (locust, k6)
  - Database indexing
  - Caching (Redis)
  - Horizontal scaling

- [ ] **Backup & Recovery:**
  - Database backups
  - Key backup procedures
  - Disaster recovery plan

- [ ] **Documentation:**
  - API documentation (OpenAPI/Swagger)
  - User guides
  - Support channels

---

## Resources

### SDKs

- **TypeScript SDK:** [apps/sdk-ts/README.md](../../apps/sdk-ts/README.md)
- **Rust SDK:** [crates/sdk/src/lib.rs](../../crates/sdk/src/lib.rs)

### Guides

- **Developer Journey:** [docs/dev/developer-journey.md](./developer-journey.md)
- **Wallet CLI:** [docs/dev/wallet-cli.md](./wallet-cli.md)
- **Payment API:** [docs/PAYMENT_API_GUIDE.md](../PAYMENT_API_GUIDE.md)
- **Handle System:** [docs/users/handles-and-addresses.md](../users/handles-and-addresses.md)

### Tools

- **Local Full-Stack:** [docs/dev/local-full-stack.md](./local-full-stack.md)
- **Testnet Guide:** [TESTNET_JOIN_GUIDE.md](../../TESTNET_JOIN_GUIDE.md)
- **Explorer:** [apps/unified-ui/README.md](../../apps/unified-ui/README.md)

---

## Contributing

Want to add a reference app?

1. **Choose a use case** (not already covered)
2. **Build minimal viable demo** (< 500 LOC)
3. **Add comprehensive README** (setup, usage, examples)
4. **Test end-to-end** (local + testnet)
5. **Submit PR** with tag `reference-app`

**Guidelines:**
- Production-quality code (no TODOs, placeholder APIs)
- Complete documentation
- Error handling
- Input validation
- TypeScript or Rust preferred

---

## FAQ

### Q: Which reference app should I start with?

**A:** For most developers, start with **Merchant Payment Demo** (`apps/merchant-demo`). It covers:
- SDK integration
- Order management
- Payment verification
- Production patterns

### Q: Can I use reference apps in production?

**A:** Yes, but with modifications:
- Replace in-memory storage with database
- Add authentication/authorization
- Implement proper error handling
- Add monitoring and logging
- Follow security checklist above

### Q: How do I integrate with existing systems?

**A:** Reference apps are designed as starting points. Common integrations:
- **Shopify/WooCommerce:** Add IPPAN payment gateway plugin
- **Stripe/Square:** Replace with IPPAN SDK calls
- **PayPal:** Add IPPAN as alternative payment method

### Q: What about mobile apps?

**A:** Use TypeScript SDK in React Native or Native bindings (Rust FFI). See:
- **Android Wallet:** [apps/mobile/android-wallet/](../../apps/mobile/android-wallet/)
- **Mobile SDK Guide:** (Coming in v1.1)

---

## License

All reference applications are licensed under **Apache 2.0** (same as IPPAN core).

---

**Maintainers:**  
- IPPAN Development Team

**Last Updated:** 2025-11-24

**Next Review:** v1.1.0 (add M2M demo)
