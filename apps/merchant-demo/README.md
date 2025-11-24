# IPPAN Merchant Demo

**A reference application demonstrating merchant payment flows on IPPAN blockchain**

---

## Overview

This demo shows how to build a simple e-commerce checkout using IPPAN for payments:

1. **Merchant** creates payment request with amount and description
2. **Customer** pays using IPPAN wallet (address or @handle)
3. **Merchant** receives payment confirmation and fulfills order

**Key Features:**
- RESTful API for order management
- Payment URI generation (for QR codes)
- Real-time payment verification
- Handle-based payments (`@merchant`, `@customer`)
- Order history and status tracking

---

## Prerequisites

- **Node.js** 18+ with npm
- **IPPAN node** running locally or testnet access
- **Merchant wallet** with IPPAN handle registered

---

## Installation

```bash
# Clone repository
cd IPPAN/apps/merchant-demo

# Install dependencies
npm install

# Build TypeScript
npm run build
```

---

## Configuration

Create `.env` file:

```env
# IPPAN node RPC endpoint
IPPAN_RPC_URL=http://localhost:8080

# Merchant's IPPAN address or handle
MERCHANT_ADDRESS=@merchant

# Server port
PORT=3000
```

---

## Running the Demo

### Start Server

```bash
npm start
```

Server starts on `http://localhost:3000`

### Create Payment Request

```bash
curl -X POST http://localhost:3000/api/orders \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "1000000",
    "description": "Premium T-Shirt",
    "currency": "IPN"
  }'
```

**Response:**
```json
{
  "orderId": "order_1732492800_abc123",
  "paymentAddress": "@merchant",
  "amount": "1000000",
  "currency": "IPN",
  "description": "Premium T-Shirt",
  "paymentUri": "ippan:@merchant?amount=1000000&memo=Premium%20T-Shirt&orderId=order_1732492800_abc123",
  "status": "pending",
  "expiresAt": 1732496400000
}
```

### Check Order Status

```bash
curl http://localhost:3000/api/orders/order_1732492800_abc123
```

**Response:**
```json
{
  "id": "order_1732492800_abc123",
  "amount": "1000000",
  "currency": "IPN",
  "description": "Premium T-Shirt",
  "merchantAddress": "@merchant",
  "status": "pending",
  "createdAt": 1732492800000
}
```

### Verify Payment

After customer pays, merchant verifies:

```bash
curl -X POST http://localhost:3000/api/orders/order_1732492800_abc123/verify \
  -H "Content-Type: application/json" \
  -d '{
    "txHash": "0xabc123..."
  }'
```

**Response:**
```json
{
  "verified": true,
  "order": {
    "id": "order_1732492800_abc123",
    "status": "paid",
    "paidAt": 1732493200000,
    "txHash": "0xabc123..."
  }
}
```

---

## Integration with IPPAN Wallet

### Customer Payment Flow

1. **Scan QR code** containing payment URI:
   ```
   ippan:@merchant?amount=1000000&memo=Premium%20T-Shirt&orderId=order_1732492800_abc123
   ```

2. **Wallet parses URI** and pre-fills payment form:
   - Recipient: `@merchant`
   - Amount: `0.001 IPN` (1,000,000 µIPN)
   - Memo: `Premium T-Shirt`

3. **Customer confirms** and signs transaction

4. **Wallet submits** to IPPAN node:
   ```bash
   cargo run -p ippan-wallet -- send \
     --to "@merchant" \
     --amount 1000000 \
     --memo "Premium T-Shirt" \
     --wallet customer.json
   ```

5. **Transaction finalizes** on blockchain (~400ms)

6. **Webhook notifies merchant** (if configured)

---

## API Reference

### Create Payment Request

**POST** `/api/orders`

**Request:**
```json
{
  "amount": "1000000",      // Amount in micro-IPN
  "description": "Product name",
  "currency": "IPN"         // Optional, default: "IPN"
}
```

**Response:**
```json
{
  "orderId": "order_...",
  "paymentAddress": "@merchant",
  "amount": "1000000",
  "paymentUri": "ippan:@merchant?...",
  "status": "pending",
  "expiresAt": 1732496400000
}
```

### Get Order Status

**GET** `/api/orders/:orderId`

**Response:**
```json
{
  "id": "order_...",
  "amount": "1000000",
  "status": "pending|paid|expired",
  "createdAt": 1732492800000,
  "paidAt": 1732493200000,  // If paid
  "txHash": "0x..."          // If paid
}
```

### Verify Payment

**POST** `/api/orders/:orderId/verify`

**Request:**
```json
{
  "txHash": "0xabc123..."
}
```

**Response:**
```json
{
  "verified": true,
  "order": { ... }
}
```

### List Orders

**GET** `/api/orders?status=paid&limit=10`

**Response:**
```json
{
  "orders": [ ... ],
  "total": 10
}
```

---

## Production Considerations

### Security

1. **Webhook Authentication:**
   ```typescript
   // Verify webhook signature
   const signature = req.headers['x-ippan-signature'];
   const isValid = verifySignature(req.body, signature, WEBHOOK_SECRET);
   ```

2. **Rate Limiting:**
   ```typescript
   import rateLimit from 'express-rate-limit';
   
   const limiter = rateLimit({
     windowMs: 15 * 60 * 1000, // 15 minutes
     max: 100 // limit each IP to 100 requests per windowMs
   });
   
   app.use('/api/', limiter);
   ```

3. **Input Validation:**
   ```typescript
   import { body, validationResult } from 'express-validator';
   
   app.post('/api/orders',
     body('amount').isInt({ min: 1 }),
     body('description').isString().trim().isLength({ min: 1, max: 200 }),
     (req, res) => {
       const errors = validationResult(req);
       if (!errors.isEmpty()) {
         return res.status(400).json({ errors: errors.array() });
       }
       // ...
     }
   );
   ```

### Database

Replace in-memory storage with PostgreSQL:

```typescript
import { Pool } from 'pg';

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
});

async function createOrder(order: Order): Promise<void> {
  await pool.query(
    'INSERT INTO orders (id, amount, description, status, created_at) VALUES ($1, $2, $3, $4, $5)',
    [order.id, order.amount, order.description, order.status, order.createdAt]
  );
}
```

### Monitoring

Add Prometheus metrics:

```typescript
import promClient from 'prom-client';

const orderCounter = new promClient.Counter({
  name: 'merchant_orders_total',
  help: 'Total orders created',
  labelNames: ['status']
});

// Increment on order creation
orderCounter.inc({ status: 'pending' });
```

---

## Testing

### Unit Tests

```bash
npm test
```

### End-to-End Test

```bash
# Terminal 1: Start IPPAN node
cargo run -p ippan-node -- --config config/devnet.toml

# Terminal 2: Start merchant demo
npm start

# Terminal 3: Create order and pay
ORDER_ID=$(curl -s -X POST http://localhost:3000/api/orders \
  -H "Content-Type: application/json" \
  -d '{"amount":"1000000","description":"Test"}' | jq -r '.orderId')

# Pay using wallet
cargo run -p ippan-wallet -- send \
  --to "@merchant" \
  --amount 1000000 \
  --memo "Test" \
  --wallet test.json

# Verify payment
curl -X POST http://localhost:3000/api/orders/$ORDER_ID/verify \
  -H "Content-Type: application/json" \
  -d '{"txHash":"0x..."}'
```

---

## Extending the Demo

### Add Product Catalog

```typescript
interface Product {
  id: string;
  name: string;
  price: string;
  description: string;
  imageUrl: string;
}

const products: Product[] = [
  { id: 'tshirt', name: 'Premium T-Shirt', price: '1000000', ... },
  { id: 'hoodie', name: 'IPPAN Hoodie', price: '5000000', ... },
];

app.get('/api/products', (req, res) => {
  res.json(products);
});
```

### Add Email Notifications

```typescript
import nodemailer from 'nodemailer';

const transporter = nodemailer.createTransport({ ... });

async function sendOrderConfirmation(order: Order, customerEmail: string) {
  await transporter.sendMail({
    to: customerEmail,
    subject: `Order ${order.id} Confirmed`,
    text: `Your order for ${order.description} has been paid. Thank you!`,
  });
}
```

---

## Resources

- **IPPAN SDK:** `apps/sdk-ts/README.md`
- **Wallet CLI:** `docs/dev/wallet-cli.md`
- **Payment API Guide:** `docs/PAYMENT_API_GUIDE.md`
- **Handle System:** `docs/users/handles-and-addresses.md`

---

## License

Apache 2.0

---

**Maintainers:**  
- IPPAN Development Team

**Last Updated:** 2025-11-24
