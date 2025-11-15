# IPPAN L1 Payment Demo

This guide walks through a reproducible end-to-end Layer 1 payment on a single developer node:

1. launch a dev-mode node,
2. generate sender/receiver key pairs,
3. fund the sender via the `/dev/fund` helper endpoint,
4. submit a fee-bearing payment with `ippan-cli pay`, and
5. inspect outgoing/incoming history with `/account/{address}/payments`.

It pairs with the automation script at `scripts/demo_payment_flow.sh`.

---

## Prerequisites

- Linux/macOS shell with `bash`, `curl`, and (optionally) `jq`
- Rust toolchain (see `docs/DEVELOPER_GUIDE.md`)
- Workspace built binaries (`cargo run` downloads dependencies)
- Node started with dev-mode enabled so `/dev/fund` is exposed:

```bash
export IPPAN_DEV_MODE=true   # or pass --dev flag
RUST_LOG=info cargo run --bin ippan-node -- --dev
```

Keep the node running in its own terminal for the remainder of the demo.

---

## Step 1 – Generate Accounts

Use `ippan-keygen` to create two Ed25519 keypairs and record their public/legacy addresses:

```bash
cargo run -p keygen -- generate --output demo-keys --name sender
cargo run -p keygen -- generate --output demo-keys --name receiver

# Derive the legacy (hex) and Base58Check forms
SENDER_HEX=$(tr -d '\n' < demo-keys/sender_public.key | tr '[:upper:]' '[:lower:]')
RECEIVER_HEX=$(tr -d '\n' < demo-keys/receiver_public.key | tr '[:upper:]' '[:lower:]')
SENDER_ADDR="i${SENDER_HEX}"
RECEIVER_ADDR="i${RECEIVER_HEX}"
```

Use the `i<hex>` form when talking to `/tx/payment` and `/dev/fund`. The raw 64-character hex (without the `i` prefix) is required for `/account/{address}` and `/account/{address}/payments`.

---

## Step 2 – Fund the Sender (`/dev/fund`)

The dev funding endpoint credits balances directly in storage. It only accepts loopback requests while `IPPAN_DEV_MODE=true` (or when the node is launched with `--dev`).

```bash
curl -sS -H "Content-Type: application/json" \
  -d "{\"address\":\"${SENDER_ADDR}\",\"amount\":1000000000,\"nonce\":0}" \
  http://127.0.0.1:8080/dev/fund | jq
```

Sample response:

```json
{
  "address_hex": "8f72...c5",
  "address_base58": "ippan1k2...k8",
  "balance": 1000000000,
  "nonce": 0,
  "created": true
}
```

`amount` is specified in atomic (yocto-IPN) units and must fit within `u64`.

---

## Step 3 – Submit a Payment

Send a payment that spends part of the funded balance plus the deterministic fee:

```bash
cargo run -p cli -- \
  --rpc-url http://127.0.0.1:8080 \
  pay \
  --from "${SENDER_ADDR}" \
  --to "${RECEIVER_ADDR}" \
  --amount 250000000 \
  --fee 2000 \
  --key-file demo-keys/sender_private.key \
  --memo "demo payment"
```

Successful output:

```
Payment accepted: 7ab38c0b4f8e4dc1c5be1d94c4f8660cc066296dbdf4faab16b922bda3f5b621
```

The RPC returns the structured payload below (visible with `RUST_LOG=debug` or by curling `/tx/payment` directly):

```json
{
  "tx_hash": "7ab3…b621",
  "status": "accepted_to_mempool",
  "from": "i8f72…c5",
  "to": "idf09…77",
  "nonce": 1,
  "amount_atomic": "250000000",
  "fee_atomic": "2000",
  "timestamp": 1734375100123456,
  "memo": "demo payment"
}
```

---

## Step 4 – Confirm Inclusion & History

1. Confirm the transaction is persisted:

   ```bash
   TX=7ab38c0b4f8e4dc1c5be1d94c4f8660cc066296dbdf4faab16b922bda3f5b621
   curl -s http://127.0.0.1:8080/tx/${TX} | jq '.hash'
   ```

2. (Optional) Inspect node status / consensus round:

   ```bash
   curl -s http://127.0.0.1:8080/status | jq '.consensus'
   ```

3. Query outgoing history for the sender (note the raw hex address in the path):

   ```bash
   curl -s "http://127.0.0.1:8080/account/${SENDER_HEX}/payments?limit=1" | jq
   ```

   Example:

   ```json
   [
     {
       "hash": "7ab3…b621",
       "from": "i8f72…c5",
       "to": "idf09…77",
       "direction": "outgoing",
       "amount_atomic": "250000000",
       "fee_atomic": "2000",
       "total_cost_atomic": "250002000",
       "nonce": 1,
       "timestamp": 1734375100123456,
       "memo": "demo payment",
       "status": "finalized"
     }
   ]
   ```

4. Query incoming history for the receiver:

   ```bash
   curl -s "http://127.0.0.1:8080/account/${RECEIVER_HEX}/payments?limit=1" | jq
   ```

   Receiver entries show `"direction": "incoming"` and omit `total_cost_atomic`.

---

## Automation Script (`scripts/demo_payment_flow.sh`)

For a scripted version of the same flow:

```bash
./scripts/demo_payment_flow.sh run-all
```

The script:

- generates/reuses demo key pairs under `KEY_DIR` (default `./demo-keys`),
- funds the sender via `/dev/fund`,
- submits a payment with `ippan-cli pay`, and
- fetches sender/receiver histories.

Individual sub-commands are available (`keys`, `fund`, `pay`, `history`), and the behavior can be tuned via environment variables (`RPC_URL`, `FUND_AMOUNT`, `PAY_AMOUNT`, `FEE_LIMIT`, `DEMO_MEMO`, `SENDER_NONCE`).

---

## Troubleshooting

- `curl /dev/fund` returns 403 — ensure the node was launched with `IPPAN_DEV_MODE=true` or `--dev`, and issue the request from `127.0.0.1` or `::1`.
- `ippan-cli pay` complains about balances — confirm the funding step shows the expected `balance` and re-run it if necessary.
- Need a clean slate — stop the node, remove `./data`, regenerate keys, and repeat the steps above.

For more context on binaries and configuration see `README.md`, `docs/DEVELOPER_GUIDE.md`, and `docs/BINARIES.md`.
