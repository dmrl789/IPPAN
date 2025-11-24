# Developer Journey

This guide walks you from cloning IPPAN → running a local network → building against the SDKs → observing transactions.

## 1. Clone & bootstrap

```bash
git clone https://github.com/dmrl789/IPPAN.git
cd IPPAN
rustup toolchain install stable
cargo --version
```

Optional tooling: Node.js 18+ (for the TypeScript SDK) and Docker (if you want to wrap services).

## 2. Launch the full stack

Use the helper script described in the [Local Full-Stack Guide](./local-full-stack.md):

```bash
scripts/run-local-full-stack.sh
```

You now have three validator nodes listening on `http://127.0.0.1:{8080,8081,8082}` plus matching P2P ports. Check health:

```bash
curl http://127.0.0.1:8080/health | jq
```

Stop everything when finished:

```bash
scripts/localnet_stop.sh
```

## 3. Create a wallet & handle

```bash
ippan-wallet generate-key --output ~/.ippan/dev.key
ippan-wallet show-address --key ~/.ippan/dev.key
ippan-wallet handle register \
  --key ~/.ippan/dev.key \
  --handle "@demo.ipn"
```

See the [Handles & Addresses guide](../users/handles-and-addresses.md) for more context.

## 4. Interact via SDKs

- **Rust**: add `ippan-sdk` to your crate and follow the samples in [SDK Overview](./sdk-overview.md). Use `client.get_account` to query balances and `client.submit_payment` to send funds between your dev wallets.
- **TypeScript**: run `npm install` inside `apps/sdk-ts`, then `npm run example:sdk_ts_payment` (see `apps/sdk-ts/examples/sdk_ts_payment.ts`) to send a payment through the localnet.

## 5. Inspect blocks & explorer

- Blocks: `curl http://127.0.0.1:8080/block/1 | jq`
- Transactions: `curl http://127.0.0.1:8080/tx/<hash>`
- Metrics: `curl http://127.0.0.1:8080/metrics | head`

## 6. Iterate

- Update Rust/TS clients, rerun the SDK examples, and watch the node logs under `localnet/node*.log`.
- Extend the localnet configs in `localnet/node*.toml` to test alternate consensus parameters.
- When ready to showcase UI flows, host your frontend locally and point it at the loopback RPC endpoints.

## Troubleshooting

- Port already in use → stop the previous localnet (`scripts/localnet_stop.sh`) or change the ports in `localnet/node*.toml`.
- Wallet issues → regenerate a dev key (`ippan-wallet generate-key`) and ensure the file is readable by your user only (`chmod 600`).
- SDK errors → enable verbose tracing (`RUST_LOG=debug` for Rust, `DEBUG=ippan:*` for TypeScript) and inspect the HTTP requests hitting `/tx/payment`.

Once the local workflow feels solid, follow the [Production Validator Runbook](../operators/production-validator-runbook.md) to deploy to shared environments.
