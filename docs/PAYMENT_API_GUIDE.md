# IPPAN Payment API Guide

This guide documents the payment-related endpoints and CLI usage for the IPPAN blockchain network.

## Overview

IPPAN provides clean, RESTful APIs for submitting payments and querying payment history. All amounts are represented as **integer atomic units** with 24 decimal precision (yocto-IPN).

## Currency Precision

IPPAN uses ultra-fine atomic units to support micro-payments:

- **1 IPN = 10²⁴ atomic units** (yocto-IPN precision)
- All API responses return amounts as **u128 integers** in atomic units
- No floating-point values in transaction data

### Common Denominations

| Name      | Atomic Units | Example Use Case           |
|-----------|--------------|----------------------------|
| IPN       | 10²⁴         | Governance, staking        |
| milli-IPN | 10²¹         | Validator rewards          |
| micro-IPN | 10¹⁸         | Transaction fees           |
| nano-IPN  | 10¹⁵         | Micro-services             |
| pico-IPN  | 10¹²         | Sub-cent settlements       |

## Payment Endpoints

### POST /tx/payment

Submit a new payment transaction to the network.

**Request Body:**

```json
{
  "from": "hex-encoded 32-byte sender address",
  "to": "hex-encoded 32-byte recipient address",
  "amount": 1000000000000000000,
  "nonce": 1,
  "signature": "hex-encoded 64-byte signature"
}
```

**Response:**

- `200 OK` - Transaction accepted
- `400 Bad Request` - Invalid transaction data
- `500 Internal Server Error` - Processing failed

**Example:**

```bash
curl -X POST http://localhost:8080/tx \
  -H "Content-Type: application/json" \
  -d '{
    "from": "0123456789abcdef...",
    "to": "fedcba9876543210...",
    "amount": 1000000000000000000,
    "nonce": 1
  }'
```

### GET /tx/:hash

Retrieve a transaction by its hash.

**Path Parameters:**

- `hash` - 64-character hex-encoded transaction hash

**Response:**

```json
{
  "hash": "transaction_hash_hex",
  "transaction": {
    "id": "32-byte transaction ID",
    "from": "32-byte sender address",
    "to": "32-byte recipient address",
    "amount": 1000000000000000000,
    "nonce": 1,
    "timestamp": 1234567890,
    "hashtimer": "hashtimer_hex",
    "signature": "64-byte signature"
  }
}
```

**Example:**

```bash
curl http://localhost:8080/tx/abc123def456...
```

### GET /account/:address/payments

Retrieve paginated payment history for an address with direction metadata.

**Path Parameters:**

- `address` - 64-character hex-encoded account address

**Query Parameters:**

- `limit` (optional) - Maximum number of payments to return (default: 50, max: 200)
- `before` (optional) - Transaction hash to paginate from (returns transactions before this one)

**Response:**

```json
{
  "address": "account_address_hex",
  "payments": [
    {
      "tx_hash": "transaction_hash_hex",
      "from": "sender_address_hex",
      "to": "recipient_address_hex",
      "amount": 1000000000000000000,
      "fee": 0,
      "nonce": 1,
      "timestamp": 1234567890,
      "hash_timer": "hashtimer_hex",
      "direction": "outgoing"
    }
  ],
  "count": 1
}
```

**Direction Values:**

- `"incoming"` - Payment received by this address
- `"outgoing"` - Payment sent from this address
- `"self"` - Self-transfer (from and to are the same)

**Example - First page:**

```bash
curl "http://localhost:8080/account/0123456789abcdef.../payments?limit=50"
```

**Example - Paginated request:**

```bash
# Get first page
curl "http://localhost:8080/account/0123456789abcdef.../payments?limit=10"

# Get next page using last transaction hash from first page
curl "http://localhost:8080/account/0123456789abcdef.../payments?limit=10&before=abc123def456..."
```

**Notes:**

- Payments are sorted by timestamp in descending order (most recent first)
- Empty accounts return an empty `payments` array with `count: 0`
- Invalid addresses return `400 Bad Request`
- Storage errors return `500 Internal Server Error`

### GET /account/:address

Get account balance and recent transaction summary.

**Response:**

```json
{
  "address": "account_address_hex",
  "balance": 1000000000000000000,
  "nonce": 5,
  "transactions": [
    {
      "hash": "tx_hash_hex",
      "transaction": { ... }
    }
  ]
}
```

## CLI Usage

### ippan-cli pay

The IPPAN CLI provides a convenient `pay` command for sending payments.

#### Installation

```bash
cargo install --path crates/cli
```

#### Basic Payment

```bash
ippan-cli --rpc-url http://localhost:8080 wallet send \
  --from <sender_address> \
  --to <recipient_address> \
  --amount <amount_in_micro_ipn>
```

#### Examples

**Send 1 IPN:**

```bash
ippan-cli wallet send \
  --from 0123456789abcdef... \
  --to fedcba9876543210... \
  --amount 1000000
```

**Query transaction:**

```bash
ippan-cli transaction get abc123def456...
```

**Check account balance:**

```bash
ippan-cli wallet balance 0123456789abcdef...
```

**View payment history:**

```bash
# Using curl since CLI doesn't have a dedicated payments command yet
curl "http://localhost:8080/account/0123456789abcdef.../payments?limit=20"
```

## Integration Examples

### JavaScript/TypeScript

```typescript
// Submit a payment
async function sendPayment(from: string, to: string, amount: bigint) {
  const response = await fetch('http://localhost:8080/tx', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ from, to, amount: amount.toString(), nonce: 1 })
  });
  return await response.json();
}

// Get payment history
async function getPaymentHistory(address: string, limit: number = 50) {
  const response = await fetch(
    `http://localhost:8080/account/${address}/payments?limit=${limit}`
  );
  return await response.json();
}

// Usage
const history = await getPaymentHistory('0123456789abcdef...', 10);
console.log(`Found ${history.count} payments`);
history.payments.forEach(payment => {
  console.log(`${payment.direction}: ${payment.amount} atomic units`);
});
```

### Python

```python
import requests

def send_payment(from_addr, to_addr, amount):
    """Send a payment transaction"""
    response = requests.post('http://localhost:8080/tx', json={
        'from': from_addr,
        'to': to_addr,
        'amount': amount,
        'nonce': 1
    })
    return response.json()

def get_payment_history(address, limit=50, before=None):
    """Get paginated payment history for an address"""
    params = {'limit': limit}
    if before:
        params['before'] = before
    
    response = requests.get(
        f'http://localhost:8080/account/{address}/payments',
        params=params
    )
    return response.json()

# Usage
history = get_payment_history('0123456789abcdef...', limit=20)
print(f"Found {history['count']} payments")

for payment in history['payments']:
    direction = payment['direction']
    amount = payment['amount']
    print(f"{direction}: {amount} atomic units")
```

### Rust

```rust
use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct PaymentRequest {
    from: String,
    to: String,
    amount: u128,
    nonce: u64,
}

#[derive(Deserialize)]
struct PaymentHistoryResponse {
    address: String,
    payments: Vec<PaymentEntry>,
    count: usize,
}

#[derive(Deserialize)]
struct PaymentEntry {
    tx_hash: String,
    from: String,
    to: String,
    amount: u128,
    fee: u128,
    nonce: u64,
    timestamp: u64,
    hash_timer: String,
    direction: String,
}

async fn get_payment_history(
    client: &reqwest::Client,
    address: &str,
    limit: usize,
) -> Result<PaymentHistoryResponse, reqwest::Error> {
    let url = format!(
        "http://localhost:8080/account/{}/payments?limit={}",
        address, limit
    );
    let response = client.get(&url).send().await?;
    response.json().await
}

// Usage
#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();
    let history = get_payment_history(
        &client,
        "0123456789abcdef...",
        20
    ).await.unwrap();
    
    println!("Found {} payments", history.count);
    for payment in history.payments {
        println!("{}: {} atomic units", payment.direction, payment.amount);
    }
}
```

## Working with Atomic Units

### Converting to Human-Readable Amounts

**In JavaScript:**

```javascript
function atomicToIPN(atomic) {
  return Number(atomic) / 10**24;
}

function ipnToAtomic(ipn) {
  return BigInt(Math.floor(ipn * 10**24));
}
```

**In Python:**

```python
def atomic_to_ipn(atomic):
    return atomic / 10**24

def ipn_to_atomic(ipn):
    return int(ipn * 10**24)
```

**In Rust:**

```rust
const ATOMIC_PER_IPN: u128 = 10u128.pow(24);

fn atomic_to_ipn(atomic: u128) -> f64 {
    atomic as f64 / ATOMIC_PER_IPN as f64
}

fn ipn_to_atomic(ipn: f64) -> u128 {
    (ipn * ATOMIC_PER_IPN as f64) as u128
}
```

## Error Handling

### Common Error Codes

- `400 Bad Request` - Invalid address format, malformed request
- `404 Not Found` - Transaction or account not found
- `429 Too Many Requests` - Rate limit exceeded
- `500 Internal Server Error` - Storage or processing error
- `503 Service Unavailable` - Consensus not active or circuit breaker open

### Example Error Response

```json
{
  "error": "Invalid account address",
  "code": 400
}
```

## Best Practices

1. **Use Integer Arithmetic**: Always work with atomic units as integers to avoid floating-point precision issues.

2. **Paginate Large Histories**: Use the `limit` and `before` parameters to fetch payment history in manageable chunks.

3. **Cache Responses**: Payment history is immutable once finalized, so cache responses for efficiency.

4. **Handle Direction**: The `direction` field simplifies UI logic by indicating whether a payment is incoming, outgoing, or a self-transfer.

5. **Verify Finalization**: Check that transactions are included in finalized blocks before considering them confirmed.

6. **Rate Limiting**: Be aware of rate limits (200 requests/second by default) and implement appropriate backoff strategies.

## Security Considerations

- All endpoints respect IP whitelisting and security policies configured on the node
- Transaction signatures are verified before acceptance
- Payment history queries are subject to rate limiting
- No sensitive private key material is exposed via these endpoints

## Support

For issues, feature requests, or questions:

- GitHub: https://github.com/dmrl789/IPPAN
- Documentation: /workspace/docs/
- Developer Guide: /workspace/docs/DEVELOPER_GUIDE.md
