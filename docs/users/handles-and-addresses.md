# IPN Handles vs. Addresses

IPPAN lets you reference accounts in two equivalent ways:

1. **Addresses** – 32-byte Ed25519 public keys rendered as Base58Check (`i…`) or 64-char hex strings (optional `0x` prefix). These are the canonical values persisted on-chain.
2. **Handles** – human-friendly aliases such as `@alice.ipn` that map back to the same 32-byte keys. Handles are stored in the L2 registry and anchored on L1 for ownership.

Handles are first-class citizens: the RPC, gateway, explorer, and wallet CLI all accept them anywhere an address is valid. Tooling that already manages keys can keep using raw addresses, while UX-oriented flows can prefer handles without sacrificing determinism.

---

## 1. Addresses in Review

- **Address format:** Base58Check strings starting with `i…` plus a hex variant (64 lowercase characters, optional `0x` prefix).
- **Canonical bytes:** 32-byte Ed25519 public keys. All consensus, storage, and RPC logic ultimately works with these bytes.
- **Where they appear:** node RPC (`/account/:address`, `/tx`, `/block`), gateway/explorer APIs, validator IDs, and wallet CLI output.

---

## 2. Handle Format & Validation

Validation is enforced by `ippan_types::Handle` and the L2 registry:

| Rule | Description |
|------|-------------|
| Prefix | Must begin with `@`. |
| Suffix | Must include a dot-separated suffix such as `.ipn`, `.iot`, `.m`, or `.cyborg`. |
| Length | Between 4 and 63 characters inclusive. |
| Characters | ASCII letters, digits, `_`, and `.` (clients trim whitespace before validation). |
| Premium TLDs | `.cyborg`, `.iot`, `.m` are flagged via `Handle::is_premium`. |

Handles live on **Layer 2** (see [`L2_HANDLE_SYSTEM.md`](../L2_HANDLE_SYSTEM.md)) while **Layer 1** stores lightweight ownership anchors. This keeps fees low and allows fast updates without sacrificing verifiability.

---

## 3. Registering a Handle

Use `POST /handle/register` (dev mode or validator/gateway RPC) to create a handle:

```bash
SIGNING_KEY=$(cat signer.hex) # 32-byte hex private key
OWNER_ADDRESS=$(ippan-wallet show-address --key ./keys/dev.key --json | jq -r '.address')

curl -sS -X POST http://127.0.0.1:8080/handle/register \
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

The handler builds a `HandleOperation::Register` transaction. Once finalized, the L2 registry records `@alice.ipn → owner public key` and serves it via RPC/DHT.

### Inspect Handle Metadata

```bash
curl -sS http://127.0.0.1:8080/handle/%40alice.ipn | jq
```

---

## 4. Resolving Handles in Payments

Phase 3 added automatic handle resolution to `/tx/payment`. You can now supply `@handles` anywhere an address is accepted.

```json
{
  "from": "@alice.ipn",
  "to": "@coffee-shop.m",
  "amount": "500000000000000000000000", // 0.5 IPN
  "memo": "latte",
  "fee": null,
  "signing_key": "7db1…",
  "nonce": null
}
```

The RPC:

1. Normalizes the incoming strings.
2. Resolves them through `L2HandleRegistry`.
3. Substitutes the underlying 32-byte addresses before signing and forwarding the transaction to consensus.

Error codes:

| Error | Meaning |
|-------|---------|
| `invalid_handle` | String failed validation (missing `@`, bad suffix, etc.). |
| `handle_not_found` | Registry has no metadata for that handle. |
| `handle_lookup_failed` | Registry backend errored (retry or check node logs). |

---

## 5. Wallet CLI Support

`ippan-wallet` accepts handles transparently:

```bash
ippan-wallet \
  --rpc-url http://127.0.0.1:18080 \
  send-payment \
  --key ./keys/devnet.key \
  --prompt-password \
  --from @alice.ipn \
  --to @friend.ipn \
  --amount 0.25 \
  --memo "soda" \
  --yes
```

The CLI forwards the handle strings; resolution happens in the RPC before signing.

---

## 6. Scripts & Automation

Use `scripts/smoke_wallet_cli.sh` as an end-to-end test:

1. Generate a devnet key (`ippan-wallet generate-key ...`).
2. Register a handle for that key (section 3).
3. Set `TO_ADDRESS='@friend.ipn'` (or another handle) before running the script.

The smoke test now exercises handle registration, funding, payment submission, and RPC verification.

---

## 7. End-to-End Demo (Manual)

1. Start the [local full-stack environment](../dev/local-full-stack.md).
2. Generate a key and register `@alice.ipn`.
3. Fund it via `/dev/fund`.
4. Send a payment from `@alice.ipn` to `@friend.ipn`.
5. Watch the transaction appear in the unified UI (`http://localhost:3000`).

---

## 8. Best Practices & Security Notes

- Treat handles as public metadata—avoid embedding secrets or PII.
- Renew handles before `expires_at` to avoid squatting.
- Store wallet key files with `chmod 600`; the same key signs handle ops and payments.
- Surface both the handle and canonical address in SDK/UI flows so users can verify the resolved recipient.

---

## 9. Troubleshooting

| Symptom | Next Steps |
|---------|------------|
| `invalid_handle` | Ensure the string starts with `@`, includes a suffix (e.g. `@user.ipn`), and contains only allowed characters. |
| `handle_not_found` | Handle is unregistered or expired. Query `/handle/<handle>` to confirm. |
| `handle_lookup_failed` | Registry/DHT returned an error; check node logs and retry. |
| Payment fails with `account_not_found` | Handle resolves, but the owner account has never been funded—send IPN or use `/dev/fund`. |
| Explorer shows only addresses | Handles are aliases; explorers render canonical addresses but `/handle/<handle>` endpoints still work. |

---

## 10. Related Documents

- [`docs/L2_HANDLE_SYSTEM.md`](../L2_HANDLE_SYSTEM.md) – registry architecture.
- [`docs/PAYMENT_API_GUIDE.md`](../PAYMENT_API_GUIDE.md) – updated request fields and error codes.
- [`docs/dev/wallet-cli.md`](../dev/wallet-cli.md) – CLI flows for signing, payments, and handles.
- [`docs/dev/sdk-overview.md`](../dev/sdk-overview.md) – Rust/TypeScript SDK helpers for the payment API.

Handles and addresses now share the same RPC surfaces, so existing tooling keeps working while wallets, explorers, and SDKs can default to human-readable identifiers.
