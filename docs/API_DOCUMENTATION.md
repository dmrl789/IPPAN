# IPPAN API Documentation

## Overview

IPPAN (InterPlanetary Network) provides a comprehensive API for blockchain operations, distributed storage, DNS management, and quantum-resistant cryptography. This document describes the available API endpoints, request/response formats, and usage examples.

## Base URL

```
http://localhost:8080/api/v1
```

## Authentication

Most API endpoints require authentication using Ed25519 signatures. Include the following headers:

```
X-IPPAN-Signature: <base64_signature>
X-IPPAN-Public-Key: <base64_public_key>
X-IPPAN-Timestamp: <unix_timestamp>
```

## Core API Endpoints

### 1. Blockchain Operations

#### 1.1 Submit Transaction
**POST** `/transactions/submit`

Submit a new transaction to the network.

**Request Body:**
```json
{
  "transaction_type": "payment",
  "sender": "i1sender1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "recipient": "i1recipient1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "amount": 1000000,
  "fee": 100,
  "memo": "Payment for services",
  "privacy_level": "confidential"
}
```

**Response:**
```json
{
  "success": true,
  "transaction_hash": "0x1234567890abcdef...",
  "status": "pending",
  "estimated_confirmation_time": 3000
}
```

#### 1.2 Get Transaction Status
**GET** `/transactions/{transaction_hash}`

Get the current status of a transaction.

**Response:**
```json
{
  "transaction_hash": "0x1234567890abcdef...",
  "status": "confirmed",
  "block_height": 12345,
  "confirmations": 6,
  "timestamp": 1640995200,
  "sender": "i1sender1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "recipient": "i1recipient1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "amount": 1000000,
  "fee": 100
}
```

#### 1.3 Get Account Balance
**GET** `/accounts/{address}/balance`

Get the current balance of an account.

**Response:**
```json
{
  "address": "i1sender1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "balance": 50000000,
  "pending_balance": 0,
  "total_received": 100000000,
  "total_sent": 50000000
}
```

### 2. Distributed Storage

#### 2.1 Store File
**POST** `/storage/store`

Store a file in the distributed storage network.

**Request Body:**
```json
{
  "file_name": "document.pdf",
  "file_size": 1048576,
  "file_hash": "sha256:abc123...",
  "replication_factor": 3,
  "encryption_enabled": true,
  "access_control": {
    "authorized_parties": ["i1user1234567890abcdef1234567890abcdef1234567890abcdef1234567890"],
    "permissions": ["read", "write"]
  }
}
```

**Response:**
```json
{
  "file_id": "f1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "storage_nodes": [
    "node1.ippan.network",
    "node2.ippan.network",
    "node3.ippan.network"
  ],
  "encryption_key_id": "k1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "status": "stored"
}
```

#### 2.2 Retrieve File
**GET** `/storage/retrieve/{file_id}`

Retrieve a file from distributed storage.

**Response:**
```json
{
  "file_id": "f1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "file_name": "document.pdf",
  "file_size": 1048576,
  "file_hash": "sha256:abc123...",
  "download_url": "https://storage.ippan.network/download/f1234567890abcdef...",
  "expires_at": 1640998800
}
```

### 3. DNS Management

#### 3.1 Create DNS Zone
**POST** `/dns/zones`

Create a new DNS zone.

**Request Body:**
```json
{
  "domain": "example.ipn",
  "owner": "i1owner1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "initial_records": [
    {
      "name": "@",
      "type": "A",
      "ttl": 300,
      "records": ["192.168.1.100"]
    }
  ]
}
```

**Response:**
```json
{
  "zone_id": "z1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "domain": "example.ipn",
  "owner": "i1owner1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "serial": 1,
  "status": "active"
}
```

#### 3.2 Update DNS Records
**POST** `/dns/zones/{zone_id}/records`

Update DNS records in a zone.

**Request Body:**
```json
{
  "operations": [
    {
      "operation": "upsert",
      "name": "www",
      "type": "A",
      "ttl": 300,
      "records": ["192.168.1.101"]
    },
    {
      "operation": "delete",
      "name": "old",
      "type": "A"
    }
  ]
}
```

### 4. Quantum-Resistant Cryptography

#### 4.1 Generate Quantum-Resistant Key Pair
**POST** `/quantum/keys/generate`

Generate a new quantum-resistant key pair.

**Request Body:**
```json
{
  "algorithm": "kyber",
  "security_level": "level3",
  "hybrid_encryption": true
}
```

**Response:**
```json
{
  "key_id": "qk1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "algorithm": "kyber",
  "security_level": "level3",
  "public_key": "base64:...",
  "key_size": 1184,
  "hybrid_encryption": true
}
```

#### 4.2 Encrypt Data
**POST** `/quantum/encrypt`

Encrypt data using quantum-resistant cryptography.

**Request Body:**
```json
{
  "data": "base64:...",
  "recipient_public_key": "base64:...",
  "algorithm": "kyber",
  "hybrid_encryption": true
}
```

**Response:**
```json
{
  "encrypted_data": "base64:...",
  "encapsulated_key": "base64:...",
  "algorithm": "kyber",
  "hybrid_scheme": "kyber-aes256-gcm"
}
```

### 5. Network Operations

#### 5.1 Get Network Status
**GET** `/network/status`

Get the current network status and statistics.

**Response:**
```json
{
  "network_id": "ippan-mainnet",
  "version": "1.0.0",
  "total_nodes": 1250,
  "active_nodes": 1180,
  "total_transactions": 1543200,
  "blocks_produced": 12345,
  "current_block_height": 12345,
  "consensus_participation": 0.95,
  "network_hashrate": "1.2 TH/s"
}
```

#### 5.2 Get Peer Information
**GET** `/network/peers`

Get information about connected peers.

**Response:**
```json
{
  "peers": [
    {
      "node_id": "n1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
      "address": "192.168.1.100:8080",
      "version": "1.0.0",
      "last_seen": 1640995200,
      "connection_quality": 0.95
    }
  ],
  "total_peers": 25
}
```

## Error Responses

All API endpoints return consistent error responses:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid transaction amount",
    "details": {
      "field": "amount",
      "value": -1000,
      "constraint": "must be positive"
    }
  }
}
```

### Common Error Codes

- `AUTHENTICATION_ERROR`: Invalid signature or missing authentication
- `VALIDATION_ERROR`: Invalid request data
- `NOT_FOUND`: Resource not found
- `INSUFFICIENT_FUNDS`: Account balance too low
- `NETWORK_ERROR`: Network communication error
- `STORAGE_ERROR`: Distributed storage error
- `QUANTUM_ERROR`: Quantum cryptography error

## Rate Limiting

API endpoints are rate-limited to prevent abuse:

- **Public endpoints**: 100 requests per minute
- **Authenticated endpoints**: 1000 requests per minute
- **Storage operations**: 50 requests per minute

Rate limit headers are included in responses:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640998800
```

## WebSocket API

For real-time updates, use the WebSocket API:

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onmessage = function(event) {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};

// Subscribe to transaction updates
ws.send(JSON.stringify({
  type: 'subscribe',
  channel: 'transactions',
  address: 'i1user1234567890abcdef1234567890abcdef1234567890abcdef1234567890'
}));
```

## SDK Libraries

Official SDK libraries are available for:

- **JavaScript/TypeScript**: `@ippan/sdk`
- **Python**: `ippan-sdk`
- **Rust**: `ippan-sdk`
- **Go**: `github.com/ippan/sdk`

## Examples

### Complete Transaction Flow

```javascript
const { IppanSDK } = require('@ippan/sdk');

const sdk = new IppanSDK({
  nodeUrl: 'http://localhost:8080',
  privateKey: 'your_private_key'
});

// Create and submit transaction
const tx = await sdk.transactions.create({
  type: 'payment',
  recipient: 'i1recipient1234567890abcdef1234567890abcdef1234567890abcdef1234567890',
  amount: 1000000,
  memo: 'Payment for services'
});

console.log('Transaction submitted:', tx.hash);

// Wait for confirmation
const status = await sdk.transactions.waitForConfirmation(tx.hash);
console.log('Transaction confirmed:', status);
```

### File Storage Example

```javascript
// Store file
const fileId = await sdk.storage.store({
  file: fileBuffer,
  fileName: 'document.pdf',
  replicationFactor: 3,
  encryption: true
});

// Retrieve file
const file = await sdk.storage.retrieve(fileId);
console.log('File retrieved:', file);
```

## Security Considerations

1. **Private Key Security**: Never expose private keys in client-side code
2. **HTTPS**: Always use HTTPS in production
3. **Input Validation**: Validate all user inputs
4. **Rate Limiting**: Respect rate limits to avoid being blocked
5. **Error Handling**: Handle errors gracefully and don't expose sensitive information

## Support

For API support and questions:

- **Documentation**: https://docs.ippan.network
- **GitHub**: https://github.com/ippan/ippan
- **Discord**: https://discord.gg/ippan
- **Email**: api-support@ippan.network
