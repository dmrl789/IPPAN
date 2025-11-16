# IPPAN Payment API Guide

This guide summarizes the Layer 1 payment HTTP APIs implemented in `crates/rpc/src/server.rs`. All endpoints live under the public RPC service exposed by the node binary and are consumed by the `ippan pay` CLI as well as automation scripts.

## Currency Model

* Every balance, amount, and fee is represented as a 128-bit unsigned integer (`u128`) standing for the smallest atomic currency unit (yocto-IPN). There are no floating-point amounts anywhere in request or response payloads.
* JSON payloads accept atomic units either as JSON numbers (when they fit into JavaScript's safe range) or as decimal strings. Responses always return strings such as `"1000000000000000000"` to avoid precision loss.
* Fees default to the protocol minimum derived from `ippan_l1_fees::FeePolicy`. Clients can override the fee by providing a custom integer limit.

## Endpoint Summary

| Method | Path | Purpose |
| ------ | ---- | ------- |
| `POST` | `/tx/payment` | Submit a signed payment transaction to the mempool. |
| `GET` | `/account/:address/payments` | Fetch finalized payments involving a specific account, newest first. |

### `POST /tx/payment`

Submit a payment transfer signed by the sender. The RPC server derives the next nonce (unless provided) and enqueues the transaction into consensus.

**Request Body**

```json
{
  "from": "0x4e52...",          // hex-encoded 32-byte account address
  "to": "0x8ab4...",            // destination address (hex or base58)
  "amount": "500000000000000000",// u128 atomic units (string or number)
  "fee": "1000000000000",       // optional u128 limit; defaults to policy minimum
  "nonce": 42,                   // optional u64; auto-derived when omitted
  "memo": "invoice #42",        // optional memo (<=256 UTF-8 bytes)
  "signing_key": "0x1234..."    // optional 32-byte hex private key used to sign
}
```

Field notes:

* `from` / `to` accept `0x` prefixes; `to` also accepts base58 handles resolved by `decode_address`.
* `amount` must be greater than zero. Fees that undershoot `FeePolicy` are rejected with `fee_too_low`.
* `signing_key` allows the node to sign on behalf of the caller. Operators that pre-sign transactions can omit it; in that case the RPC endpoint must be extended to accept explicit signatures (not yet implemented).

**Response**

On success the server returns HTTP `200 OK` with:

```json
{
  "tx_hash": "0xbaf2...",
  "status": "accepted_to_mempool",    // or "finalized" when queried later
  "from": "0x4e52...",
  "to": "0x8ab4...",
  "nonce": 42,
  "amount_atomic": "500000000000000000",
  "fee_atomic": "1000000000000",
  "timestamp": 1728000000000,
  "memo": "invoice #42"
}
```

Errors surface JSON payloads with `code` + `message` and include validation failures such as `invalid_address`, `missing_signing_key`, or `consensus_unavailable`.

### `GET /account/:address/payments`

Returns the most recent finalized payments (incoming, outgoing, and self transfers) for a single address.

**Path Parameter**

* `:address` — base58 or `0x`-prefixed 32-byte account identifier.

**Query Parameters**

| Name | Description | Default/Max |
| ---- | ----------- | ----------- |
| `limit` | Clamp the number of records returned. Server defaults to `25` and caps the value at `200`. | `25` / `200` |
| `before` | Reserved cursor (e.g., previous `tx_hash`) for future pagination. Passing this parameter is harmless today—the current implementation simply ignores it and always returns the newest `limit` entries. | *(optional)* |

**Response**

The endpoint currently returns a JSON array of payment records sorted by `timestamp` descending. Each element matches `PaymentView` from the RPC crate:

```json
[
  {
    "hash": "0xbaf2...",
    "from": "0x4e52...",
    "to": "0x8ab4...",
    "direction": "outgoing",         // "incoming" | "outgoing" | "self_transfer"
    "amount_atomic": "500000000000000000",
    "fee_atomic": "1000000000000",
    "total_cost_atomic": "500001000000000000", // omitted when irrelevant
    "nonce": 42,
    "timestamp": 1728000000000,
    "memo": "invoice #42",
    "status": "finalized"
  }
]
```

* `direction` is computed from the caller's perspective.
* `status` is `finalized` for history queries; new mempool entries can be tracked via `/account/:address` if needed.
* The `timestamp` reflects the transaction's hash timer microsecond timestamp. A dedicated `hash_timer` blob is not yet exposed but can be pulled from block data if required.

### Demo & Helper Scripts

* 📄 [docs/payments/demo_end_to_end_payment.md](payments/demo_end_to_end_payment.md) walks through creating funding accounts, running the CLI, and verifying state transitions.
* 🛠️ [`scripts/demo_payment_flow.sh`](../scripts/demo_payment_flow.sh) automates the same steps (fund dev account, send payment, fetch history) against a local node. Customize environment variables inside the script to point at your RPC endpoint.

## Client Examples

Below are lightweight examples showing how to hit the payment endpoints. Replace host/port with your node's RPC listener (default `http://127.0.0.1:8080`).

### JavaScript / TypeScript

```ts
const baseUrl = "http://127.0.0.1:8080";

async function sendPayment() {
  const res = await fetch(`${baseUrl}/tx/payment`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      from: "0x4e52...",
      to: "0x8ab4...",
      amount: "500000000000000000",
      signing_key: process.env.SENDER_KEY,
    }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

async function listPayments(address) {
  const res = await fetch(`${baseUrl}/account/${address}/payments?limit=10`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}
```

### Python (`requests`)

```python
import os
import requests

BASE_URL = "http://127.0.0.1:8080"

payment = {
    "from": "0x4e52...",
    "to": "0x8ab4...",
    "amount": "750000000000000000",
    "memo": "python demo",
    "signing_key": os.environ["SENDER_KEY"],
}

resp = requests.post(f"{BASE_URL}/tx/payment", json=payment, timeout=10)
resp.raise_for_status()
print(resp.json())

history = requests.get(
    f"{BASE_URL}/account/{payment['from']}/payments",
    params={"limit": 5},
    timeout=10,
)
history.raise_for_status()
print(history.json())
```

### Rust (`reqwest` + `serde_json`)

```rust
use reqwest::blocking::Client;
use serde_json::json;

fn main() -> anyhow::Result<()> {
    let client = Client::new();
    let base = "http://127.0.0.1:8080";

    let body = json!({
        "from": "0x4e52...",
        "to": "0x8ab4...",
        "amount": "1000000000000000000",
        "signing_key": std::env::var("SENDER_KEY")?,
    });

    let res = client.post(format!("{}/tx/payment", base)).json(&body).send()?;
    println!("payment response: {}", res.text()?);

    let history = client
        .get(format!("{}/account/{}/payments", base, "0x4e52..."))
        .query(&[("limit", "10")])
        .send()?;
    println!("history: {}", history.text()?);
    Ok(())
}
```

## Related References

* RPC implementation: [`crates/rpc/src/server.rs`](../crates/rpc/src/server.rs)
* Consensus payment pipeline: [`crates/consensus/src/payments.rs`](../crates/consensus/src/payments.rs)
* CLI helper: [`crates/cli/src/main.rs` (`PayCommand`)](../crates/cli/src/main.rs)

With these endpoints, operators can script deterministic payment flows and rely on integer-only accounting end-to-end.
