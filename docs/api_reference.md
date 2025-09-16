# IPPAN Blockchain API Reference

## Table of Contents
1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Node API](#node-api)
4. [Network API](#network-api)
5. [Consensus API](#consensus-api)
6. [Transaction API](#transaction-api)
7. [Block API](#block-api)
8. [Wallet API](#wallet-api)
9. [Storage API](#storage-api)
10. [Monitoring API](#monitoring-api)
11. [Admin API](#admin-api)
12. [Error Handling](#error-handling)
13. [Rate Limiting](#rate-limiting)
14. [WebSocket API](#websocket-api)

## Overview

The IPPAN Blockchain provides a comprehensive REST API for interacting with the network. This API allows you to query blockchain data, submit transactions, manage wallets, and monitor node operations.

### Base URL
```
http://localhost:8080/api/v1
```

### Content Types
- **Request**: `application/json`
- **Response**: `application/json`

### HTTP Status Codes
- `200` - Success
- `201` - Created
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `429` - Too Many Requests
- `500` - Internal Server Error

## Authentication

### API Key Authentication
```bash
curl -H "X-API-Key: your-api-key" http://localhost:8080/api/v1/status
```

### JWT Authentication
```bash
# Get JWT token
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "user", "password": "password"}'

# Use JWT token
curl -H "Authorization: Bearer your-jwt-token" http://localhost:8080/api/v1/status
```

### Session Authentication
```bash
# Login
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "user", "password": "password"}' \
  -c cookies.txt

# Use session
curl -b cookies.txt http://localhost:8080/api/v1/status
```

## Node API

### Get Node Status
```http
GET /node/status
```

**Response:**
```json
{
  "node_id": "12D3KooW...",
  "version": "1.0.0",
  "network_id": "ippan-mainnet",
  "chain_id": "ippan-1",
  "uptime": 86400,
  "status": "running",
  "peer_count": 15,
  "block_height": 12345,
  "sync_status": "synced"
}
```

### Get Node Information
```http
GET /node/info
```

**Response:**
```json
{
  "node_id": "12D3KooW...",
  "public_key": "0x1234...",
  "listen_address": "0.0.0.0:30333",
  "api_address": "0.0.0.0:8080",
  "p2p_address": "0.0.0.0:30333",
  "is_bootstrap_node": false,
  "is_validator": true,
  "validator_address": "0x5678...",
  "stake_amount": 1000000
}
```

### Get Node Health
```http
GET /node/health
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": 1640995200,
  "components": {
    "consensus": "healthy",
    "network": "healthy",
    "storage": "healthy",
    "database": "healthy"
  },
  "metrics": {
    "cpu_usage": 45.2,
    "memory_usage": 67.8,
    "disk_usage": 23.4,
    "network_throughput": 125.6
  }
}
```

### Get Node Metrics
```http
GET /node/metrics
```

**Response:**
```json
{
  "timestamp": 1640995200,
  "system": {
    "cpu_usage_percent": 45.2,
    "memory_usage_mb": 2048,
    "disk_usage_percent": 23.4,
    "network_throughput_mbps": 125.6
  },
  "application": {
    "transactions_processed": 12345,
    "blocks_created": 567,
    "peers_connected": 15,
    "sync_time_ms": 250
  }
}
```

## Network API

### Get Network Status
```http
GET /network/status
```

**Response:**
```json
{
  "network_id": "ippan-mainnet",
  "chain_id": "ippan-1",
  "total_peers": 150,
  "connected_peers": 15,
  "bootstrap_nodes": 5,
  "validators": 25,
  "network_throughput": 125.6,
  "latency_avg_ms": 45.2
}
```

### Get Connected Peers
```http
GET /network/peers
```

**Response:**
```json
{
  "peers": [
    {
      "peer_id": "12D3KooW...",
      "address": "peer1.ippan.net:30333",
      "connected_at": 1640995200,
      "last_seen": 1640995200,
      "is_bootstrap": true,
      "is_validator": false,
      "latency_ms": 45.2,
      "bytes_sent": 1024000,
      "bytes_received": 2048000
    }
  ],
  "total_count": 15
}
```

### Add Peer
```http
POST /network/peers
```

**Request:**
```json
{
  "peer_id": "12D3KooW...",
  "address": "new-peer.ippan.net:30333"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Peer added successfully",
  "peer_id": "12D3KooW..."
}
```

### Remove Peer
```http
DELETE /network/peers/{peer_id}
```

**Response:**
```json
{
  "success": true,
  "message": "Peer removed successfully"
}
```

### Get Network Metrics
```http
GET /network/metrics
```

**Response:**
```json
{
  "timestamp": 1640995200,
  "connections": {
    "total": 15,
    "active": 12,
    "failed": 3
  },
  "throughput": {
    "bytes_sent": 1024000,
    "bytes_received": 2048000,
    "messages_sent": 500,
    "messages_received": 750
  },
  "latency": {
    "min_ms": 10.5,
    "max_ms": 150.2,
    "avg_ms": 45.2,
    "p95_ms": 85.6
  }
}
```

## Consensus API

### Get Consensus Status
```http
GET /consensus/status
```

**Response:**
```json
{
  "consensus_type": "bft",
  "status": "active",
  "current_round": 12345,
  "current_phase": "propose",
  "block_time": 5,
  "finality_threshold": 0.67,
  "validators": 25,
  "active_validators": 23
}
```

### Get Validators
```http
GET /consensus/validators
```

**Response:**
```json
{
  "validators": [
    {
      "validator_id": "12D3KooW...",
      "address": "0x1234...",
      "stake_amount": 1000000,
      "commission_rate": 0.05,
      "status": "active",
      "voting_power": 1000000,
      "last_activity": 1640995200
    }
  ],
  "total_count": 25,
  "active_count": 23
}
```

### Get Latest Block
```http
GET /consensus/blocks/latest
```

**Response:**
```json
{
  "block": {
    "hash": "0x1234...",
    "height": 12345,
    "timestamp": 1640995200,
    "proposer": "12D3KooW...",
    "transaction_count": 100,
    "size_bytes": 1048576,
    "parent_hash": "0x5678...",
    "state_root": "0x9abc...",
    "transactions_root": "0xdef0..."
  }
}
```

### Get Block by Height
```http
GET /consensus/blocks/{height}
```

**Response:**
```json
{
  "block": {
    "hash": "0x1234...",
    "height": 12345,
    "timestamp": 1640995200,
    "proposer": "12D3KooW...",
    "transaction_count": 100,
    "size_bytes": 1048576,
    "parent_hash": "0x5678...",
    "state_root": "0x9abc...",
    "transactions_root": "0xdef0...",
    "transactions": [
      {
        "hash": "0xabcd...",
        "from": "0x1234...",
        "to": "0x5678...",
        "amount": 1000,
        "fee": 10,
        "timestamp": 1640995200
      }
    ]
  }
}
```

### Get Block Statistics
```http
GET /consensus/blocks/stats
```

**Response:**
```json
{
  "total_blocks": 12345,
  "blocks_per_second": 0.2,
  "average_block_time": 5.0,
  "average_block_size": 1048576,
  "average_transactions_per_block": 100,
  "finality_time_avg": 15.0
}
```

## Transaction API

### Submit Transaction
```http
POST /transactions
```

**Request:**
```json
{
  "from": "0x1234...",
  "to": "0x5678...",
  "amount": 1000,
  "fee": 10,
  "nonce": 123,
  "signature": "0xabcd...",
  "memo": "Payment for services"
}
```

**Response:**
```json
{
  "success": true,
  "transaction_hash": "0xabcd...",
  "message": "Transaction submitted successfully"
}
```

### Get Transaction
```http
GET /transactions/{hash}
```

**Response:**
```json
{
  "transaction": {
    "hash": "0xabcd...",
    "from": "0x1234...",
    "to": "0x5678...",
    "amount": 1000,
    "fee": 10,
    "nonce": 123,
    "signature": "0xabcd...",
    "memo": "Payment for services",
    "status": "confirmed",
    "block_height": 12345,
    "timestamp": 1640995200
  }
}
```

### Get Transaction History
```http
GET /transactions/history/{address}
```

**Query Parameters:**
- `limit` (optional): Number of transactions to return (default: 100)
- `offset` (optional): Number of transactions to skip (default: 0)
- `status` (optional): Filter by transaction status

**Response:**
```json
{
  "transactions": [
    {
      "hash": "0xabcd...",
      "from": "0x1234...",
      "to": "0x5678...",
      "amount": 1000,
      "fee": 10,
      "status": "confirmed",
      "block_height": 12345,
      "timestamp": 1640995200
    }
  ],
  "total_count": 500,
  "has_more": true
}
```

### Get Pending Transactions
```http
GET /transactions/pending
```

**Response:**
```json
{
  "transactions": [
    {
      "hash": "0xabcd...",
      "from": "0x1234...",
      "to": "0x5678...",
      "amount": 1000,
      "fee": 10,
      "submitted_at": 1640995200
    }
  ],
  "total_count": 25
}
```

### Get Transaction Pool Status
```http
GET /transactions/pool/status
```

**Response:**
```json
{
  "pool_size": 25,
  "max_size": 10000,
  "memory_usage_mb": 50,
  "max_memory_mb": 1024,
  "average_fee": 15.5,
  "oldest_transaction": 1640995200
}
```

## Block API

### Get Block by Hash
```http
GET /blocks/{hash}
```

**Response:**
```json
{
  "block": {
    "hash": "0x1234...",
    "height": 12345,
    "timestamp": 1640995200,
    "proposer": "12D3KooW...",
    "transaction_count": 100,
    "size_bytes": 1048576,
    "parent_hash": "0x5678...",
    "state_root": "0x9abc...",
    "transactions_root": "0xdef0...",
    "transactions": [
      {
        "hash": "0xabcd...",
        "from": "0x1234...",
        "to": "0x5678...",
        "amount": 1000,
        "fee": 10,
        "timestamp": 1640995200
      }
    ]
  }
}
```

### Get Block Range
```http
GET /blocks
```

**Query Parameters:**
- `from_height` (optional): Starting block height
- `to_height` (optional): Ending block height
- `limit` (optional): Number of blocks to return (default: 100)

**Response:**
```json
{
  "blocks": [
    {
      "hash": "0x1234...",
      "height": 12345,
      "timestamp": 1640995200,
      "proposer": "12D3KooW...",
      "transaction_count": 100,
      "size_bytes": 1048576
    }
  ],
  "total_count": 1000,
  "has_more": true
}
```

### Get Block Statistics
```http
GET /blocks/stats
```

**Response:**
```json
{
  "total_blocks": 12345,
  "blocks_per_second": 0.2,
  "average_block_time": 5.0,
  "average_block_size": 1048576,
  "average_transactions_per_block": 100,
  "finality_time_avg": 15.0
}
```

## Wallet API

### Create Account
```http
POST /wallet/accounts
```

**Request:**
```json
{
  "account_type": "standard",
  "password": "secure_password"
}
```

**Response:**
```json
{
  "success": true,
  "account": {
    "address": "0x1234...",
    "account_type": "standard",
    "created_at": 1640995200
  }
}
```

### Get Account Balance
```http
GET /wallet/accounts/{address}/balance
```

**Response:**
```json
{
  "address": "0x1234...",
  "balance": 1000000,
  "locked_balance": 50000,
  "available_balance": 950000,
  "last_updated": 1640995200
}
```

### Get Account Information
```http
GET /wallet/accounts/{address}
```

**Response:**
```json
{
  "address": "0x1234...",
  "account_type": "standard",
  "balance": 1000000,
  "locked_balance": 50000,
  "available_balance": 950000,
  "transaction_count": 150,
  "created_at": 1640995200,
  "last_activity": 1640995200
}
```

### Send Transaction
```http
POST /wallet/transactions/send
```

**Request:**
```json
{
  "from": "0x1234...",
  "to": "0x5678...",
  "amount": 1000,
  "fee": 10,
  "password": "secure_password",
  "memo": "Payment for services"
}
```

**Response:**
```json
{
  "success": true,
  "transaction_hash": "0xabcd...",
  "message": "Transaction sent successfully"
}
```

### Get Transaction History
```http
GET /wallet/accounts/{address}/transactions
```

**Query Parameters:**
- `limit` (optional): Number of transactions to return (default: 100)
- `offset` (optional): Number of transactions to skip (default: 0)
- `status` (optional): Filter by transaction status

**Response:**
```json
{
  "transactions": [
    {
      "hash": "0xabcd...",
      "from": "0x1234...",
      "to": "0x5678...",
      "amount": 1000,
      "fee": 10,
      "status": "confirmed",
      "block_height": 12345,
      "timestamp": 1640995200
    }
  ],
  "total_count": 150,
  "has_more": true
}
```

### Backup Wallet
```http
POST /wallet/backup
```

**Request:**
```json
{
  "password": "secure_password"
}
```

**Response:**
```json
{
  "success": true,
  "backup_data": "encrypted_backup_data",
  "message": "Wallet backed up successfully"
}
```

### Restore Wallet
```http
POST /wallet/restore
```

**Request:**
```json
{
  "backup_data": "encrypted_backup_data",
  "password": "secure_password"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Wallet restored successfully"
}
```

## Storage API

### Get Storage Status
```http
GET /storage/status
```

**Response:**
```json
{
  "status": "healthy",
  "total_capacity": 1000000000000,
  "used_capacity": 250000000000,
  "available_capacity": 750000000000,
  "shard_count": 10,
  "replication_factor": 3,
  "compression_ratio": 0.75
}
```

### Get Storage Metrics
```http
GET /storage/metrics
```

**Response:**
```json
{
  "timestamp": 1640995200,
  "capacity": {
    "total_bytes": 1000000000000,
    "used_bytes": 250000000000,
    "available_bytes": 750000000000
  },
  "performance": {
    "read_ops_per_sec": 1000,
    "write_ops_per_sec": 500,
    "average_read_latency_ms": 5.2,
    "average_write_latency_ms": 8.7
  },
  "shards": {
    "total": 10,
    "healthy": 9,
    "degraded": 1,
    "failed": 0
  }
}
```

### Get Storage Health
```http
GET /storage/health
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": 1640995200,
  "shards": [
    {
      "shard_id": "shard-1",
      "status": "healthy",
      "capacity_used_percent": 25.0,
      "replication_factor": 3,
      "last_backup": 1640995200
    }
  ]
}
```

## Monitoring API

### Get Metrics
```http
GET /metrics
```

**Response:**
```json
{
  "timestamp": 1640995200,
  "system": {
    "cpu_usage_percent": 45.2,
    "memory_usage_mb": 2048,
    "disk_usage_percent": 23.4,
    "network_throughput_mbps": 125.6
  },
  "application": {
    "transactions_processed": 12345,
    "blocks_created": 567,
    "peers_connected": 15,
    "sync_time_ms": 250
  },
  "consensus": {
    "block_time_avg": 5.0,
    "finality_time_avg": 15.0,
    "validator_count": 25,
    "active_validators": 23
  },
  "network": {
    "peers_connected": 15,
    "messages_sent": 500,
    "messages_received": 750,
    "latency_avg_ms": 45.2
  }
}
```

### Get Health Status
```http
GET /health
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": 1640995200,
  "components": {
    "consensus": "healthy",
    "network": "healthy",
    "storage": "healthy",
    "database": "healthy"
  },
  "metrics": {
    "cpu_usage": 45.2,
    "memory_usage": 67.8,
    "disk_usage": 23.4,
    "network_throughput": 125.6
  }
}
```

### Get Alerts
```http
GET /alerts
```

**Response:**
```json
{
  "alerts": [
    {
      "id": "alert-1",
      "name": "High CPU Usage",
      "severity": "warning",
      "status": "active",
      "message": "CPU usage is above 80%",
      "created_at": 1640995200,
      "updated_at": 1640995200
    }
  ],
  "total_count": 1
}
```

### Create Alert
```http
POST /alerts
```

**Request:**
```json
{
  "name": "High Memory Usage",
  "condition": "memory_usage > 85",
  "severity": "warning",
  "notification": "email:admin@ippan.net"
}
```

**Response:**
```json
{
  "success": true,
  "alert_id": "alert-2",
  "message": "Alert created successfully"
}
```

## Admin API

### Get System Information
```http
GET /admin/system
```

**Response:**
```json
{
  "system": {
    "os": "Linux",
    "kernel": "5.4.0-74-generic",
    "architecture": "x86_64",
    "cpu_count": 8,
    "memory_total": 16777216,
    "disk_total": 1000000000000
  },
  "application": {
    "version": "1.0.0",
    "build_date": "2024-01-01T00:00:00Z",
    "git_commit": "abc123...",
    "rust_version": "1.70.0"
  }
}
```

### Get Configuration
```http
GET /admin/config
```

**Response:**
```json
{
  "network": {
    "network_id": "ippan-mainnet",
    "chain_id": "ippan-1",
    "listen_address": "0.0.0.0:30333",
    "api_address": "0.0.0.0:8080"
  },
  "consensus": {
    "consensus_type": "bft",
    "block_time": 5,
    "max_block_size": 1048576,
    "finality_threshold": 0.67
  },
  "security": {
    "enable_tls": true,
    "enable_encryption": true,
    "rate_limit": 1000
  }
}
```

### Update Configuration
```http
PUT /admin/config
```

**Request:**
```json
{
  "consensus": {
    "block_time": 10
  },
  "security": {
    "rate_limit": 2000
  }
}
```

**Response:**
```json
{
  "success": true,
  "message": "Configuration updated successfully"
}
```

### Get Logs
```http
GET /admin/logs
```

**Query Parameters:**
- `level` (optional): Log level filter
- `component` (optional): Component filter
- `limit` (optional): Number of log entries (default: 100)

**Response:**
```json
{
  "logs": [
    {
      "timestamp": 1640995200,
      "level": "info",
      "component": "consensus",
      "message": "Block created successfully",
      "details": {}
    }
  ],
  "total_count": 1000,
  "has_more": true
}
```

### Restart Node
```http
POST /admin/restart
```

**Response:**
```json
{
  "success": true,
  "message": "Node restart initiated"
}
```

### Shutdown Node
```http
POST /admin/shutdown
```

**Response:**
```json
{
  "success": true,
  "message": "Node shutdown initiated"
}
```

## Error Handling

### Error Response Format
```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Invalid request parameters",
    "details": {
      "field": "amount",
      "reason": "Must be a positive integer"
    },
    "timestamp": 1640995200,
    "request_id": "req-123"
  }
}
```

### Common Error Codes
- `INVALID_REQUEST` - Invalid request parameters
- `UNAUTHORIZED` - Authentication required
- `FORBIDDEN` - Insufficient permissions
- `NOT_FOUND` - Resource not found
- `RATE_LIMITED` - Rate limit exceeded
- `INTERNAL_ERROR` - Internal server error
- `NETWORK_ERROR` - Network connectivity issue
- `CONSENSUS_ERROR` - Consensus-related error
- `STORAGE_ERROR` - Storage-related error

## Rate Limiting

### Rate Limit Headers
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640995200
```

### Rate Limit Response
```json
{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded",
    "details": {
      "limit": 1000,
      "remaining": 0,
      "reset_time": 1640995200
    }
  }
}
```

## WebSocket API

### Connect to WebSocket
```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = function() {
    console.log('Connected to IPPAN WebSocket');
};

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    console.log('Received:', data);
};
```

### Subscribe to Events
```javascript
// Subscribe to new blocks
ws.send(JSON.stringify({
    type: 'subscribe',
    event: 'new_block'
}));

// Subscribe to new transactions
ws.send(JSON.stringify({
    type: 'subscribe',
    event: 'new_transaction'
}));

// Subscribe to node status
ws.send(JSON.stringify({
    type: 'subscribe',
    event: 'node_status'
}));
```

### WebSocket Events
```json
{
  "type": "event",
  "event": "new_block",
  "data": {
    "block": {
      "hash": "0x1234...",
      "height": 12345,
      "timestamp": 1640995200,
      "proposer": "12D3KooW...",
      "transaction_count": 100
    }
  }
}
```

### WebSocket Error
```json
{
  "type": "error",
  "error": {
    "code": "INVALID_SUBSCRIPTION",
    "message": "Invalid event type"
  }
}
```

## SDKs and Libraries

### JavaScript/TypeScript
```bash
npm install @ippan/sdk
```

```javascript
import { IppanClient } from '@ippan/sdk';

const client = new IppanClient('http://localhost:8080');

// Get node status
const status = await client.node.getStatus();

// Submit transaction
const tx = await client.transactions.submit({
  from: '0x1234...',
  to: '0x5678...',
  amount: 1000,
  fee: 10
});
```

### Python
```bash
pip install ippan-sdk
```

```python
from ippan import IppanClient

client = IppanClient('http://localhost:8080')

# Get node status
status = client.node.get_status()

# Submit transaction
tx = client.transactions.submit({
    'from': '0x1234...',
    'to': '0x5678...',
    'amount': 1000,
    'fee': 10
})
```

### Go
```bash
go get github.com/ippan/go-sdk
```

```go
package main

import (
    "github.com/ippan/go-sdk"
)

func main() {
    client := ippan.NewClient("http://localhost:8080")
    
    // Get node status
    status, err := client.Node.GetStatus()
    
    // Submit transaction
    tx, err := client.Transactions.Submit(ippan.TransactionRequest{
        From:   "0x1234...",
        To:     "0x5678...",
        Amount: 1000,
        Fee:    10,
    })
}
```

## Support

For additional support and documentation:

- **Documentation**: https://docs.ippan.net
- **GitHub**: https://github.com/your-org/ippan
- **Discord**: https://discord.gg/ippan
- **Email**: support@ippan.net

## License

This API reference is part of the IPPAN project and is licensed under the MIT License.