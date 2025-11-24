# IPPAN Wallet CLI

The `ippan-wallet` binary provides a focused toolkit for operators and
developers who need to:

- generate and store Ed25519 keys that map to IPN addresses,
- inspect addresses and metadata,
- sign arbitrary payloads or transaction blobs,
- craft and submit payment transactions through the node RPC API.

The CLI is installed automatically with `cargo build --bin ippan-wallet` and
relies only on standard workspace dependencies (no OpenSSL bindings required).

> **Prerequisites**
> - Rust toolchain (per `rust-toolchain.toml`)
> - `jq` and `curl` for the smoke test script
> - A running `ippan-node` instance; devnet flows assume the node started with
>   the `--dev` flag or `IPPAN_DEV_MODE=true`.

---

## 1. Key management

Generate a password-protected key file for the devnet profile:

```bash
ippan-wallet --network devnet generate-key \
  --out ./keys/devnet.key \
  --prompt-password
```

Flags of note:

| Flag | Purpose |
|------|---------|
| `--network devnet|testnet|mainnet` | Annotates the key metadata with the intended network. |
| `--prompt-password` / `--password-file` | Securely capture the password used for AES-GCM encryption. |
| `--insecure-plaintext` | Store the key unencrypted (only for automated dev/test flows). |
| `--print-private-key` | Echo the private key hex to stdout with a warning. |

To inspect the derived address and metadata:

```bash
ippan-wallet show-address \
  --key ./keys/devnet.key \
  --prompt-password \
  --json
```

The JSON output contains the Base58Check address, public key, creation
timestamp, network profile, and any user notes.

---

## 2. Signing workflows

Sign inline messages:

```bash
ippan-wallet sign \
  --key ./keys/devnet.key \
  --prompt-password \
  --message "hello ipn" \
  --out hello.sig
```

Sign raw transaction blobs or other binary data:

```bash
ippan-wallet sign \
  --key ./keys/devnet.key \
  --prompt-password \
  --file ./payload.bin \
  --raw > payload.sig
```

Options:

- `--message-hex` – treat the payload as hex-encoded bytes.
- `--file` – read raw bytes from a file (mutually exclusive with message flags).
- `--raw` – emit binary signature bytes instead of hex.

---

## 3. Payment submissions

Send a payment against a running node (default RPC URL is
`http://127.0.0.1:8080`):

```bash
ippan-wallet \
  --rpc-url http://127.0.0.1:18080 \
  send-payment \
  --key ./keys/devnet.key \
  --prompt-password \
  --to ippan1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqdsk9t \
  --amount 0.25 \
  --memo "devnet test" \
  --yes
```

Highlights:

| Flag | Purpose |
|------|---------|
| `--rpc-url` | Override the node RPC endpoint (profile-aware defaults are supported via Phase 1 work). |
| `--amount` / `--amount-atomic` | Specify the payment amount in IPN (with up to 24 decimals) or atomic units. |
| `--fee` / `--fee-atomic` | Cap the required fee; defaults to the minimum enforced by the node. |
| `--nonce` | Manually force a nonce; otherwise the CLI fetches the next nonce and displays it before sending. |
| `--memo` | Attach a topic/memo (≤256 bytes). |
| `--yes` | Skip the interactive confirmation prompt. |

If the key file is stored unencrypted (dev/test), omit the password flags. Both
`--from` and `--to` accept any identifier that the RPC understands: Base58Check,
hex (`0x…`), or `@handle` strings (e.g. `@alice.ipn`). Handles are normalized
and resolved on the node before signing.

---

## 4. End-to-end smoke test

The repository includes `scripts/smoke_wallet_cli.sh`, which generates a
temporary key, funds it via `/dev/fund`, submits a signed payment, and verifies
the transaction via `/tx/<hash>`.

```bash
# Start the node in dev mode (requires IPPAN to be built):
ippan-node --dev start &

# In another shell:
chmod +x scripts/smoke_wallet_cli.sh
RPC_URL=http://127.0.0.1:18080 scripts/smoke_wallet_cli.sh
```

Environment variables recognized by the script:

| Variable | Default | Description |
|----------|---------|-------------|
| `RPC_URL` | `http://127.0.0.1:18080` | Target node RPC endpoint. |
| `KEY_PATH` | `./tmp/devnet-wallet.key` | Where the throwaway key is stored. |
| `AMOUNT` | `0.01` | Payment amount (IPN). |
| `TO_ADDRESS` | `auto` | Recipient address; defaults to the sender itself. |

The script requires `jq` and `curl`, and `/dev/fund` is only available when the
node runs with `--dev` or `IPPAN_DEV_MODE=true`.

---

## 5. Next steps

- Pair the CLI with the Phase 1 network profiles by exporting
  `IPPAN_NETWORK=testnet` when targeting the RC network.
- Integrate the signing commands into CI pipelines or SDK examples by invoking
  `ippan-wallet sign --file payload.bin --raw`.
- For production operations, store keys on encrypted filesystems and rely on
  password prompts or hardware-backed key management (planned for future
  phases).
