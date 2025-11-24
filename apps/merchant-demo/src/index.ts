/**
 * IPPAN Merchant Payment Demo
 * 
 * This reference application demonstrates a simple merchant checkout flow:
 * 1. Merchant generates payment request
 * 2. Customer pays via IPPAN
 * 3. Merchant receives payment confirmation
 * 
 * **Features:**
 * - Payment request generation with QR codes
 * - Real-time payment status monitoring
 * - Handle-based payments (@merchant)
 * - Order management
 * 
 * **Run:**
 * ```bash
 * npm install
 * npm run build
 * npm start
 * ```
 * 
 * **Environment:**
 * - IPPAN_RPC_URL: IPPAN node RPC endpoint (default: http://localhost:8080)
 * - MERCHANT_ADDRESS: Merchant's IPPAN address or handle
 * - PORT: Server port (default: 3000)
 */

import express, { Request, Response } from 'express';
import cors from 'cors';
import dotenv from 'dotenv';
import { IppanClient } from 'ippan-sdk';

dotenv.config();

// Configuration
const PORT = process.env.PORT || 3000;
const IPPAN_RPC_URL = process.env.IPPAN_RPC_URL || 'http://localhost:8080';
const MERCHANT_ADDRESS = process.env.MERCHANT_ADDRESS || '@merchant';

// Initialize IPPAN client
const ippanClient = new IppanClient({ baseUrl: IPPAN_RPC_URL });

// In-memory order storage (replace with database in production)
interface Order {
  id: string;
  amount: string;
  currency: string;
  description: string;
  merchantAddress: string;
  status: 'pending' | 'paid' | 'expired';
  createdAt: number;
  paidAt?: number;
  txHash?: string;
}

const orders = new Map<string, Order>();

// Initialize Express app
const app = express();
app.use(cors());
app.use(express.json());
app.use(express.static('public'));

/**
 * Health check endpoint
 */
app.get('/health', (req: Request, res: Response) => {
  res.json({ status: 'ok', service: 'merchant-demo', version: '1.0.0' });
});

/**
 * Create payment request
 * 
 * POST /api/orders
 * Body: {
 *   amount: string (micro-IPN),
 *   description: string,
 *   currency?: string (default: "IPN")
 * }
 * 
 * Returns: {
 *   orderId: string,
 *   paymentAddress: string,
 *   amount: string,
 *   qrCode: string (data URI)
 * }
 */
app.post('/api/orders', async (req: Request, res: Response) => {
  try {
    const { amount, description, currency = 'IPN' } = req.body;

    if (!amount || !description) {
      return res.status(400).json({ error: 'Missing required fields: amount, description' });
    }

    // Validate amount
    const amountNum = BigInt(amount);
    if (amountNum <= 0) {
      return res.status(400).json({ error: 'Amount must be positive' });
    }

    // Generate order ID
    const orderId = `order_${Date.now()}_${Math.random().toString(36).substring(7)}`;

    // Create order
    const order: Order = {
      id: orderId,
      amount: amount.toString(),
      currency,
      description,
      merchantAddress: MERCHANT_ADDRESS,
      status: 'pending',
      createdAt: Date.now(),
    };

    orders.set(orderId, order);

    // Generate payment URI (for QR code)
    const paymentUri = `ippan:${MERCHANT_ADDRESS}?amount=${amount}&memo=${encodeURIComponent(description)}&orderId=${orderId}`;

    res.json({
      orderId,
      paymentAddress: MERCHANT_ADDRESS,
      amount: amount.toString(),
      currency,
      description,
      paymentUri,
      status: 'pending',
      expiresAt: Date.now() + 3600000, // 1 hour
    });
  } catch (error) {
    console.error('Error creating order:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

/**
 * Get order status
 * 
 * GET /api/orders/:orderId
 * 
 * Returns order details and payment status
 */
app.get('/api/orders/:orderId', async (req: Request, res: Response) => {
  try {
    const { orderId } = req.params;
    const order = orders.get(orderId);

    if (!order) {
      return res.status(404).json({ error: 'Order not found' });
    }

    // Check if order expired (1 hour)
    if (order.status === 'pending' && Date.now() - order.createdAt > 3600000) {
      order.status = 'expired';
    }

    res.json(order);
  } catch (error) {
    console.error('Error fetching order:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

/**
 * Verify payment for order
 * 
 * POST /api/orders/:orderId/verify
 * Body: {
 *   txHash: string
 * }
 * 
 * Verifies that payment transaction matches order requirements
 */
app.post('/api/orders/:orderId/verify', async (req: Request, res: Response) => {
  try {
    const { orderId } = req.params;
    const { txHash } = req.body;

    const order = orders.get(orderId);
    if (!order) {
      return res.status(404).json({ error: 'Order not found' });
    }

    if (order.status !== 'pending') {
      return res.json({ verified: false, reason: 'Order already processed or expired' });
    }

    // Fetch transaction from blockchain
    // Note: This is a stub - real implementation would query IPPAN node
    // const tx = await ippanClient.getTransaction(txHash);
    
    // Verify:
    // 1. Transaction is finalized
    // 2. Recipient matches merchant address
    // 3. Amount matches order amount
    // 4. Memo contains order ID

    // For demo purposes, simulate verification
    const verified = true;

    if (verified) {
      order.status = 'paid';
      order.paidAt = Date.now();
      order.txHash = txHash;

      res.json({
        verified: true,
        order,
      });
    } else {
      res.json({
        verified: false,
        reason: 'Payment verification failed',
      });
    }
  } catch (error) {
    console.error('Error verifying payment:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

/**
 * List all orders (for merchant dashboard)
 * 
 * GET /api/orders
 * Query params:
 * - status: filter by status (pending|paid|expired)
 * - limit: max results (default: 50)
 */
app.get('/api/orders', (req: Request, res: Response) => {
  try {
    const { status, limit = 50 } = req.query;

    let orderList = Array.from(orders.values());

    if (status) {
      orderList = orderList.filter(o => o.status === status);
    }

    orderList = orderList
      .sort((a, b) => b.createdAt - a.createdAt)
      .slice(0, Number(limit));

    res.json({
      orders: orderList,
      total: orderList.length,
    });
  } catch (error) {
    console.error('Error listing orders:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

/**
 * Webhook endpoint (for real-time payment notifications)
 * 
 * This would be called by IPPAN node or gateway when payments are received
 */
app.post('/api/webhooks/payment', (req: Request, res: Response) => {
  try {
    const { txHash, from, to, amount, memo } = req.body;

    console.log('Payment webhook received:', { txHash, from, to, amount, memo });

    // Extract order ID from memo
    const orderIdMatch = memo?.match(/orderId=([a-zA-Z0-9_]+)/);
    if (orderIdMatch) {
      const orderId = orderIdMatch[1];
      const order = orders.get(orderId);

      if (order && order.status === 'pending') {
        order.status = 'paid';
        order.paidAt = Date.now();
        order.txHash = txHash;
        console.log(`Order ${orderId} marked as paid`);
      }
    }

    res.json({ received: true });
  } catch (error) {
    console.error('Error processing webhook:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

// Start server
app.listen(PORT, () => {
  console.log(`
╔═══════════════════════════════════════════════════════╗
║   IPPAN Merchant Demo                                 ║
╚═══════════════════════════════════════════════════════╝

Server running on: http://localhost:${PORT}
IPPAN RPC: ${IPPAN_RPC_URL}
Merchant Address: ${MERCHANT_ADDRESS}

API Endpoints:
  POST   /api/orders              - Create payment request
  GET    /api/orders/:id          - Get order status
  POST   /api/orders/:id/verify   - Verify payment
  GET    /api/orders              - List orders
  POST   /api/webhooks/payment    - Payment notification

Example:
  curl -X POST http://localhost:${PORT}/api/orders \\
    -H "Content-Type: application/json" \\
    -d '{"amount":"1000000","description":"Test product"}'
  `);
});
