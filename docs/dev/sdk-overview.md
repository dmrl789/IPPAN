# SDK Overview

IPPAN ships two first-class SDKs that expose the same typed RPC surface:

- **Rust** (`crates/sdk`) – async `IppanClient` for querying chain state and submitting transactions.
- **TypeScript** (`apps/sdk-ts`) – browser/Node-friendly `fetch` wrapper with typed DTOs.

## Canonical RPC Surface

| Endpoint | Method | Description |
| --- | --- | --- |
| `/health` | GET | Liveness check (plain text) |
| `/time` | GET | HashTimer + network time data |
| `/account/{address}` | GET | Account snapshot plus recent payments/transactions |
| `/tx/{hash}` | GET | Transaction summary + status |
| `/block/{id}` | GET | Block info by hash or height |
| `/tx/payment` | POST | Submit signed payments (addresses or `@handle`) |
| `/handle/{@alias}` | GET | Resolve handles to account data |

> Tip: when working through the gateway (`apps/gateway`), the same routes appear under `/api/*`. Use `http://localhost:8081/api/` whenever the Docker full-stack is running.

## Rust crate `ippan-sdk`

Add to your binary crate:

```toml
[dependencies]
ippan-sdk = { path = "crates/sdk" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Example (see `crates/sdk/examples/sdk_rust_payment.rs` for the complete flow):

```rust
use ippan_sdk::{IppanClient, PaymentRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = IppanClient::new("http://localhost:8081/api/")?;
    let account = client.get_account("i1z...").await?;

    let receipt = client
        .submit_payment(
            PaymentRequest::new(account.address.clone(), "@friend.ipn", 100_000, "<hex-signing-key>")
                .with_nonce(account.nonce + 1)
                .with_memo(Some("sdk demo".into())),
        )
        .await?;

    println!("Tx hash {}", receipt.tx_hash);
    Ok(())
}
```

Highlights:

- Async helpers for `get_account`, `get_block`, `get_transaction`, `get_time`, `submit_payment`, `resolve_handle`, and more.
- Strongly typed responses convert JSON payloads into IPPAN domain structs.
- Rich `SdkError` enum differentiates transport vs. RPC vs. parse failures.
- `IppanClient::with_http_client` lets you inject a mocked `reqwest::Client` (great for `wiremock`).

## TypeScript SDK (`@ippan/sdk`)

Location: `apps/sdk-ts`

- Zero-runtime-dependency wrapper built around `fetch` (works in Node 18+ or browsers; supply `fetchImpl` for custom runtimes).
- Typed request/response models derived from the RPC schema.
- Example script: `apps/sdk-ts/examples/sdk_ts_payment.ts`.

Install/build:

```bash
npm install --prefix apps/sdk-ts
npm run --prefix apps/sdk-ts build
```

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
  nonce: account.nonce + 1,
  memo: "sdk demo"
});
```

## Example Journeys

1. **Rust** – Ensure the [local full-stack](./local-full-stack.md) is running, export wallet env vars, and run `cargo run -p ippan-sdk --example sdk_rust_payment`.
2. **TypeScript** – Build the TS SDK, set `IPPAN_SIGNING_KEY`, and execute `npx ts-node apps/sdk-ts/examples/sdk_ts_payment.ts`.

## Testing & Mocking

- **Rust** – Inject your own HTTP client via `IppanClient::with_http_client` (e.g., a `reqwest::Client` backed by `wiremock`).
- **TypeScript** – Provide a custom `fetchImpl` (e.g., `node-fetch`, `whatwg-fetch`) or substitute a Jest mock for deterministic tests.

## Error Surface

Both SDKs align on error semantics:

- Transport failures bubble up as `SdkError::Http` (`IppanSdkError` in TS) with HTTP status codes.
- Server-side `{code, message}` replies become typed RPC errors.
- Parse/conversion issues return `SdkError::Parse`.

## Next Steps

- Follow the [Developer Journey](./developer-journey.md) for a complete onboarding path.
- Expand coverage to handles/governance endpoints as those APIs stabilize.
- Publish to crates.io/npm once API compatibility is locked for v1.0.
