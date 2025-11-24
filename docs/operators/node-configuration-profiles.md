# IPPAN Node Configuration Profiles

IPPAN now ships with three explicit network profiles that bundle sane defaults
for local development, public RC testing, and production-grade deployments.
Each profile corresponds to a TOML file under `config/` and can be selected
either via CLI flag or the `IPPAN_NETWORK` environment variable.

## Selecting a profile

- CLI: `ippan-node --network devnet|testnet|mainnet start`
- Env: `export IPPAN_NETWORK=testnet`
- Defaults: `devnet` if neither CLI nor env overrides are provided.
- Custom config: `--config /path/to/custom.toml` loads that file and still
  applies the selected profile’s validation rules.

Environment variables (prefixed with `IPPAN_`) always override the loaded
configuration. Common overrides include `IPPAN_RPC_PORT`, `IPPAN_P2P_PORT`,
`IPPAN_DATA_DIR`, `IPPAN_DB_PATH`, and `IPPAN_BOOTSTRAP_NODES`.

## Profile summary

| Profile | Config file | RPC (host:port) | P2P (host:port) | Data dir | Logging | Bootstrap expectation |
|---------|-------------|-----------------|-----------------|----------|---------|-----------------------|
| devnet  | `config/devnet.toml`   | `127.0.0.1:18080` | `0.0.0.0:19000` | `./data/devnet` | `debug` + `pretty` | None (empty list allowed) |
| testnet | `config/testnet.toml`  | `0.0.0.0:28080`   | `0.0.0.0:29000` | `./data/testnet` | `info` + `json` | Required (`rc-node1/2.testnet…`) |
| mainnet | `config/mainnet.toml`  | `127.0.0.1:38080` | `0.0.0.0:39000` | `/var/lib/ippan` | `warn` + `json` | Required (`bootstrap{1,2}.mainnet…`) |

> ℹ️ Testnet and mainnet fail fast at startup if the effective
> `p2p.bootstrap_nodes` list is empty. Override via TOML or
> `IPPAN_BOOTSTRAP_NODES` when pointing to alternate peers.

## Example commands

- **Devnet (local laptop)**
  ```bash
  cargo run -p node -- --network devnet start
  ```
- **Testnet (public RC)**
  ```bash
  export IPPAN_NETWORK=testnet
  ./target/release/ippan-node start
  ```
- **Mainnet (hardened host)**
  ```bash
  ./target/release/ippan-node \
    --network mainnet \
    --config /etc/ippan/mainnet.toml \
    start
  ```

### Overriding specific fields

```bash
IPPAN_RPC_HOST=0.0.0.0 \
IPPAN_RPC_PORT=8081 \
IPPAN_DATA_DIR=/srv/ippan/testnet \
ippan-node --network testnet start
```

The CLI `--data-dir`, `--rpc-port`, `--p2p-port`, and `--log-level` flags still
apply on top of the selected profile.

## Bootstraps and validation

- **Devnet** defaults to an empty bootstrap list so isolated local clusters can
  start without internet connectivity.
- **Testnet** ships with the current RC peers:
  - `http://rc-node1.testnet.ippan.network:29000`
  - `http://rc-node2.testnet.ippan.network:29000`
- **Mainnet** placeholders are provided:
  - `https://bootstrap1.mainnet.ippan.network:39000`
  - `https://bootstrap2.mainnet.ippan.network:39000`

The node refuses to start on testnet or mainnet if bootstrap nodes are missing
or blank, reducing footguns for operators.

## Where to go next

- Full RC operator guide: `docs/operators/running-ippan-rc-node.md`
- Profile-aware wallet and payment flows: (Phase 2 doc TBD)
- Raise issues or submit PRs when the canonical bootstrap list evolves so the
  defaults stay in sync.
