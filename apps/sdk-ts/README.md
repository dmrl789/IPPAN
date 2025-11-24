# IPPAN TypeScript SDK

Small, dependency-light wrapper around the IPPAN RPC/gateway REST API.

## Installation

```bash
npm install --prefix apps/sdk-ts
npm run --prefix apps/sdk-ts build
```

You can then import the client directly from the source (for monorepo consumers) or point your bundler to `apps/sdk-ts/dist`.

## Usage

```ts
import { IppanClient } from "@ippan/sdk";

const client = new IppanClient({ baseUrl: "http://localhost:8081/api/" });
const account = await client.getAccount("b4f3..."); // hex string
console.log(account.balance_atomic);

await client.sendPayment({
  from: "b4f3...",
  to: "@friend.ipn",
  amount: "1000000000000000000",
  signingKey: process.env.SIGNING_KEY!
});
```

See `examples/sdk_ts_payment.ts` for a runnable script using `ts-node`.
