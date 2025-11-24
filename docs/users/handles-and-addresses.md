# IPN Handles vs. Addresses

IPPAN lets you reference recipients in two ways:

1. **Addresses** – 32-byte Ed25519 public keys rendered as Base58Check (`i...`) or 64-char hex strings (optionally `0x`-prefixed).
2. **Handles** – human-readable aliases such as `@alice.ipn` that resolve back to the same 32-byte keys.

Human-readable handles (e.g. `@alice.ipn`) are first-class citizens in IPPAN.  
They wrap the same 32-byte Ed25519 public keys used for account addresses, but provide friendlier UX for wallets, explorers, and operator tooling.

Use addresses for low-level tooling or SDK integrations that already manage keys, and prefer handles anywhere you expose flows to end users (wallets, explorers, payments). The gateway/RPC layer translates handles into canonical addresses at runtime, so you never lose determinism.

This note explains the relationship between handles and addresses, how to register and resolve handles, and how to send payments using the wallet CLI and RPC features.

---

## 1. Address Recap

- **Format:** Base58Check strings that start with `i…`, plus a hex variant (64 lowercase characters with an optional `0x` prefix).
- **Canonical form:** Every address ultimately resolves to a 32-byte Ed25519 public key stored in the node’s state database. All RPC state, storage, and consensus logic operate on these bytes.
- **Where they appear:** Node RPC (`/account/:address`, `/tx`, `/block`), gateway/explorer API responses, validator IDs, and the wallet CLI.

---

## 2. Handle Format & Validation

Validation is enforced by `crates/types/src/handle.rs` and `crates/l2_handle_registry`:

| Rule          | Description                                                                 |
|---------------|-----------------------------------------------------------------------------|
| Prefix        | Must start with `@`.                                                        |
| Suffix        | Must include a dot-separated suffix such as `.ipn`, `.iot`, `.m`, `.cyborg`.|
| Length        | Between 4 and 63 characters inclusive.                                      |
| Character set | ASCII alphanumerics plus `_` and `.` (inputs are trimmed before validation).|
| Premium TLDs  | `.cyborg`, `.iot`, `.m` are flagged as premium via `Handle::is_premium`.    |

Handles live on **Layer 2** (see [`L2_HANDLE_SYSTEM.md`](../L2_HANDLE_SYSTEM.md)), while **Layer 1** only stores lightweight ownership anchors that prove who controls a given handle.

---

## 3. Registering a Handle

Use the RPC’s `POST /handle/register` endpoint (available when a node runs with `--dev` or via validator/gateway instances) to submit a `HandleOperation::Register`:

```bash
SIGNING_KEY=$(cat signer.hex) # 32-byte private key in hex
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
