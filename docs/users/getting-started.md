# Getting Started with IPPAN (Users)

This guide is for non-developers who want to try IPPAN on devnet/testnet.

## 1. What you need

- A laptop with macOS, Windows (WSL), or Linux.
- Basic command-line access (Terminal/PowerShell) for running the wallet CLI.
- A network endpoint (devnet/testnet). For local experiments use `http://127.0.0.1:8080` from the [Local Full-Stack Guide](../dev/local-full-stack.md). For shared testnets use the URL published by your operator.

## 2. Create a wallet

```bash
curl -L https://github.com/dmrl789/IPPAN/releases/latest/download/ippan-wallet -o ippan-wallet
chmod +x ippan-wallet
./ippan-wallet generate-key --output ~/.ippan/user.key
./ippan-wallet show-address --key ~/.ippan/user.key
```

Take note of:

- `Address`: 32-byte hex string (used by SDKs/explorer).
- `Handle` (optional): register one later with `./ippan-wallet handle register ...`.
- Store `user.key` somewhere safe (USB drive, password manager). Treat it like cash—anyone with the file can move your funds.

## 3. Fund the wallet

- **Local devnet**: use the faucet helper `./ippan-wallet send-dev-funds --key ~/.ippan/dev.key --to 0xYOURADDR`.
- **Shared testnet**: request IPN from your operator (Discord/Slack) or run the faucet command provided in the testnet announcement.

Confirm the balance:

```bash
curl http://127.0.0.1:8080/account/0xYOURADDR | jq '.balance'
```

## 4. Send a payment

```bash
./ippan-wallet send-payment \
  --key ~/.ippan/user.key \
  --to 0xRECIPIENT \
  --amount 1000000 \
  --rpc http://127.0.0.1:8080
```

You can also send to handles (`--to @alice.ipn`) once the recipient registers one (see [Handles & Addresses](handles-and-addresses.md)).

Track status:

- CLI prints the transaction hash — query it via `curl http://127.0.0.1:8080/tx/<hash>`.
- Explorer UI (if available) lets you paste the hash or your handle to see history.

## 5. Receive payments

1. Share your address (`0x...`) or handle (`@you.ipn`) with the sender.
2. Watch the explorer or run `curl http://127.0.0.1:8080/account/0xYOURADDR | jq`.
3. For invoices, include a memo string (`--memo "Invoice #123"`) in the payment CLI.

## 6. Stay safe

- Keep wallet files offline when not in use; make encrypted backups.
- Verify RPC URLs — only use endpoints you trust (local node, official testnet gateway).
- Rotate handles if you lose a key (register a new handle and inform contacts).

Need help? Start with the [Developer Journey](../dev/developer-journey.md) to spin up your own network, then share feedback in the community channels.
