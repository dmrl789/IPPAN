# IPPAN v0.9.0-rc1 Testnet Guide

This guide shows how to spin up an IPPAN v0.9.0-rc1 Release Candidate testnet locally (single-node or a tiny 3-node mesh), run a few basic flows, and share feedback.

## Prerequisites
- Linux/macOS shell with `bash` and `curl` (Linux recommended)
- Rust toolchain + `cargo`
- Git (for cloning)

## Build the RC node
```bash
git clone https://github.com/dmrl789/IPPAN.git
cd IPPAN
cargo build --release -p ippan-node
```

## Run a single-node RC testnet
```bash
scripts/testnet/run-local-single-node.sh
```
- Config: `testnet/node1.toml`
- Data dir: `testnet/data/node1`
- Logs: `testnet/logs/node1.log` (also streamed to the terminal)
- Ports: RPC `127.0.0.1:3111`, P2P `127.0.0.1:4111`, IPNDHT `/ip4/127.0.0.1/tcp/9211`
- Stop with `Ctrl+C` (process exits and the log file is preserved)

## Run a 3-node mini-network (optional)
```bash
scripts/testnet/run-local-multi-node.sh
```
- Configs: `testnet/node{1,2,3}.toml`
- Data dirs: `testnet/data/node{1,2,3}`
- Logs: `testnet/logs/node{1,2,3}.log`
- All nodes share the same RC network ID (`ippan-rc1`) and bootstrap to one another.
- Stop with `Ctrl+C` to terminate all three nodes.

## Quick health checks
- RPC health: `curl -s http://127.0.0.1:3111/health | jq`
- Consensus status: `curl -s http://127.0.0.1:3111/status | jq '.consensus'`
- Peer count (mini-network): `curl -s http://127.0.0.1:3112/health | jq '.peer_count'`

## Basic RC test scenarios
1. **Generate demo keys**
   ```bash
   cargo run -p ippan-keygen -- generate --output testnet/keys --name sender
   cargo run -p ippan-keygen -- generate --output testnet/keys --name receiver

   SENDER_HEX=$(tr -d '\n' < testnet/keys/sender_public.key | tr '[:upper:]' '[:lower:]')
   RECEIVER_HEX=$(tr -d '\n' < testnet/keys/receiver_public.key | tr '[:upper:]' '[:lower:]')
   SENDER_ADDR="i${SENDER_HEX}"
   RECEIVER_ADDR="i${RECEIVER_HEX}"
   ```

2. **Fund the sender (dev helper exposed in configs)**
   ```bash
   curl -sS -H "Content-Type: application/json" \
     -d "{\"address\":\"${SENDER_ADDR}\",\"amount\":1000000000,\"nonce\":0}" \
     http://127.0.0.1:3111/dev/fund | jq
   ```

3. **Send a payment via the CLI**
   ```bash
   cargo run -p ippan-cli -- \
     --rpc-url http://127.0.0.1:3111 \
     pay \
     --from "${SENDER_ADDR}" \
     --to "${RECEIVER_ADDR}" \
     --amount 250000000 \
     --fee 2000 \
     --key-file testnet/keys/sender_private.key \
     --memo "rc testnet payment"
   ```

4. **Verify inclusion & history**
   ```bash
   # Replace <TX_HASH> with the value printed above
   TX=<TX_HASH>
   curl -s http://127.0.0.1:3111/tx/${TX} | jq
   curl -s "http://127.0.0.1:3111/account/${SENDER_HEX}/payments?limit=5" | jq
   curl -s "http://127.0.0.1:3111/account/${RECEIVER_HEX}/payments?limit=5" | jq
   ```

5. **(Mini-network) Check replication**
   ```bash
   # Confirm the receiver's history is visible on another node
   curl -s "http://127.0.0.1:3113/account/${RECEIVER_HEX}/payments?limit=5" | jq
   ```

For more detailed payment walkthroughs, see `docs/payments/demo_end_to_end_payment.md`.

## How to report RC bugs or feedback
- Open a GitHub issue using the "RC Bug Report" or "RC Feedback / UX" templates.
- Include your RC build (git commit or tag), OS/architecture, and whether you ran single-node or multi-node.
- Attach relevant logs from `testnet/logs/` when possible.
