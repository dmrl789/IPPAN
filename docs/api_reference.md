# 🔌 IPPAN API Reference

This document provides comprehensive API documentation for the IPPAN network, including REST endpoints, WebSocket APIs, and client libraries.

## Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [REST API](#rest-api)
4. [WebSocket API](#websocket-api)
5. [Error Handling](#error-handling)
6. [Rate Limiting](#rate-limiting)
7. [Client Libraries](#client-libraries)
8. [Examples](#examples)

## Overview

The IPPAN API provides programmatic access to all network functionality:

- **Node Management**: Start, stop, and monitor nodes
- **Storage Operations**: Upload, download, and manage files
- **Wallet Operations**: Send payments and manage M2M channels
- **Domain Management**: Register and manage domains
- **Network Information**: Get network statistics and peer information
- **Consensus Data**: Access blockchain and consensus information

### Base URL

```
http://localhost:3000/api/v1
```

### Content Types

- **Request**: `application/json`
- **Response**: `application/json`
- **File Upload**: `multipart/form-data`

## Authentication

### API Key Authentication

```bash
# Include API key in headers
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:3000/api/v1/status
```

### Node Authentication

```bash
# Include node signature in headers
curl -H "X-Node-Signature: SIGNATURE" \
     -H "X-Node-ID: NODE_ID" \
     http://localhost:3000/api/v1/status
```

## REST API

### Node Management

#### Get Node Status

```http
GET /api/v1/status
```

**Response:**
```json
{
  "status": "running",
  "uptime": 3600,
  "version": "1.0.0",
  "node_id": "abc123...",
  "network": {
    "connected_peers": 15,
    "total_peers": 150
  },
  "storage": {
    "used_bytes": 1073741824,
    "total_bytes": 107374182400,
    "files_count": 1250
  },
  "consensus": {
    "current_round": 12345,
    "is_validator": true,
    "stake_amount": 50000000000
  }
}
```

#### Start Node

```http
POST /api/v1/node/start
```

**Request:**
```json
{
  "config": {
    "network_port": 8080,
    "api_port": 3000,
    "storage_dir": "/path/to/storage"
  }
}
```

**Response:**
```json
{
  "success": true,
  "message": "Node started successfully",
  "node_id": "abc123..."
}
```

#### Stop Node

```http
POST /api/v1/node/stop
```

**Response:**
```json
{
  "success": true,
  "message": "Node stopped successfully"
}
```

### Storage Operations

#### Upload File

```http
POST /api/v1/storage/upload
Content-Type: multipart/form-data
```

**Request:**
```bash
curl -X POST http://localhost:3000/api/v1/storage/upload \
  -F "file=@/path/to/file.txt" \
  -F "name=my-document" \
  -F "encrypt=true"
```

**Response:**
```json
{
  "success": true,
  "file_hash": "abc123...",
  "file_size": 1024,
  "encrypted": true,
  "upload_time": "2024-01-01T12:00:00Z"
}
```

#### Download File

```http
GET /api/v1/storage/download/{file_hash}
```

**Response:**
```json
{
  "success": true,
  "file_data": "base64_encoded_data",
  "file_name": "my-document.txt",
  "file_size": 1024,
  "download_time": "2024-01-01T12:00:00Z"
}
```

#### Get File Info

```http
GET /api/v1/storage/info/{file_hash}
```

**Response:**
```json
{
  "success": true,
  "file_hash": "abc123...",
  "file_name": "my-document.txt",
  "file_size": 1024,
  "upload_time": "2024-01-01T12:00:00Z",
  "encrypted": true,
  "available": true,
  "replicas": 3
}
```

#### List Files

```http
GET /api/v1/storage/list?page=1&limit=10
```

**Response:**
```json
{
  "success": true,
  "files": [
    {
      "file_hash": "abc123...",
      "file_name": "document1.txt",
      "file_size": 1024,
      "upload_time": "2024-01-01T12:00:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 10,
    "total": 1250,
    "pages": 125
  }
}
```

### Wallet Operations

#### Get Wallet Balance

```http
GET /api/v1/wallet/balance
```

**Response:**
```json
{
  "success": true,
  "balance": {
    "total": 1000000000000,
    "available": 950000000000,
    "staked": 50000000000,
    "locked": 0
  },
  "currency": "IPN",
  "decimals": 8
}
```

#### Send Payment

```http
POST /api/v1/wallet/send
```

**Request:**
```json
{
  "to": "recipient_address",
  "amount": 1000000000,
  "fee": 10000000,
  "memo": "Payment for services"
}
```

**Response:**
```json
{
  "success": true,
  "transaction_hash": "tx123...",
  "amount": 1000000000,
  "fee": 10000000,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

#### Get Transaction History

```http
GET /api/v1/wallet/history?page=1&limit=10
```

**Response:**
```json
{
  "success": true,
  "transactions": [
    {
      "hash": "tx123...",
      "type": "send",
      "amount": 1000000000,
      "fee": 10000000,
      "timestamp": "2024-01-01T12:00:00Z",
      "status": "confirmed"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 10,
    "total": 150,
    "pages": 15
  }
}
```

### M2M Payment Channels

#### Create Payment Channel

```http
POST /api/v1/wallet/channels
```

**Request:**
```json
{
  "recipient": "recipient_address",
  "amount": 10000000000,
  "duration_hours": 24,
  "description": "IoT device payment channel"
}
```

**Response:**
```json
{
  "success": true,
  "channel_id": "ch123...",
  "recipient": "recipient_address",
  "total_amount": 10000000000,
  "remaining_amount": 10000000000,
  "expires_at": "2024-01-02T12:00:00Z"
}
```

#### Send Micro-Payment

```http
POST /api/v1/wallet/channels/{channel_id}/pay
```

**Request:**
```json
{
  "amount": 1000000,
  "type": "data_transfer",
  "metadata": {
    "bytes_transferred": 1024,
    "service_type": "sensor_data"
  }
}
```

**Response:**
```json
{
  "success": true,
  "payment_id": "pay123...",
  "amount": 1000000,
  "remaining_amount": 9999000000,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### Domain Management

#### Register Domain

```http
POST /api/v1/domains
```

**Request:**
```json
{
  "name": "alice.ipn",
  "data": "https://alice.com",
  "duration_years": 1
}
```

**Response:**
```json
{
  "success": true,
  "domain": "alice.ipn",
  "owner": "owner_address",
  "expires_at": "2025-01-01T12:00:00Z",
  "registration_fee": 1000000000
}
```

#### Get Domain Info

```http
GET /api/v1/domains/{domain_name}
```

**Response:**
```json
{
  "success": true,
  "domain": "alice.ipn",
  "owner": "owner_address",
  "data": "https://alice.com",
  "created_at": "2024-01-01T12:00:00Z",
  "expires_at": "2025-01-01T12:00:00Z",
  "status": "active"
}
```

#### Update Domain

```http
PUT /api/v1/domains/{domain_name}
```

**Request:**
```json
{
  "data": "https://new-alice.com"
}
```

**Response:**
```json
{
  "success": true,
  "domain": "alice.ipn",
  "updated_at": "2024-01-01T12:00:00Z"
}
```

### Network Information

#### Get Network Stats

```http
GET /api/v1/network/stats
```

**Response:**
```json
{
  "success": true,
  "total_nodes": 1500,
  "active_nodes": 1200,
  "total_storage": 1500000000000000,
  "used_storage": 750000000000000,
  "total_transactions": 15000000,
  "average_block_time": 10.5
}
```

#### Get Connected Peers

```http
GET /api/v1/network/peers
```

**Response:**
```json
{
  "success": true,
  "peers": [
    {
      "node_id": "peer123...",
      "address": "192.168.1.100:8080",
      "last_seen": "2024-01-01T12:00:00Z",
      "latency_ms": 50
    }
  ],
  "total_peers": 15
}
```

### Consensus Information

#### Get Consensus Status

```http
GET /api/v1/consensus/status
```

**Response:**
```json
{
  "success": true,
  "current_round": 12345,
  "current_block": "block123...",
  "validators_count": 100,
  "total_stake": 50000000000000,
  "ippan_time": "2024-01-01T12:00:00.000100Z"
}
```

### L2 Blockchain Integration

#### Submit L2 Settlement

```http
POST /api/v1/l2/settlement
```

**Request:**
```json
{
  "l2_chain_id": 12345,
  "l2_block_hash": "block123...",
  "l2_state_root": "state123...",
  "settlement_amount": 1000000000,
  "metadata": {
    "l2_chain_name": "L2_Chain_A",
    "transaction_count": 150
  }
}
```

**Response:**
```json
{
  "success": true,
  "transaction_hash": "tx123...",
  "l2_chain_id": 12345,
  "settlement_amount": 1000000000,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

#### Store L2 Data

```http
POST /api/v1/l2/data
```

**Request:**
```json
{
  "l2_chain_id": 12345,
  "data_type": "state_update",
  "data_hash": "data123...",
  "data_size": 1024,
  "data": "base64_encoded_data"
}
```

**Response:**
```json
{
  "success": true,
  "file_hash": "file123...",
  "l2_chain_id": 12345,
  "data_type": "state_update",
  "storage_fee": 500000,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

#### Get L2 Integration Status

```http
GET /api/v1/l2/status
```

**Response:**
```json
{
  "success": true,
  "integrated_l2_chains": [
    {
      "chain_id": 12345,
      "chain_name": "L2_Chain_A",
      "settlement_count": 150,
      "data_storage_mb": 1024,
      "last_settlement": "2024-01-01T12:00:00Z"
    }
  ],
  "total_settlements": 1500,
  "total_data_storage_mb": 10240
}
```

#### Get Recent Blocks

```http
GET /api/v1/consensus/blocks?limit=10
```

**Response:**
```json
{
  "success": true,
  "blocks": [
    {
      "hash": "block123...",
      "round": 12345,
      "timestamp": "2024-01-01T12:00:00Z",
      "transactions_count": 150,
      "validator": "validator123..."
    }
  ]
}
```

## WebSocket API

### Connection

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onopen = function() {
    console.log('Connected to IPPAN WebSocket');
};

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    console.log('Received:', data);
};

ws.onclose = function() {
    console.log('Disconnected from IPPAN WebSocket');
};
```

### Subscriptions

#### Subscribe to Network Events

```javascript
ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'network',
    events: ['peer_connected', 'peer_disconnected']
}));
```

#### Subscribe to Storage Events

```javascript
ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'storage',
    events: ['file_uploaded', 'file_downloaded']
}));
```

#### Subscribe to Consensus Events

```javascript
ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'consensus',
    events: ['block_created', 'transaction_confirmed']
}));
```

### Event Types

#### Network Events

```json
{
  "type": "network_event",
  "event": "peer_connected",
  "data": {
    "node_id": "peer123...",
    "address": "192.168.1.100:8080",
    "timestamp": "2024-01-01T12:00:00Z"
  }
}
```

#### Storage Events

```json
{
  "type": "storage_event",
  "event": "file_uploaded",
  "data": {
    "file_hash": "abc123...",
    "file_size": 1024,
    "uploader": "uploader123...",
    "timestamp": "2024-01-01T12:00:00Z"
  }
}
```

#### Consensus Events

```json
{
  "type": "consensus_event",
  "event": "block_created",
  "data": {
    "block_hash": "block123...",
    "round": 12345,
    "validator": "validator123...",
    "transactions_count": 150,
    "timestamp": "2024-01-01T12:00:00Z"
  }
}
```

## Error Handling

### Error Response Format

```json
{
  "success": false,
  "error": {
    "code": "INVALID_INPUT",
    "message": "Invalid input provided",
    "details": {
      "field": "amount",
      "reason": "Amount must be positive"
    }
  },
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `INVALID_INPUT` | Invalid request parameters |
| `UNAUTHORIZED` | Authentication required |
| `FORBIDDEN` | Insufficient permissions |
| `NOT_FOUND` | Resource not found |
| `RATE_LIMITED` | Too many requests |
| `INTERNAL_ERROR` | Server error |
| `NETWORK_ERROR` | Network communication error |
| `STORAGE_ERROR` | Storage operation failed |
| `WALLET_ERROR` | Wallet operation failed |
| `CONSENSUS_ERROR` | Consensus operation failed |

## Rate Limiting

### Rate Limit Headers

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640995200
```

### Rate Limits

| Endpoint | Limit | Window |
|----------|-------|--------|
| General | 1000 | 1 hour |
| File Upload | 100 | 1 hour |
| Payment | 50 | 1 hour |
| Domain Registration | 10 | 1 hour |

## Client Libraries

### JavaScript/TypeScript

```bash
npm install ippan-client
```

```javascript
import { IppanClient } from 'ippan-client';

const client = new IppanClient({
    baseUrl: 'http://localhost:3000/api/v1',
    apiKey: 'your-api-key'
});

// Upload file
const result = await client.storage.uploadFile('/path/to/file.txt');

// Send payment
const tx = await client.wallet.sendPayment('recipient', 1000000000);

// Get network stats
const stats = await client.network.getStats();
```

### Python

```bash
pip install ippan-python
```

```python
from ippan import IppanClient

client = IppanClient(
    base_url='http://localhost:3000/api/v1',
    api_key='your-api-key'
)

# Upload file
result = client.storage.upload_file('/path/to/file.txt')

# Send payment
tx = client.wallet.send_payment('recipient', 1000000000)

# Get network stats
stats = client.network.get_stats()
```

### Rust

```toml
# Cargo.toml
[dependencies]
ippan-client = "1.0.0"
```

```rust
use ippan_client::IppanClient;

let client = IppanClient::new(
    "http://localhost:3000/api/v1",
    "your-api-key"
);

// Upload file
let result = client.storage.upload_file("/path/to/file.txt").await?;

// Send payment
let tx = client.wallet.send_payment("recipient", 1000000000).await?;

// Get network stats
let stats = client.network.get_stats().await?;
```

## Examples

### Complete File Upload Example

```javascript
const fs = require('fs');
const FormData = require('form-data');

async function uploadFile(filePath, fileName) {
    const form = new FormData();
    form.append('file', fs.createReadStream(filePath));
    form.append('name', fileName);
    form.append('encrypt', 'true');

    const response = await fetch('http://localhost:3000/api/v1/storage/upload', {
        method: 'POST',
        body: form,
        headers: {
            'Authorization': 'Bearer YOUR_API_KEY'
        }
    });

    const result = await response.json();
    return result.file_hash;
}

// Usage
const fileHash = await uploadFile('/path/to/document.txt', 'my-document');
console.log('File uploaded:', fileHash);
```

### M2M Payment Channel Example

```javascript
async function createPaymentChannel(recipient, amount, duration) {
    const response = await fetch('http://localhost:3000/api/v1/wallet/channels', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': 'Bearer YOUR_API_KEY'
        },
        body: JSON.stringify({
            recipient,
            amount,
            duration_hours: duration,
            description: 'IoT device payment channel'
        })
    });

    return await response.json();
}

async function sendMicroPayment(channelId, amount, metadata) {
    const response = await fetch(`http://localhost:3000/api/v1/wallet/channels/${channelId}/pay`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': 'Bearer YOUR_API_KEY'
        },
        body: JSON.stringify({
            amount,
            type: 'data_transfer',
            metadata
        })
    });

    return await response.json();
}

// Usage
const channel = await createPaymentChannel('recipient_address', 10000000000, 24);
console.log('Channel created:', channel.channel_id);

const payment = await sendMicroPayment(channel.channel_id, 1000000, {
    bytes_transferred: 1024,
    service_type: 'sensor_data'
});
console.log('Payment sent:', payment.payment_id);
```

### Real-time Monitoring Example

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onopen = function() {
    // Subscribe to all events
    ws.send(JSON.stringify({
        type: 'subscribe',
        channel: 'network',
        events: ['peer_connected', 'peer_disconnected']
    }));

    ws.send(JSON.stringify({
        type: 'subscribe',
        channel: 'storage',
        events: ['file_uploaded', 'file_downloaded']
    }));

    ws.send(JSON.stringify({
        type: 'subscribe',
        channel: 'consensus',
        events: ['block_created', 'transaction_confirmed']
    }));
};

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    
    switch(data.type) {
        case 'network_event':
            console.log('Network event:', data.event, data.data);
            break;
        case 'storage_event':
            console.log('Storage event:', data.event, data.data);
            break;
        case 'consensus_event':
            console.log('Consensus event:', data.event, data.data);
            break;
    }
};
```

---

For more information, visit:
- [IPPAN Documentation](https://docs.ippan.net)
- [IPPAN GitHub](https://github.com/ippan/ippan)
- [IPPAN Discord](https://discord.gg/ippan) 