## Transaction Feed API

### POST /api/txfeed

**Description:**
Receive and store transactions from archive nodes.

**Request Body:**
```json
{
  "tx_hash": "abc123...",
  "timestamp": "2025-07-21T10:01:03.000Z",
  "tx_type": "file_announce",
  "payload": { ... },
  "proof": {
    "round": "R123...",
    "zk_stark": "..."
  },
  "signature": "..."
}
```

**Response:**
- `200 OK` if the transaction is successfully stored.
- `400 Bad Request` if the request is malformed.
- `500 Internal Server Error` if there is a server error.

**Validation:**
- Ensure hash matches payload.
- Verify signature.
- Validate zk-STARK or round hash.
