# IPN Handles vs. Addresses

Human-readable handles (for example `@alice.ipn`) wrap the same 32-byte Ed25519
public keys that back standard IPPAN addresses. They make life easier for wallet
users, explorers, and operators without changing the underlying security model.

This guide explains how handles relate to raw addresses, how to register and
resolve them, and how to use handles in the wallet CLI or RPC calls.

---

## 1. Address Recap

- **Format:** Base58Check strings that start with `i…` plus a hex variant (64
  lowercase characters with an optional `0x` prefix).
- **Canonical form:** Every address ultimately resolves to a 32-byte Ed25519
  public key stored in the node’s state database.
- **Where they appear:** Node RPC (`/account/:address`, `/tx`, `/block`),
  gateway/explorer API responses, validator IDs, and the wallet CLI.

---

## 2. Handle Format & Validation

Validation is enforced by `crates/types/src/handle.rs` and
`crates/l2_handle_registry`:

| Rule | Description |
|------|-------------|
| Prefix | Must start with `@`. |
| Suffix | Must include a dot-separated suffix such as `.ipn`, `.iot`, `.m`, `.cyborg`. |
| Length | Between 4 and 63 characters inclusive. |
| Character set | ASCII alphanumerics plus `_` and `.` (trimmed before validation). |
| Premium TLDs | `.cyborg`, `.iot`, `.m` are flagged as premium via `Handle::is_premium`. |

Handles are stored on **Layer 2** (see [`L2_HANDLE_SYSTEM.md`](../L2_HANDLE_SYSTEM.md)),
while Layer 1 only keeps lightweight ownership anchors that prove who controls a
given handle.

---

## 3. Registering a Handle

Use the RPC’s `POST /handle/register` endpoint (available when a node runs with
`--dev` or via validator/gateway instances) to submit a `HandleOperation::Register`.

```bash
SIGNING_KEY=$(cat signer.hex) # 32-byte private key in hex
OWNER_ADDRESS=$(ippan-wallet show-address --key ./keys/dev.key --json | jq -r '.address')

curl -sS -X POST http://localhost:8080/handle/register \
  -H "Content-Type: application/json" \
  -d '{
        "handle": "@alice.ipn",
        "owner": "'$OWNER_ADDRESS'",
        "metadata": {"display_name": "Alice"},
        "nonce": null,
        "fee": null,
        "signing_key": "'$SIGNING_KEY'"
      }'
```

To inspect metadata later:

```bash
curl -sS http://localhost:8080/handle/%40alice.ipn | jq
```

---

## 4. Payment Flow with Handles

Phase 3 added automatic handle resolution to `/tx/payment`. You can now supply
`@handles` anywhere an address is accepted:

```jsonc
{
  "from": "@alice.ipn",
  "to": "@coffee-shop.m",
  "amount": "500000000000000000000000", // 0.5 IPN
  "memo": "latte",
  "signing_key": "7db1…",
  "nonce": null
}
```

The RPC will:

1. Normalize the handle string.
2. Resolve it via `L2HandleRegistry`.
3. Replace it with the owner’s 32-byte address before signing and forwarding the
   transaction to consensus.

Errors are surfaced with specific codes:

| Error | Meaning |
|-------|---------|
| `invalid_handle` | String failed validation (missing `@`, bad suffix, etc.). |
| `handle_not_found` | Registry has no metadata for that handle. |
| `handle_lookup_failed` | Registry backend errored (try again or check node logs). |

---

## 5. Wallet CLI Support

The Phase 2 wallet CLI now accepts handles transparently:

```bash
ippan-wallet --rpc-url http://localhost:8081 send-payment \
  --key ./keys/dev.key \
  --prompt-password \
  --from @alice.ipn \
  --to @friend.ipn \
  --amount 0.25 \
  --memo "local demo" \
  --yes
```

The CLI simply forwards the handle strings to `/tx/payment`; the node performs
the final resolution.

---

## 6. End-to-End Demo

1. Start the [local full-stack environment](../dev/local-full-stack.md).
2. Generate a key and register `@alice.ipn`.
3. Fund it via `/dev/fund`.
4. Send a payment from `@alice.ipn` to `@friend.ipn`.
5. Watch the transaction appear in the unified UI (http://localhost:3000).

For a scripted version see `scripts/smoke_wallet_cli.sh` (set
`TO_ADDRESS='@friend.ipn'` to test handle payments).

---

## Troubleshooting

| Symptom | Next Steps |
|---------|------------|
| `invalid_handle` | Ensure you include the `@` prefix and a suffix (e.g. `@user.ipn`). Trim whitespace. |
| `handle_not_found` | The handle is not registered or has expired. Query `/handle/<handle>` to confirm. |
| `handle_lookup_failed` | The in-memory registry or DHT returned an error; check node logs and retry. |
| Payment succeeds but explorer shows a raw address | Handles are aliases. The explorer always renders the canonical address, but you can still search by handle via `/handle/<handle>`. |
