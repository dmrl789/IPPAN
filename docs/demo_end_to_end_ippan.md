# End-to-End IPPAN Dev Demo

This guide pairs the CLI, RPC, AI, and DHT integrations into a single flow so
operators can see handles, payments, file descriptors, and deterministic AI
status working together on one developer node. It is mirrored by the
`scripts/demo_ippan_full_flow.sh` automation script.

## 1. Overview

**What you will do:**

1. Launch a dev-mode node that exposes stub IPNDHT backends and `/dev` helpers.
2. Generate two demo accounts.
3. Register an `@handle.ipn` that resolves to the sender account.
4. Send a payment from the sender → receiver and read history back.
5. Publish a file descriptor and retrieve it via the RPC.
6. Query `/ai/status` plus the active DHT modes to prove deterministic AI + DHT
   plumbing is wired up.

**Requirements:**

- Rust toolchain + `cargo`
- `bash`, `curl`, and optionally `jq`
- Access to a single local node (`http://127.0.0.1:8080` by default)
- Dev mode enabled so `/dev/fund` and stub DHTs are exposed

## 2. Start a dev node

Export the dev-friendly environment variables and launch the node in a separate
terminal:

```bash
export IPPAN_DEV_MODE=true
export IPPAN_FILE_DHT_MODE=${IPPAN_FILE_DHT_MODE:-stub}
export IPPAN_HANDLE_DHT_MODE=${IPPAN_HANDLE_DHT_MODE:-stub}
RUST_LOG=info cargo run --bin ippan-node -- --dev
```

Notes:

- `--dev` / `IPPAN_DEV_MODE=true` unlock `/dev/fund` and looser RPC guards.
- Stub DHTs keep everything deterministic; switching to `libp2p` is a drop-in
  change if you already run peers.
- The RPC binds to `127.0.0.1:8080` unless `RPC_PORT` is overridden.

## 3. Register a handle

1. Generate two key pairs and derive addresses:

   ```bash
   cargo run -p keygen -- generate --output demo-keys --name sender
   cargo run -p keygen -- generate --output demo-keys --name receiver

   SENDER_HEX=$(tr -d '\n' < demo-keys/sender_public.key | tr '[:upper:]' '[:lower:]')
   RECEIVER_HEX=$(tr -d '\n' < demo-keys/receiver_public.key | tr '[:upper:]' '[:lower:]')
   SENDER_ADDR="i${SENDER_HEX}"
   RECEIVER_ADDR="i${RECEIVER_HEX}"
   SENDER_KEY=$(tr -d '\n' < demo-keys/sender_private.key | tr '[:upper:]' '[:lower:]')
   ```

2. Fund the sender so payments/handles have balance + nonces:

   ```bash
   curl -sS -H "Content-Type: application/json" \
     -d "{\"address\":\"${SENDER_ADDR}\",\"amount\":1000000000,\"nonce\":0}" \
     http://127.0.0.1:8080/dev/fund | jq
   ```

3. Register `@demo.ipn` → sender via `POST /handle/register`:

   ```bash
   curl -sS -X POST http://127.0.0.1:8080/handle/register \
     -H "Content-Type: application/json" \
     -d @- <<'JSON'
   {
     "handle": "@demo.ipn",
     "owner": "${SENDER_ADDR}",
     "metadata": {"purpose": "e2e demo"},
     "expires_at": null,
     "fee": "2000",
     "nonce": null,
     "signing_key": "${SENDER_KEY}"
   }
   JSON
   ```

4. Verify the registration (URL-encode `@`):

   ```bash
   curl -s http://127.0.0.1:8080/handle/%40demo.ipn | jq
   ```

## 4. Send a payment

Re-use the funded sender and submit a deterministic payment (all amounts are
atomic units):

```bash
curl -sS -X POST http://127.0.0.1:8080/tx/payment \
  -H "Content-Type: application/json" \
  -d @- <<JSON
{
  "from": "${SENDER_ADDR}",
  "to": "${RECEIVER_ADDR}",
  "amount": "250000000",
  "fee": "2000",
  "memo": "demo payment",
  "signing_key": "${SENDER_KEY}"
}
JSON
```

Confirm history for both accounts (raw hex addresses in the path):

```bash
curl -s "http://127.0.0.1:8080/account/${SENDER_HEX}/payments?limit=5" | jq
curl -s "http://127.0.0.1:8080/account/${RECEIVER_HEX}/payments?limit=5" | jq
```

See `docs/PAYMENT_API_GUIDE.md` if you prefer `ippan-cli pay` or need more
payload details.

## 5. Publish and query a file descriptor

1. Prepare a demo file and content hash:

   ```bash
   echo "IPPAN demo artifact" > demo-file.txt
   CONTENT_HASH=$(sha256sum demo-file.txt | cut -d ' ' -f1)
   FILE_SIZE=$(stat -c%s demo-file.txt)   # macOS: stat -f%z demo-file.txt
   ```

2. Publish via `POST /files/publish`:

   ```bash
   curl -sS -X POST http://127.0.0.1:8080/files/publish \
     -H "Content-Type: application/json" \
     -d @- <<JSON
   {
     "owner": "${SENDER_ADDR}",
     "content_hash": "${CONTENT_HASH}",
     "size_bytes": ${FILE_SIZE},
     "mime_type": "text/plain",
     "tags": ["demo", "ippan"]
   }
   JSON
   ```

   The response includes an `id` (hex `FileId`).

3. Fetch the descriptor:

   ```bash
   curl -s http://127.0.0.1:8080/files/<FILE_ID_FROM_RESPONSE> | jq
   ```

`docs/ipndht/file-descriptors.md` documents every field plus the libp2p/stub
runtime split.

## 6. Check AI & DHT status

- Deterministic AI:

  ```bash
  curl -s http://127.0.0.1:8080/ai/status | jq
  ```

  Look for `enabled`, `using_stub`, `model_hash`, and `model_version` to confirm
  the deterministic D-GBDT pipeline is wired (see `docs/AI_STATUS_API.md`).

- DHT modes: the running node inherits `IPPAN_FILE_DHT_MODE` and
  `IPPAN_HANDLE_DHT_MODE`. Echo them in the shell you used to start the node:

  ```bash
  echo "File DHT mode: ${IPPAN_FILE_DHT_MODE:-stub}"
  echo "Handle DHT mode: ${IPPAN_HANDLE_DHT_MODE:-stub}"
  ```

  `stub` keeps everything in-process. Swap to `libp2p` for distributed testing
  without changing the RPC calls above.

## 7. Cleanup / next steps

- Stop the node with `Ctrl+C` in the process window.
- Remove demo artifacts (`rm -rf demo-keys demo-file.txt`) if desired.
- Explore related docs:
  - [Payment API Guide](PAYMENT_API_GUIDE.md)
  - [IPNDHT File Descriptor Notes](ipndht/file-descriptors.md)
- [L2 Handle System Architecture](L2_HANDLE_SYSTEM.md) or handle PRDs
  - [AI Status API](AI_STATUS_API.md)
- Automate the entire walkthrough via
  `./scripts/demo_ippan_full_flow.sh` once your node is already running.
