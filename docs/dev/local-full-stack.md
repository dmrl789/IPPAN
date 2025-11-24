# Local Full-Stack Environment

Run the entire IPPAN stack (node + gateway + explorer UI) on your workstation with a single command.

## Prerequisites

- Docker Engine 20.10+ (or Docker Desktop)
- Docker Compose plugin (`docker compose`) or the legacy `docker-compose` binary
- ~8 GB RAM available for containers
- `ippan-wallet` CLI built locally (optional but recommended for the walkthrough)

## Quick Start

```bash
scripts/run-local-full-stack.sh
```

The script will:

1. Ensure Docker + Compose are available.
2. Build fresh images for the node, gateway, and unified UI directly from the repo.
3. Start the stack in detached mode using `localnet/docker-compose.full-stack.yaml`.
4. Print container status plus the URLs you’ll need next.

Stop everything with:

```bash
docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local down
```

## Services & Ports

| Service | URL | Description |
|---------|-----|-------------|
| Node RPC | `http://localhost:8080` | Native RPC + health endpoints. |
| Node P2P | `localhost:9000` | P2P gossip (devnet only; exposed so you can tether other nodes). |
| Gateway API | `http://localhost:8081/api` | REST/WebSocket proxy in front of the node. |
| Gateway WS | `ws://localhost:8081/ws` | Real-time stream mirrored from the node. |
| Unified UI | `http://localhost:3000` | Next.js dashboard/explorer backed by the gateway API. |

All services run on a dedicated Docker bridge network (`ippan-local`). Node data is persisted under `localnet/data/node` on your host so you can restart without losing chain state.

## End-to-End Flow

1. **Start the stack**  
   `scripts/run-local-full-stack.sh`

2. **Generate a devnet key**  
   ```bash
   mkdir -p keys
   ippan-wallet --network devnet generate-key --out ./keys/dev.key --prompt-password
   ```

3. **Fund the account (dev mode)**  
   ```bash
   ADDRESS=$(ippan-wallet show-address --key ./keys/dev.key --json | jq -r '.address')
   curl -sS -X POST http://localhost:8080/dev/fund \
     -H "Content-Type: application/json" \
     -d "{\"address\":\"$ADDRESS\",\"amount\":100000000000000000000000,\"nonce\":0}"
   ```

4. **Send a payment through the gateway**  
   ```bash
   ippan-wallet \
     --rpc-url http://localhost:8081 \
     send-payment \
     --key ./keys/dev.key \
     --prompt-password \
     --to @friend.ipn \
     --amount 0.25 \
     --memo "local stack demo" \
     --yes
   ```

5. **Verify the transaction**  
   - `curl http://localhost:8081/api/tx/<hash>`  
   - or open `http://localhost:3000`, navigate to the explorer view, and search for the hash or account.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| Containers stuck on `starting` | Run `docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local logs -f` to see build errors. |
| Port already in use | Change the host port mappings in `localnet/docker-compose.full-stack.yaml` (e.g., `8080:8080` → `18080:8080`). |
| UI shows “Unable to reach API” | Ensure the gateway container is healthy and that your browser can reach `http://localhost:8081/api/health`. CORS is already configured for `http://localhost:3000`. |
| Need a clean slate | `docker compose ... down -v` removes containers and the persistent volume, then delete `localnet/data/node`. |

## What’s Inside the Compose File?

- **ippan-node**  
  Builds from `docker/Dockerfile.node`, starts in devnet mode, disables security manager, and exposes RPC/P2P ports.

- **ippan-gateway**  
  Builds from `apps/gateway`, proxies `/api` & `/ws` to the node, and enables explorer-prefixed routes.

- **ippan-explorer**  
  Builds the unified Next.js UI with local gateway URLs baked into the static export and serves it via Nginx.

All images are rebuilt each time you run the script, so any code changes in the repo are reflected automatically.

## Related Resources

- [`docs/dev/wallet-cli.md`](./wallet-cli.md) – Using the wallet CLI (key generation, signing, payments).
- [`scripts/smoke_wallet_cli.sh`](../../scripts/smoke_wallet_cli.sh) – Automated devnet payment smoke test (works great against the local stack).
- [`docs/PAYMENT_API_GUIDE.md`](../PAYMENT_API_GUIDE.md) – RPC contract details if you’re hitting endpoints directly.
