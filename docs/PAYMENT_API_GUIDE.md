# IPPAN Payment API Guide

The payment RPC is implemented in `crates/rpc/src/server.rs` and mirrors the Layer 1
currency model: every payment amount is an integer counted in atomic IPN units. This
document explains the denomination model, REST endpoints, CLI helpers, and reference
client snippets.

---

## Currency Model

| Denomination | Atomic Units (u128) | Notes |
|--------------|--------------------|-------|
| 1 IPN        | 10^24              | Native ledger unit; transactions store `Amount` as `u128`. |
| 1 milli-IPN  | 10^21              | 1/1,000 IPN; still integer math. |
| 1 micro-IPN  | 10^18              | 1/1,000,000 IPN; helpful for UI displays. |
| 1 nano-IPN   | 10^15              | 1/10^9 IPN. |
| 1 yocto-IPN  | 1                  | Atomic base unit (`Amount`/`fee` fields use this). |

Key rules:

- RPC payloads **never** accept floating-point numbers. Use strings or JSON integers that
  fit within `u128`.
- Fees share the same units; `ippan_l1_fees::FeePolicy` enforces minimums in atomic units.
- Wallets and CLI commands should perform any human-readable conversions before hitting
  the RPC.

---

## `POST /tx/payment`

Submit an L1 payment that transfers funds and pays the deterministic fee.

- **Method:** `POST`
- **Path:** `/tx/payment`
- **Content-Type:** `application/json`

### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `from` | string | ✅ | Sender account (hex-encoded 32-byte address or Base58Check with `i` prefix). |
| `to` | string | ✅ | Recipient account (same encoding rules as `from`). |
| `amount` | `u128` (JSON number or string) | ✅ | Atomic IPN amount to transfer; must be > 0. |
| `fee` | `u128` (JSON number or string) | optional | Max fee you are willing to pay. Handler enforces `FeePolicy`; omit to let the server estimate. |
| `nonce` | `u64` | optional | Explicit nonce. If omitted the handler fetches/derives the next nonce. |
| `memo` | string | optional | Up to 256 bytes; rejected if longer. |
| `signing_key` | string | ✅ | Hex-encoded 32-byte Ed25519 secret key. The RPC signs the transaction before broadcasting. |

> **Note:** There is no standalone `signature` field yet. Submitting a signature will be
> supported once external signing flows land; currently the RPC requires `signing_key` and
> performs signing server-side (`PaymentError::MissingSigningKey`).

### Success Response

```jsonc
{
  "tx_hash": "7ab38c0b4f8e4dc1c5be1d94c4f8660cc066296dbdf4faab16b922bda3f5b621",
  "status": "accepted_to_mempool",
  "from": "i8f72…c5",
  "to": "idf09…77",
  "nonce": 42,
  "amount_atomic": "250000000000000000000000", // string to avoid JS precision loss
  "fee_atomic": "2000",
  "timestamp": 1734375100123456,
  "memo": "demo payment"
}
```

### Error Responses

Errors use `{ "code": string, "message": string }` envelopes with appropriate HTTP
status codes. Example for an invalid fee limit:

```json
HTTP/1.1 400 Bad Request
{
  "code": "fee_too_low",
  "message": "required fee 5000 exceeds provided limit 2000"
}
```

### `curl` Example

```bash
curl -sS -X POST http://127.0.0.1:8080/tx/payment \
  -H "Content-Type: application/json" \
  -d '{
        "from": "i8f72...c5",
        "to": "idf09...77",
        "amount": "250000000000000000000000",
        "fee": "2000",
        "nonce": 42,
        "memo": "invoice #42",
        "signing_key": "'$SIGNING_KEY_HEX'"
      }'
```

---

## `GET /account/:address/payments`

Fetch finalized payment history for an address. The handler queries storage, sorts by
`timestamp`, and truncates based on the requested limit.

- **Method:** `GET`
- **Path:** `/account/:address/payments`
- **Path Parameter:** `address` is the 32-byte account in hex (with or without `0x`).

### Query Parameters

| Param | Type | Default | Max | Notes |
|-------|------|---------|-----|-------|
| `limit` | integer | 25 | 200 | Clamped between 1 and 200 by `clamp_history_limit()`. |
| `before` | string | _(reserved)_ | _(reserved)_ | The current server ignores this parameter; future pagination will treat it as a tx hash cursor. |

### Response Body

The RPC currently returns an array of `PaymentView` objects (no wrapper). Each entry looks
like:

```json
[
  {
    "hash": "7ab3…b621",
    "from": "i8f72…c5",
    "to": "idf09…77",
    "direction": "outgoing",
    "amount_atomic": "250000000",
    "fee_atomic": "2000",
    "total_cost_atomic": "250002000", // present for outgoing/self transfers
    "nonce": 1,
    "timestamp": 1734375100123456,
    "memo": "demo payment",
    "status": "finalized"
  }
]
```

> Pagination metadata (`address`, `count`, `before`) will be added once the RPC exposes a
> cursor-aware payload. For now the caller infers count via `array.length`.

### `curl` Usage

Fetch the first page (default limit 25):

```bash
ACCOUNT_HEX=8f72...c5
curl -s "http://127.0.0.1:8080/account/${ACCOUNT_HEX}/payments" | jq
```

Request a smaller page:

```bash
curl -s "http://127.0.0.1:8080/account/${ACCOUNT_HEX}/payments?limit=5" | jq
```

When `before` support lands, you will be able to provide `?before=<tx_hash>` to request
older entries.

---

## CLI + Demo Flow

- **CLI:** `ippan-cli pay --from <addr> --to <addr> --amount <atomic> --signing-key-hex <key>`
  - Optional flags: `--fee`, `--nonce`, `--memo`, `--key-file`.
  - Implementation lives in `crates/cli/src/main.rs` and posts to `/tx/payment`.
- **Demo Docs:** `docs/payments/demo_end_to_end_payment.md` walks through generating keys,
  funding via `/dev/fund`, sending a payment, and querying `/account/:address/payments`.
- **Demo Script:** `scripts/demo_payment_flow.sh` automates the same steps (`run-all`
orchestration plus `keys/fund/pay/history` subcommands).

Both the manual guide and script rely on `ippan-cli pay` under the hood; the CLI composes
payloads exactly like the `curl` example and prints the `tx_hash` returned by the RPC.

---

## Client Snippets

### JavaScript / TypeScript (fetch)

```ts
const res = await fetch("http://127.0.0.1:8080/tx/payment", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    from: sender,
    to: recipient,
    amount: amountAtomic.toString(),
    fee: "2000",
    signing_key: signingKeyHex,
  }),
});
const payment = await res.json();
console.log(payment.tx_hash);
```

### Python (requests)

```python
import requests

payload = {
    "from": sender,
    "to": recipient,
    "amount": str(amount_atomic),
    "signing_key": signing_key_hex,
}
resp = requests.post("http://127.0.0.1:8080/tx/payment", json=payload, timeout=10)
resp.raise_for_status()
print(resp.json()["tx_hash"])
```

### Rust (reqwest + serde)

```rust
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct PaymentResponse {
    tx_hash: String,
}

let client = Client::new();
let response: PaymentResponse = client
    .post("http://127.0.0.1:8080/tx/payment")
    .json(&serde_json::json!({
        "from": sender,
        "to": recipient,
        "amount": amount_atomic.to_string(),
        "signing_key": signing_key_hex,
    }))
    .send()?
    .json()?;
println!("tx hash: {}", response.tx_hash);
```

These snippets illustrate the required fields and integer-only payloads. Any additional
metadata (fee caps, memo, nonce) can be added to the JSON body as described earlier.

---

See also: [End-to-End IPPAN Dev Demo](demo_end_to_end_ippan.md)
