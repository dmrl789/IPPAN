# IPPAN Three-Node Localnet Demo

This guide explains how to spin up a three-node "localnet" topology with libp2p-based IPNDHT and run an end-to-end scenario that exercises handles, payments, and file metadata replication.

## Prerequisites

- Rust toolchain (same requirements as the main workspace).
- `cargo` on your PATH.
- `curl`; `jq` is optional but recommended for pretty-printing responses.
- Three free port pairs on `127.0.0.1` (RPC: 3111-3113, P2P: 4111-4113, IPNDHT: 9211-9213).

## Localnet configuration layout

The repo now ships with ready-made configs under `localnet/`:

| Node | RPC | P2P | IPNDHT | Config file |
|------|-----|-----|--------|-------------|
| Node 1 (A) | `127.0.0.1:3111` | `127.0.0.1:4111` | `/ip4/127.0.0.1/tcp/9211` | `localnet/node1.toml` |
| Node 2 (B) | `127.0.0.1:3112` | `127.0.0.1:4112` | `/ip4/127.0.0.1/tcp/9212` | `localnet/node2.toml` |
| Node 3 (C) | `127.0.0.1:3113` | `127.0.0.1:4113` | `/ip4/127.0.0.1/tcp/9213` | `localnet/node3.toml` |

Each config enables:

- Unique node IDs and validator IDs for deterministic identities.
- Distinct RPC/P2P/IPNDHT ports.
- libp2p-backed file + handle DHTs with bootstrap peers pointing at the other nodes.
- DLC enabled with relaxed validator bonding for local demos.
- `DEV_MODE=true` so `/dev/fund` is accessible from localhost.
- Security manager disabled to simplify cURL-based testing.

## Starting and stopping the cluster

From the repo root:

```bash
scripts/localnet_start.sh
```

The script builds the `ippan-node` binary (if needed), starts all three nodes in the background, and writes logs to `localnet/node{1,2,3}.log`. PID files live next to the configs to simplify cleanup.

To stop the cluster:

```bash
scripts/localnet_stop.sh
```

This sends `SIGTERM` (and escalates to `SIGKILL` if a node does not exit) and removes the PID files.

## Running the basic scenario

With the nodes running, execute:

```bash
scripts/localnet_scenario_basic.sh
```

What the script does:

1. Health-checks all RPC endpoints.
2. Generates two demo keypairs (stored in `localnet/keys/`) if they do not already exist.
3. Funds both accounts on every node through `/dev/fund` to keep local storage in sync.
4. Registers `@demo1.ipn` on node A using the handle registry endpoint.
5. Submits a signed payment from account A → account B via node A.
6. Publishes a file descriptor on node B that references account B as owner.
7. Polls node C until the handle, payment history, and file descriptor are visible, proving that libp2p + HTTP P2P replication worked.

By default the script prints raw JSON; if `jq` is installed the responses are pretty-printed. Environment variables allow overriding RPC URLs, funding amounts, memo text, and the handle/file metadata for experimentation.

If any step fails (node offline, RPC rejects the request, data never appears on node C), the script exits non-zero so you can inspect the relevant `localnet/node*.log` file.

## Manual verification checklist

- `curl http://127.0.0.1:3113/handle/@demo1.ipn` should return the registered metadata within a few seconds.
- `curl http://127.0.0.1:3113/account/<B_HEX>/payments?limit=5` should list the payment with direction `incoming`.
- `curl http://127.0.0.1:3113/files/<FILE_ID>` should return the descriptor published on node 2 with `dht_published` set to `true`.

## Future extensions

- Swap DLC/PoA settings or validator IDs to match other devnets.
- Extend the scenario script to cover L2 commits or AI/DLC health checks.
- Wrap the flow inside docker-compose if a containerized setup is needed later.
