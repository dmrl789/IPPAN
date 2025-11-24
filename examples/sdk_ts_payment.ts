/**
 * Minimal example that reuses the in-repo TypeScript SDK without publishing.
 *
 * Run with:
 *   export IPPAN_SIGNING_KEY=<hex>; export IPPAN_FROM_ADDRESS=<hex>; export IPPAN_TO_ADDRESS=@friend.ipn
 *   npx ts-node examples/sdk_ts_payment.ts
 *
 * Ensure you have built the sdk package at least once:
 *   npm install --prefix apps/sdk-ts
 *   npm run --prefix apps/sdk-ts build
 */
import { IppanClient } from "../apps/sdk-ts/dist/index.js";

async function main() {
  const api = process.env.IPPAN_API_URL ?? "http://127.0.0.1:8081/api/";
  const from = process.env.IPPAN_FROM_ADDRESS;
  const signingKey = process.env.IPPAN_SIGNING_KEY;
  const to = process.env.IPPAN_TO_ADDRESS ?? "@friend.ipn";

  if (!from || !signingKey) {
    throw new Error("Set IPPAN_FROM_ADDRESS and IPPAN_SIGNING_KEY in your environment.");
  }

  const client = new IppanClient({ baseUrl: api });
  const account = await client.getAccount(from);
  console.log(`Balance for ${from}: ${account.balance_atomic} (nonce ${account.nonce})`);

  const receipt = await client.sendPayment({
    from,
    to,
    amount: "1000000000000000000",
    signingKey,
    memo: "sdk ts example",
    nonce: account.nonce + 1
  });

  console.log(`Submitted tx ${receipt.tx_hash} -> status ${receipt.status}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
