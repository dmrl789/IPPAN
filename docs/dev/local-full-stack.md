# Local Full-Stack Guide

`scripts/run-local-full-stack.sh` now supports two workflows:

| Mode | How to invoke | Result |
| ---- | ------------- | ------ |
| **Docker Compose (default)** | `scripts/run-local-full-stack.sh` | Builds container images and launches node + gateway + unified UI via `localnet/docker-compose.full-stack.yaml`. |
| **Native 3-node localnet** | `scripts/run-local-full-stack.sh --native` | Builds the Rust workspace and launches three validators locally without Docker. |

Pick the mode that matches your environment and follow the relevant section below.

---

## Mode 1 – Docker Compose Stack (default)

### Prerequisites

- Docker Engine 20.10+ or Docker Desktop
- Docker Compose plugin (`docker compose`) or the legacy `docker-compose` binary
- ~8 GB RAM available for containers
- `ippan-wallet` CLI built locally (optional, but used in the walkthrough)

### Quick Start

```bash
scripts/run-local-full-stack.sh
```

The script will:

1. Verify that Docker + Compose are available.
2. Build fresh images for the node, gateway, and unified UI directly from the repo.
3. Start the stack in detached mode using `localnet/docker-compose.full-stack.yaml`.
4. Print container status plus the URLs you’ll need next.

Stop everything with:

```bash
docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local down
```

### Services & Ports

| Service | URL | Description |
|---------|-----|-------------|
| Node RPC | `http://localhost:8080` | Native RPC + health endpoints |
| Node P2P | `localhost:9000` | P2P gossip (devnet only; exposed for tethering) |
| Gateway API | `http://localhost:8081/api` | REST/WebSocket proxy in front of the node |
| Gateway WS | `ws://localhost:8081/ws` | Real-time stream mirrored from the node |
| Unified UI | `http://localhost:3000` | Next.js explorer backed by the gateway API |

All services share the `ippan-local` Docker network. Node data persists under `localnet/data/node` so you can restart without losing state.

### End-to-End Flow

1. **Start the stack**: `scripts/run-local-full-stack.sh`
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
5. **Verify the transaction** via `curl http://localhost:8081/api/tx/<hash>` or open `http://localhost:3000` and search through the explorer UI.

### Troubleshooting

| Issue | Fix |
|-------|-----|
| Containers stuck on `starting` | `docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local logs -f` to inspect build/runtime errors. |
| Port already in use | Change host mappings in the compose file (e.g., `8080:8080` → `18080:8080`). |
| UI shows “Unable to reach API” | Ensure the gateway container is healthy and reachable at `http://localhost:8081/api/health`. CORS already includes `http://localhost:3000`. |
| Need a clean slate | `docker compose ... down -v` followed by removing `localnet/data/node`. |

### What’s Inside the Compose File?

- **ippan-node** – Builds from `docker/Dockerfile.node`, starts in devnet mode, disables the security manager, and exposes RPC/P2P ports.
- **ippan-gateway** – Builds from `apps/gateway`, proxies `/api` & `/ws` to the node, and exposes explorer-friendly routes.
- **ippan-explorer** – Builds the Next.js UI with local gateway URLs baked into the export and serves it via Nginx.

Every run rebuilds the images so local code changes are reflected automatically.

---

## Mode 2 – Native Three-Node Localnet (`--native`)

### Prerequisites

- Linux/macOS host with the Rust toolchain (1.71+) installed
- `cargo` on your `$PATH`
- Ports `8080-8082` (RPC) and `9000-9002` (P2P) available on loopback

### Quick Start

```bash
git clone https://github.com/dmrl789/IPPAN.git
cd IPPAN
scripts/run-local-full-stack.sh --native
```

What the native flow does:

1. Runs `cargo build --workspace` for up-to-date binaries.
2. Stops any lingering localnet with `scripts/localnet_stop.sh`.
3. Launches three validators via `scripts/localnet_start.sh`, each using `localnet/node{1,2,3}.toml`.

Once the script prints the endpoint table you can:

- Submit payments: `curl -X POST http://127.0.0.1:8080/tx/payment ...`
- Query accounts: `curl http://127.0.0.1:8080/account/<address>`
- Check network health: `curl http://127.0.0.1:8080/health`

Stop everything with:

```bash
scripts/localnet_stop.sh
```

### Tips

- Run `RUST_LOG=debug scripts/run-local-full-stack.sh --native` for verbose logs.
- Tweak validator IDs or port allocations in `localnet/*.toml` before relaunching.
- Logs stream to `localnet/node*.log`; tail them with `tail -f localnet/node1.log`.
- Need the unified UI against native validators? Run the Docker mode in a second terminal and point `RPC_HOST` to `http://127.0.0.1:8080`.

Need more realism (dockerized gateway/explorer) or a hybrid setup? Follow the [Gateway & Explorer Runbook](../operators/gateway-explorer-runbook.md) and wire those services to your native nodes.

---

## Related Resources

- [`docs/dev/developer-journey.md`](./developer-journey.md)
- [`docs/dev/sdk-overview.md`](./sdk-overview.md)
- [`docs/operators/production-validator-runbook.md`](../operators/production-validator-runbook.md)
- [`docs/operators/gateway-explorer-runbook.md`](../operators/gateway-explorer-runbook.md)
