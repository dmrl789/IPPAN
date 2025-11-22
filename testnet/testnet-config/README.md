# IPPAN Public RC Testnet Configuration Samples

This folder packages operator-facing configuration for the **public RC testnet**.
These files are intentionally separate from `localnet/` or dev configs to avoid
mix-ups when joining the shared network.

## Files
- `node-config.toml` — baseline node settings with the RC testnet network ID and
  seed list placeholders.
- `logging.toml` — logging profiles tailored for the RC testnet.
- `seed-nodes.txt` — list of bootstrap peers; replace with the published hosts
  before going live.
- `docker-compose.testnet.yml` — minimal compose file to build and run a node
  against the RC testnet.

## Quick usage
```bash
# Copy the samples and adjust hostnames/paths as needed
cp testnet/testnet-config/node-config.toml ~/ippan-testnet/node-config.toml
cp testnet/testnet-config/logging.toml ~/ippan-testnet/logging.toml
cp testnet/testnet-config/docker-compose.testnet.yml ~/ippan-testnet/docker-compose.yml
```
