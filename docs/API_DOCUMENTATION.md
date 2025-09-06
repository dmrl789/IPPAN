# IPPAN API Documentation

## Overview

IPPAN (InterPlanetary Network) provides a comprehensive API for blockchain operations, distributed storage, DNS management, quantum-resistant cryptography, and high-performance operations. This document describes the available API endpoints, request/response formats, and usage examples.

## 🚀 Performance Features

- **1-10 Million TPS**: Optimized for high-throughput operations
- **Lock-Free Data Structures**: High-performance concurrent operations
- **Memory Pooling**: Zero-copy operations with efficient memory reuse
- **Batch Processing**: Parallel batch processing with configurable thread pools
- **Multi-Level Caching**: L1/L2 cache hierarchy for optimal data access
- **High-Performance Serialization**: Optimized data serialization/deserialization

## 🔒 Security Features

- **Quantum-Resistant Cryptography**: Post-quantum cryptographic algorithms
- **Ed25519 Signatures**: High-performance digital signatures
- **Key Management**: Secure key storage with automatic rotation
- **Network Security**: TLS/SSL with mutual authentication
- **Rate Limiting**: API rate limiting and DDoS protection
- **Audit Logging**: Complete audit trail for all operations

## Base URL

```
http://localhost:8080/api/v1
```

## Canonical JSON Schema

All IPPAN API responses follow the canonical JSON schema specification. See [JSON Schema Specification](./JSON_SCHEMA_SPECIFICATION.md) for complete details.

### Key Schema Features

- **Stringified Big Integers**: All large numbers are strings to avoid JavaScript 53-bit limitations
- **HashTimer v1**: Complete timing structure with nanosecond precision
- **Deterministic Ordering**: Canonical ordering for transactions and blocks
- **16-byte Node IDs**: Proper 32 hex character node identifiers
- **32-byte Digests**: All hashes are 64 hex characters

### Example Schema Response

```json
{
  "version": "v1",
  "round_id": "8784975040",
  "state": "finalized",
  "time": { 
    "start_ns": "1756995008000000000", 
    "end_ns": "1756995008250000000" 
  },
  "block_count": 8,
  "zk_stark_proof": "b597133e7c45d8c0b3b0c9a2b1f0f9aa9c00aa11bb22cc33dd44ee55ff667788",
  "merkle_root": "c5d42a59e1ae68e1c2a9ff00bb11aa22cc33dd44ee55ff66778899aabbccddee",
  "blocks": [...]
}
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

#### 1.3 Get Block Information
**GET** `/blocks/{block_hash}`

Get detailed information about a specific block including parent relationships.

**Response:**
```json
{
  "block_id": "block-54",
  "producer": {
    "node_id": "76687a7e80849ea0c7c0a0d9f4d0a3b1",
    "label": "validator-1"
  },
  "status": "finalized",
  "tx_count": 54,
  "header_digest": "3440434551535e626ca478b18893979da2dbb5bbbfc4cf07dd15eb23f9310209",
  "parents": [
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  ],
  "parent_rounds": ["8784975037", "8784975039"],
  "hashtimer": {
    "version": "v1",
    "time": { 
      "t_ns": "1756995008183000000", 
      "precision_ns": 100, 
      "drift_ns": "-116" 
    },
    "position": { 
      "round": "8784975040", 
      "seq": 1, 
      "kind": "Block" 
    },
    "node_id": "76687a7e80849ea0c7c0a0d9f4d0a3b1",
    "payload_digest": "a847df59a9e8a893351ba4d26508cdefa2b4c6d8e0f1a2b3c4d5e6f708192a3b",
    "hash_timer_digest": "1f5b7c3a2e0d4c9b8a76f5e4d3c2b1a0ffeeddccbbaa99887766554433221100"
  },
  "txs": [...]
}
```

#### 1.4 Get Block Parents
**GET** `/blocks/{block_hash}/parents`

Get the parent blocks of a specific block.

**Response:**
```json
{
  "block_hash": "3440434551535e626ca478b18893979da2dbb5bbbfc4cf07dd15eb23f9310209",
  "parents": [
    {
      "parent_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "parent_round": "8784975037",
      "parent_height": 12343
    },
    {
      "parent_hash": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "parent_round": "8784975039",
      "parent_height": 12344
    }
  ]
}
```

#### 1.5 Get Block Ancestors
**GET** `/blocks/{block_hash}/ancestors?max_depth=10`

Get all ancestor blocks up to a specified depth.

**Query Parameters:**
- `max_depth` (optional): Maximum depth to traverse (default: 10, max: 100)

**Response:**
```json
{
  "block_hash": "3440434551535e626ca478b18893979da2dbb5bbbfc4cf07dd15eb23f9310209",
  "ancestors": [
    {
      "ancestor_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "ancestor_round": "8784975037",
      "ancestor_height": 12343,
      "depth": 1
    },
    {
      "ancestor_hash": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "ancestor_round": "8784975035",
      "ancestor_height": 12341,
      "depth": 2
    }
  ]
}
```

#### 1.6 Get Block Descendants
**GET** `/blocks/{block_hash}/descendants?max_depth=10`

Get all descendant blocks up to a specified depth.

**Query Parameters:**
- `max_depth` (optional): Maximum depth to traverse (default: 10, max: 100)

**Response:**
```json
{
  "block_hash": "3440434551535e626ca478b18893979da2dbb5bbbfc4cf07dd15eb23f9310209",
  "descendants": [
    {
      "descendant_hash": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
      "descendant_round": "8784975041",
      "descendant_height": 12346,
      "depth": 1
    }
  ]
}
```

#### 1.7 Get Round Information
**GET** `/rounds/{round_id}`

Get information about a specific consensus round including all blocks.

**Response:**
```json
{
  "version": "v1",
  "round_id": "8784975040",
  "state": "finalized",
  "time": { 
    "start_ns": "1756995008000000000", 
    "end_ns": "1756995008250000000" 
  },
  "block_count": 8,
  "zk_stark_proof": "b597133e7c45d8c0b3b0c9a2b1f0f9aa9c00aa11bb22cc33dd44ee55ff667788",
  "merkle_root": "c5d42a59e1ae68e1c2a9ff00bb11aa22cc33dd44ee55ff66778899aabbccddee",
  "blocks": [
    {
      "block_id": "block-54",
      "producer": {
        "node_id": "76687a7e80849ea0c7c0a0d9f4d0a3b1",
        "label": "validator-1"
      },
      "status": "finalized",
      "tx_count": 54,
      "header_digest": "3440434551535e626ca478b18893979da2dbb5bbbfc4cf07dd15eb23f9310209",
      "parents": [
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
      ],
      "parent_rounds": ["8784975037", "8784975039"],
      "hashtimer": {...},
      "txs": [...]
    }
  ]
}
```

#### 1.8 Get Account Balance
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
