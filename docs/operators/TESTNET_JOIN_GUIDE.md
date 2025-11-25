# IPPAN Public RC Testnet Join Guide

> **Release Candidate warning:** This RC testnet is not production. Expect network
> restarts, config updates, and chain resets. Tokens/IPN on this network have no
> economic value.

## Prerequisites
- Rust toolchain (stable 1.78+ with `cargo`) for native builds.
- Docker + docker-compose plugin (for the container option).
- Open network ports: `8080/tcp` for RPC and `9000/tcp` for P2P gossip.
- Disk space for chain data (defaults to `./data`).

## Configuration assets
All testnet-ready configs live in `testnet/testnet-config/`:
- `node-config.toml` — baseline RC testnet node config (network id, ports, seed list).
- `logging.toml` — sample log profiles.
- `seed-nodes.txt` — placeholder seed peers to bootstrap; replace with the published list when available.
- `docker-compose.testnet.yml` — minimal compose to run a node against the RC testnet.

The RC network id is `ippan-public-rc-testnet` and should not be changed. Keep
these files separate from any `localnet` or dev configs to avoid accidental
cross-connection.

## Option 1: Native binary
1. Clone and build:
   ```bash
   git clone https://github.com/dmrl789/IPPAN.git
   cd IPPAN
   cargo build --release -p node
   ```
2. Copy configs to a writable location and edit as needed (node id, bootstrap
   seeds, paths):
   ```bash
   mkdir -p ~/ippan-testnet
   cp testnet/testnet-config/node-config.toml ~/ippan-testnet/
   cp testnet/testnet-config/logging.toml ~/ippan-testnet/
   cp testnet/testnet-config/seed-nodes.txt ~/ippan-testnet/
   ```
3. Update `bootstrap_nodes` in `node-config.toml` with the current seeds from
   `seed-nodes.txt` (or operator-provided endpoints).
4. Start the node using the testnet config:
   ```bash
   ./target/release/node --config ~/ippan-testnet/node-config.toml
   ```

## Option 2: Docker Compose
1. Copy the compose file and configs to a working directory:
   ```bash
   mkdir -p ~/ippan-testnet && cd ~/ippan-testnet
   cp /workspace/IPPAN/testnet/testnet-config/node-config.toml .
   cp /workspace/IPPAN/testnet/testnet-config/logging.toml .
   cp /workspace/IPPAN/testnet/testnet-config/seed-nodes.txt .
   cp /workspace/IPPAN/testnet/testnet-config/docker-compose.testnet.yml ./docker-compose.yml
   ```
2. Edit `node-config.toml` to point `bootstrap_nodes` at the latest RC seeds.
3. Build and start:
   ```bash
   docker compose up --build
   ```
   The service exposes RPC on `localhost:8080` and P2P on `localhost:9000` by default.

## Verify your node
- **Logs:** Confirm startup shows `network.id = ippan-public-rc-testnet` and that
  peers begin connecting to the listed seeds.
- **Health endpoint:**
  ```bash
  curl -s http://localhost:8080/health | jq
  ```
  Expect a JSON object with `status` and your node id.
- **Sample RPC (replace hash as needed):**
  ```bash
  curl -s -X POST http://localhost:8080/rpc -H 'Content-Type: application/json' \
    -d '{"method":"chain.get_block","params":{"hash":"<block-hash>"}}'
  ```
  A valid response indicates RPC routing is working.

## Safety and limitations
- RC testnet state can be reset at any time; do not treat stored data as durable.
- No real-value assets exist on this network; all IPN/test assets are void of
  economic value.
- Config schemas may change before production; re-apply updates when new RC drops
  are announced.
