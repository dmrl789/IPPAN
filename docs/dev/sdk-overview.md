## Developer SDKs & Typed Clients

Phase 5 introduces first-class SDKs for Rust (crate `ippan-sdk`) and TypeScript (`apps/sdk-ts`). Both wrappers provide consistent error handling and typed responses for the REST/RPC surface that operators and app developers touch most frequently.

### Canonical RPC Surface

| Endpoint | Method | Description |
| --- | --- | --- |
| `/health` | GET | Liveness check (plain text) |
| `/time` | GET | HashTimer + network time data |
| `/account/{address}` | GET | Account snapshot plus recent payments/transactions |
| `/tx/{hash}` | GET | Transaction summary + status |
| `/block/{id}` | GET | Block info by hash or height |
| `/tx/payment` | POST | Submit signed payments (addresses or `@handle`) |
| `/handle/{@alias}` | GET | Resolve handles to account data |

> Tip: When developing against the gateway (`apps/gateway`), the same routes are exposed under `/api/*`. Point the SDK base URL at `http://localhost:8081/api/` when running the full-stack compose stack.

### Rust crate `ippan-sdk`

Location: `crates/sdk`

Key features:

- `IppanClient` with async helpers `get_account`, `get_block`, `get_transaction`, `get_time`, `submit_payment`.
- Strongly typed responses that translate RPC payloads into `Amount`-aware structs.
- Rich `SdkError` enumerations (invalid URL, HTTP transport, server `code/message`, parse errors).
- Example program: `cargo run -p ippan-sdk --example sdk_rust_payment`.

Quick glance:

```rust
use ippan_sdk::{IppanClient, PaymentRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = IppanClient::new("http://localhost:8081/api/")?;
    let account = client.get_account("b4f3...").await?;

    let receipt = client
        .submit_payment(
            PaymentRequest::new(account.address.clone(), "@friend.ipn", 100_000, "<key>")
                .with_nonce(account.nonce + 1)
                .with_memo(Some("sdk demo".into())),
        )
        .await?;

    println!("Tx hash {}", receipt.tx_hash);
    Ok(())
}
```

### TypeScript SDK (`@ippan/sdk`)

Location: `apps/sdk-ts`

- Zero-runtime-dependency wrapper built around `fetch`.
- Works in Node 18+ (native fetch) or browsers; pass `fetchImpl` for custom environments.
- Typed request/response models that mirror the JSON payloads (snake_case).
- Example script lives in `examples/sdk_ts_payment.ts`.

Usage:

```ts
import { IppanClient } from "@ippan/sdk";

const client = new IppanClient({ baseUrl: "http://localhost:8081/api/" });
const account = await client.getAccount(process.env.IPPAN_FROM_ADDRESS!);

await client.sendPayment({
  from: account.address,
  to: "@friend.ipn",
  amount: "500000000000000000",
  signingKey: process.env.IPPAN_SIGNING_KEY!,
  nonce: account.nonce + 1
});
```

Build it once with:

```bash
npm install --prefix apps/sdk-ts
npm run --prefix apps/sdk-ts build
```

### Example Journeys

1. **Query + Submit with Rust**
   - Ensure the local stack is running (`scripts/run-local-full-stack.sh`).
   - `export IPPAN_FROM_ADDRESS=<hex>` etc.
   - `cargo run -p ippan-sdk --example sdk_rust_payment`.
2. **Query + Submit with TypeScript**
   - Build the TS SDK as above.
   - `export IPPAN_SIGNING_KEY=...`
   - `npx ts-node examples/sdk_ts_payment.ts`.

### Error Surface

Both SDKs expose the same semantics:

- HTTP layer/transport errors bubble up as `SdkError::Http` (`IppanSdkError` in TS).
- API errors returned by the server include `{code, message}`; both SDKs parse these and attach HTTP status.
- Parse/conversion failures (e.g., malformed atomic strings) return `SdkError::Parse`.

### Next Steps

- Expand coverage to `/handles` mutation endpoints once the governance rules land.
- Layer in typed helpers for staking/governance RPCs after those APIs stabilize.
- Upstream both SDKs to crates.io/npm once we lock API compatibility for v1.0.
