# IPN Handles vs. Addresses

Human-readable handles (e.g. `@alice.ipn`) are first-class citizens in IPPAN.
They wrap the same 32-byte Ed25519 public keys used for account addresses, but
provide friendlier UX for wallets, explorers, and operator tooling.

This note explains the relationship between handles and addresses, how to
register and resolve handles, and how to send payments using the new CLI and
RPC features.

---

## 1. Addresses in Review

- **Address format:** Base58Check strings starting with `i…` plus a hex variant
  (64 lowercase characters, optional `0x` prefix).
- **Canonical bytes:** 32 bytes (Ed25519 public key). All RPC state, storage,
  and consensus logic operate on these bytes.
- **Where they show up:** account storage, transaction `from`/`to`, validator
  IDs, and CLI outputs.

---

## 2. Handle Format & Validation

Handles are strings validated by `ippan_types::Handle` and the L2 registry:

| Rule | Description |
|------|-------------|
| Prefix | Must begin with `@`. |
| Suffix | Must include a dot-separated suffix such as `.ipn`, `.iot`, `.m`, or `.cyborg`. |
| Length | Between 4 and 63 characters inclusive. |
| Characters | ASCII letters/digits/`_`/`.` (enforced by clients; registry rejects invalid Unicode). |
| Premium TLDs | `.cyborg`, `.iot`, `.m` are marked premium (see `Handle::is_premium`). |

Handles live on **L2** (see `docs/L2_HANDLE_SYSTEM.md` for the architecture),
while **L1** only stores lightweight ownership anchors.

---

## 3. Registering a Handle

Use `POST /handle/register` (dev mode or validator RPC) to register a handle:

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

The handler builds an on-chain transaction carrying `HandleOperation::Register`.
Once finalized, the L2 registry records `@alice.ipn → owner public key` and
exposes it via the RPC.

### Inspect Handle Metadata

```bash
curl -sS http://127.0.0.1:8080/handle/%40alice.ipn | jq
```

---

## 4. Resolving Handles in Payments

**Phase 3 adds automatic resolution for `from` and `to` fields in `/tx/payment`.**
You can now submit handles anywhere an address is accepted.

Example payload:

```json
{
  "from": "@alice.ipn",
  "to": "@coffee-shop.m",
  "amount": "500000000000000000000000", // 0.5 IPN
  "memo": "latte",
  "signing_key": "7db1…",
  "fee": null,
  "nonce": null
}
```

The RPC:

1. Normalizes the handles (`normalize_handle_input`).
2. Resolves them through `L2HandleRegistry`.
3. Substitutes the underlying 32-byte addresses before signing.

If a handle cannot be found the RPC now returns `404 handle_not_found`.

---

## 5. Wallet CLI Support

`ippan-wallet` (Phase 2 CLI) now accepts handles transparently:

```bash
# Send 0.25 IPN to a handle
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

The CLI simply forwards the handle strings; resolution happens inside the RPC.

---

## 6. Scripts & Automation

Use `scripts/smoke_wallet_cli.sh` as a starting point. To incorporate handles:

1. Register a handle for the generated key (see section 3).
2. Update the script’s `TO_ADDRESS` env var to `@recipient.ipn`.
3. Run the smoke test; it now shows handle-to-handle transfers.

---

## 7. Troubleshooting

| Symptom | Next Steps |
|---------|------------|
| `handle_not_found` | Confirm the handle was registered and not expired. Query `/handle/<handle>` to inspect metadata. |
| `invalid_handle` | Ensure the string starts with `@` and includes a suffix, e.g. `@user.ipn`. Trim whitespace. |
| `handle_lookup_failed` | The L2 registry returned an internal error. Check node logs and ensure the registry is initialized (dev nodes use in-memory registries). |
| Payment fails with `account_not_found` | Handle resolves correctly, but the owner account has never been funded. Deposit IPN (or use `/dev/fund` in dev mode). |

---

## 8. Related Documents

- [`docs/L2_HANDLE_SYSTEM.md`](../L2_HANDLE_SYSTEM.md) – deep dive into the registry architecture.
- [`docs/PAYMENT_API_GUIDE.md`](../PAYMENT_API_GUIDE.md) – updated request fields and examples.
- [`docs/dev/wallet-cli.md`](../dev/wallet-cli.md) – CLI walkthrough including signing and payments.
- [`docs/demo_end_to_end_ippan.md`](../demo_end_to_end_ippan.md) – broader demo flow (update handles section as features land).

Handles and addresses now share the same RPC surfaces, so existing tooling keeps
working while new UX layers—wallets, explorers, SDKs—can prefer friendly
identifiers.
