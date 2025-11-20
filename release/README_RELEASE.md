# IPPAN Release Artifacts

This directory contains the files operators need to provision a hardened IPPAN node:

- `config-template.toml` — safe defaults for RPC, P2P, DHT, consensus, storage, and security.
- `ippan-node.service` — a reference systemd service definition for Linux hosts.

## Build a production binary

```
cargo build --release --bin ippan-node --features production
```

The resulting binary lives at `target/release/ippan-node` and enables the optimized
release profile (`opt-level = "z"`, LTO, panic = abort, stripped symbols).

## Install on a server

1. Create a dedicated runtime user and directories:
   ```bash
   sudo useradd --system --home /var/lib/ippan --shell /usr/sbin/nologin ippan
   sudo mkdir -p /var/lib/ippan /etc/ippan
   sudo chown -R ippan:ippan /var/lib/ippan
   ```
2. Copy the binary and service assets:
   ```bash
   sudo install -m 0755 target/release/ippan-node /usr/local/bin/ippan-node
   sudo install -m 0644 release/ippan-node.service /etc/systemd/system/ippan-node.service
   ```
3. Place your configuration at `/etc/ippan/config.toml` (see below).

## Configure the node

1. Copy `release/config-template.toml` to `/etc/ippan/config.toml`.
2. Review each section:
   - `[rpc]` — keep `bind = "127.0.0.1"` unless a reverse proxy protects the node.
   - `[p2p]` — set `bootstrap_nodes` and `bind` to match your firewall rules.
   - `[dht]` — select `mode = "libp2p"` for production; keep `handle_mode = "stub"` until handle relays are live.
   - `[consensus]` — DLC is the default; PoA remains only as a legacy/dev fallback.
   - `[storage]` — ensure `data_dir` and `db_path` point to a persistent disk.
   - `[security]` — leave `enable = true` to enforce RPC security policies.
3. Run `ippan-node --check --config /etc/ippan/config.toml` to validate ports, directories, and settings.

## Manage the systemd service

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now ippan-node

# Check logs
journalctl -fu ippan-node

# Stop the service
sudo systemctl stop ippan-node
```

The service definition runs the node as the `ippan` user, stores data in `/var/lib/ippan`,
and loads `/etc/ippan/config.toml` at startup.
