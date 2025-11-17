# Explorer & Operator RPC Surface

This catalog lists the read-only RPC routes that explorers, dashboards, and
mobile clients can safely expose. All payloads return integers for amounts,
counts, and timestamps (microseconds) and never mutate node state. Paths are
relative to the RPC base URL (default `http://127.0.0.1:8080`).

## Public explorer endpoints

### `GET /block/:id`

* **Path parameter:** `:id` can be a hex block hash (`0x…` optional) or an
  integer height.
* **Response:**
  * `block` – metadata describing the block.
    * `id` (hex), `round`, optional `height`, `creator` (hex validator ID).
    * `hash_timer` (HashTimer hex), `timestamp` (µs), `parent_ids` array.
    * `transaction_hashes` list and `transactions` (full objects, see below).
    * `tx_count` – number of transactions inlined.
  * `fee_summary` – optional round-fee snapshot with `round`,
    `total_fees_atomic`, `treasury_fees_atomic`, `applied_payments`,
    `rejected_payments`.
* **Notes:** `/block/:id` already includes full transaction objects, so a
  dedicated `/block/:id/txs` route is not required.
* **Example:**

```json
{
  "block": {
    "id": "4e8c…",
    "round": 128,
    "height": 128,
    "creator": "06bb…",
    "hash_timer": "2ef0d1…",
    "timestamp": 1731000123456,
    "parent_ids": ["4a10…", "3ff2…"],
    "transaction_hashes": ["9a7c…"],
    "tx_count": 1,
    "transactions": [
      {
        "hash": "9a7c…",
        "from": "ippan1…",
        "to": "ippan1…",
        "amount_atomic": "125000000",
        "fee_atomic": "25000",
        "nonce": 42,
        "timestamp": 1731000123000,
        "hash_timer": "14cc…",
        "status": "finalized",
        "visibility": "public",
        "memo": "invoice-42"
      }
    ]
  },
  "fee_summary": {
    "round": 128,
    "total_fees_atomic": "50000",
    "treasury_fees_atomic": "20000",
    "applied_payments": 2,
    "rejected_payments": 1
  }
}
```

### `GET /tx/:hash`

* **Path parameter:** 32-byte transaction hash (hex, `0x` optional).
* **Response:** Single transaction object identical to the entries returned
  inside `/block/:id`.
* **Example:**

```json
{
  "hash": "9a7c…",
  "from": "ippan1…",
  "to": "ippan1…",
  "amount_atomic": "125000000",
  "fee_atomic": "25000",
  "nonce": 42,
  "timestamp": 1731000123000,
  "hash_timer": "14cc…",
  "status": "finalized",
  "visibility": "public",
  "memo": "invoice-42",
  "handle_operation": null
}
```

### `GET /account/:address`

* **Path parameter:** account address (base58check or raw hex).
* **Response fields:**
  * `address` – canonical hex encoding.
  * `balance_atomic` – string integer (atomic IPN units).
  * `nonce` – last confirmed nonce.
  * `recent_transactions` – array of transaction objects (same shape as above).
  * `recent_payments` – payments derived from finalized transactions with
    direction hints, fees, timestamps, hash timers, and memos.
* **Example:**

```json
{
  "address": "8f22…",
  "balance_atomic": "1000000000000",
  "nonce": 57,
  "recent_transactions": [ { "hash": "…" } ],
  "recent_payments": [
    {
      "hash": "9a7c…",
      "from": "ippan1…",
      "to": "ippan1…",
      "direction": "outgoing",
      "amount_atomic": "125000000",
      "fee_atomic": "25000",
      "total_cost_atomic": "125025000",
      "nonce": 42,
      "timestamp": 1731000123000,
      "hash_timer": "14cc…",
      "memo": "invoice-42",
      "status": "finalized"
    }
  ]
}
```

### `GET /account/:address/payments`

* **Query parameters:** `limit` (optional, default 25, max 200).
* **Response:** Same `PaymentView` objects as `recent_payments`, sorted by
  timestamp descending.

### `GET /handle/{handle}`

* **Path parameter:** canonical handle (accepts with/without `@` prefix).
* **Response:**
  * `handle`, `owner` (address), `status` (`active`, `expired`, `suspended`,
    or `transferred`).
  * `expires_at` (optional UNIX seconds), `metadata` map, `created_at`,
    `updated_at` (µs).

```json
{
  "handle": "@alice.ipn",
  "owner": "ippan1…",
  "status": "active",
  "expires_at": 1767225600,
  "metadata": {"avatar": "ipfs://…"},
  "created_at": 1731000000000,
  "updated_at": 1731000000000
}
```

### `GET /files/:id`

* **Path parameter:** file descriptor ID (HashTimer hex) or content hash.
* **Response:** `id`, `content_hash`, `owner`, `size_bytes`, `created_at_us`,
  optional `mime_type`, `tags`.

```json
{
  "id": "fd_01f3…",
  "content_hash": "b9f1…",
  "owner": "ippan1…",
  "size_bytes": 5242880,
  "created_at_us": 1730999900000,
  "mime_type": "application/pdf",
  "tags": ["kyc", "prospectus"]
}
```

### `GET /ai/status`

* **Response fields:**
  * `enabled` – whether the deterministic AI/DLC pipeline is active.
  * `using_stub` – true when a stub/placeholder model is serving.
  * `model_hash`, `model_version` – optional strings identifying the active
    model artifact.
  * `consensus_mode` – node consensus label (`"poa"`, `"dlc"`, etc.).
* **Example:**

```json
{
  "enabled": true,
  "using_stub": false,
  "model_hash": "b8f91b…",
  "model_version": "dlc-2025-11-01",
  "consensus_mode": "dlc"
}
```

## Operator observability endpoints

Explorers and dashboards may also poll these read-only routes:

* `GET /health` – holistic node snapshot (see `docs/OBSERVABILITY_GUIDE.md`).
* `GET /metrics` – Prometheus text exposition (enabled via config flag).

## Dev-only and mutating routes — do **NOT** expose

These endpoints change node state or are only meant for localhost testing. They
are guarded by `SecurityManager`, `IPPAN_DEV_MODE`, or loopback checks and
should remain behind firewalls/reverse proxies:

* `POST /tx`, `POST /tx/payment`
* `POST /handle/register`
* `POST /files/publish`
* `POST /dev/fund`
* All `/p2p/*` gossip relays

## Example: minimal operator dashboard

### Node overview card

Combine `/health` and `/ai/status`:

```json
{
  "health": {
    "status": "ok",
    "node_id": "node-01",
    "consensus_mode": "dlc",
    "consensus_healthy": true,
    "peer_count": 6,
    "mempool_size": 4,
    "uptime_seconds": 3600,
    "requests_served": 420,
    "last_finalized_round": 128
  },
  "ai": {
    "enabled": true,
    "using_stub": false,
    "model_hash": "b8f91b…",
    "model_version": "dlc-2025-11-01",
    "consensus_mode": "dlc"
  }
}
```

### Account overview card

Poll `/account/:address` and `/account/:address/payments`:

```json
{
  "account": {
    "address": "8f22…",
    "balance_atomic": "1000000000000",
    "nonce": 57
  },
  "recent_payments": [
    {"hash": "9a7c…", "direction": "outgoing", "amount_atomic": "125000000"}
  ]
}
```

### Recent block view

Fetch `/block/:id` (latest height) and use transaction hashes to hydrate rows via
`/tx/:hash` when necessary:

```json
{
  "block": {
    "id": "4e8c…",
    "round": 128,
    "tx_count": 12,
    "timestamp": 1731000123456
  },
  "transactions": [
    {"hash": "9a7c…", "from": "ippan1…", "amount_atomic": "125000000"}
  ]
}
```

All examples above are representative; the exact keys/structures come directly
from the RPC implementations documented here.
