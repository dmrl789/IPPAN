# IPPAN API Reference

This document provides comprehensive documentation for the IPPAN HTTP API endpoints.

## Base URL

All API endpoints are available at:
```
http://localhost:8080
```

## Authentication

Currently, the API does not require authentication. In production, this will be implemented using JWT tokens or API keys.

## Response Format

All API responses follow a consistent JSON format:

```json
{
  "success": true,
  "data": {...},
  "error": null,
  "message": "Operation completed successfully"
}
```

## Error Responses

When an error occurs, the response will have:

```json
{
  "success": false,
  "data": null,
  "error": "Error description",
  "message": "Operation failed"
}
```

## Health & Status Endpoints

### GET /health

Health check endpoint to verify the API is running.

**Response:**
```json
{
  "success": true,
  "data": "OK",
  "error": null,
  "message": "Health check passed"
}
```

### GET /status

Get comprehensive node status information.

**Response:**
```json
{
  "success": true,
  "data": {
    "version": "0.1.0",
    "uptime_seconds": 3600,
    "consensus_round": 1234,
    "storage_usage": {
      "used_bytes": 1073741824,
      "total_bytes": 10737418240,
      "shard_count": 150
    },
    "network_peers": 25,
    "wallet_balance": 1000000,
    "dht_keys": 500
  },
  "error": null,
  "message": "Node status retrieved"
}
```

### GET /version

Get the current IPPAN version.

**Response:**
```json
{
  "success": true,
  "data": "0.1.0",
  "error": null,
  "message": "Version retrieved"
}
```

## Storage Endpoints

### POST /storage/files

Store a file in the distributed storage system.

**Request Body:**
```json
{
  "file_id": "unique_file_identifier",
  "name": "document.pdf",
  "data": "base64_encoded_file_data",
  "mime_type": "application/pdf",
  "replication_factor": 3,
  "encryption_enabled": true
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "file_id": "unique_file_identifier",
    "size": 1048576,
    "shard_count": 2,
    "message": "File stored successfully"
  },
  "error": null,
  "message": "File stored successfully"
}
```

### GET /storage/files/:file_id

Retrieve a file from the distributed storage system.

**Response:**
```json
{
  "success": true,
  "data": {
    "file_id": "unique_file_identifier",
    "name": "document.pdf",
    "data": "base64_encoded_file_data",
    "mime_type": "application/pdf",
    "size": 1048576,
    "created_at": "2024-01-15T10:30:00Z"
  },
  "error": null,
  "message": "File retrieved successfully"
}
```

### DELETE /storage/files/:file_id

Delete a file from the distributed storage system.

**Response:**
```json
{
  "success": true,
  "data": "unique_file_identifier",
  "error": null,
  "message": "File deleted successfully"
}
```

### GET /storage/stats

Get comprehensive storage statistics.

**Response:**
```json
{
  "success": true,
  "data": {
    "total_files": 150,
    "total_shards": 450,
    "total_nodes": 25,
    "online_nodes": 23,
    "used_bytes": 1073741824,
    "total_bytes": 10737418240,
    "replication_factor": 3
  },
  "error": null,
  "message": "Storage stats retrieved"
}
```

## Network Endpoints

### GET /network/peers

Get the list of connected peers.

**Response:**
```json
{
  "success": true,
  "data": [
    "peer_id_1",
    "peer_id_2",
    "peer_id_3"
  ],
  "error": null,
  "message": "Peer list retrieved"
}
```

### DELETE /network/peers/:peer_id

Disconnect from a specific peer.

**Response:**
```json
{
  "success": true,
  "data": "peer_id",
  "error": null,
  "message": "Peer disconnected"
}
```

## Consensus Endpoints

### GET /consensus/round

Get the current consensus round information.

**Response:**
```json
{
  "success": true,
  "data": 1234,
  "error": null,
  "message": "Consensus round retrieved"
}
```

### GET /consensus/validators

Get the list of active validators.

**Response:**
```json
{
  "success": true,
  "data": [
    "validator_id_1",
    "validator_id_2",
    "validator_id_3"
  ],
  "error": null,
  "message": "Validator list retrieved"
}
```

## Wallet Endpoints

### GET /wallet/balance

Get the current wallet balance.

**Response:**
```json
{
  "success": true,
  "data": 1000000,
  "error": null,
  "message": "Wallet balance retrieved"
}
```

### GET /wallet/address

Get the wallet address.

**Response:**
```json
{
  "success": true,
  "data": "i1exampleaddress123456789",
  "error": null,
  "message": "Wallet address retrieved"
}
```

### POST /wallet/send

Send a transaction.

**Request Body:**
```json
{
  "to_address": "i1recipientaddress123456789",
  "amount": 100000,
  "fee": 1000,
  "memo": "Payment for services"
}
```

**Response:**
```json
{
  "success": true,
  "data": "tx_hash_example",
  "error": null,
  "message": "Transaction sent successfully"
}
```

## DHT (Distributed Hash Table) Endpoints

### GET /dht/keys

Get all DHT keys.

**Response:**
```json
{
  "success": true,
  "data": [
    "key_1",
    "key_2",
    "key_3"
  ],
  "error": null,
  "message": "DHT keys retrieved"
}
```

### GET /dht/keys/:key

Get a specific DHT value.

**Response:**
```json
{
  "success": true,
  "data": "dht_value_example",
  "error": null,
  "message": "DHT value retrieved"
}
```

### POST /dht/keys/:key

Store a value in the DHT.

**Request Body:**
```json
{
  "value": "data_to_store",
  "ttl": 3600
}
```

**Response:**
```json
{
  "success": true,
  "data": "key",
  "error": null,
  "message": "DHT value stored successfully"
}
```

## Error Codes

The API uses standard HTTP status codes:

- `200 OK` - Request successful
- `400 Bad Request` - Invalid request parameters
- `404 Not Found` - Resource not found
- `500 Internal Server Error` - Server error

## Rate Limiting

Currently, the API does not implement rate limiting. In production, this will be implemented to prevent abuse.

## Examples

### Store a File

```bash
curl -X POST http://localhost:8080/storage/files \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "my_document",
    "name": "document.txt",
    "data": "SGVsbG8sIFdvcmxkIQ==",
    "mime_type": "text/plain",
    "replication_factor": 3,
    "encryption_enabled": true
  }'
```

### Get Node Status

```bash
curl http://localhost:8080/status
```

### Send a Transaction

```bash
curl -X POST http://localhost:8080/wallet/send \
  -H "Content-Type: application/json" \
  -d '{
    "to_address": "i1recipientaddress123456789",
    "amount": 100000,
    "fee": 1000,
    "memo": "Payment"
  }'
```

## SDK Examples

### JavaScript/Node.js

```javascript
const axios = require('axios');

const API_BASE = 'http://localhost:8080';

// Store a file
async function storeFile(fileId, name, data, mimeType) {
  const response = await axios.post(`${API_BASE}/storage/files`, {
    file_id: fileId,
    name: name,
    data: Buffer.from(data).toString('base64'),
    mime_type: mimeType,
    replication_factor: 3,
    encryption_enabled: true
  });
  return response.data;
}

// Get node status
async function getNodeStatus() {
  const response = await axios.get(`${API_BASE}/status`);
  return response.data;
}
```

### Python

```python
import requests
import base64

API_BASE = 'http://localhost:8080'

def store_file(file_id, name, data, mime_type):
    response = requests.post(f'{API_BASE}/storage/files', json={
        'file_id': file_id,
        'name': name,
        'data': base64.b64encode(data).decode('utf-8'),
        'mime_type': mime_type,
        'replication_factor': 3,
        'encryption_enabled': True
    })
    return response.json()

def get_node_status():
    response = requests.get(f'{API_BASE}/status')
    return response.json()
```

## Development

To start the API server in development mode:

```bash
cargo run --bin ippan
```

The API will be available at `http://localhost:8080`.

## Testing

Run the test suite:

```bash
cargo test --lib
```

## Contributing

When adding new endpoints:

1. Add the endpoint handler in `src/api/http.rs`
2. Add comprehensive tests
3. Update this documentation
4. Follow the existing response format patterns

## Version History

- **v0.1.0** - Initial API implementation with storage, network, consensus, wallet, and DHT endpoints 