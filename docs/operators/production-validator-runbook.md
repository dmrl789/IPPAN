# Production Validator Runbook

Validated: 2025-11-24 — keep local adaptations under version control alongside this document.

## 1. Baseline requirements

- **Hardware**: 8+ vCPU, 32 GB RAM, NVMe SSD (≥1 TB) with sustained 1 GB/s writes.
- **OS**: Ubuntu 22.04 LTS or newer. Keep `unattended-upgrades` enabled for security fixes.
- **Networking**: Dedicated public IPv4/IPv6, symmetric 1 Gbps bandwidth. Open only the RPC and P2P ports documented below.
- **Time sync**: Enable `systemd-timesyncd` or `chrony`; HashTimer derivation depends on monotonic clocks.

## 2. OS hardening checklist

- Create a dedicated `ippan` user and run the service under that account.
- Patch the host: `sudo apt update && sudo apt upgrade`.
- Disable password SSH logins; rely on keys + MFA.
- Configure the firewall (ufw/nftables):
  - Allow `tcp/8080` (RPC) **only** from trusted ingress (reverse proxy, bastion).
  - Allow `tcp/9000` (P2P) from validator peers.
  - Deny all other inbound traffic.
- Enable automatic reboots after kernel patching if your cluster manager supports draining.

## 3. Node configuration

1. Copy the desired network profile (`config/mainnet.toml`, `config/testnet.toml`, etc.) into `/etc/ippan/ippan.toml`.
2. Set environment overrides via `/etc/ippan/ippan.env` (systemd `EnvironmentFile=`):

   ```env
   IPPAN_NETWORK_ID=ippan-mainnet
   IPPAN_RPC_HOST=127.0.0.1
   IPPAN_RPC_PORT=8080
   IPPAN_RPC_ALLOWED_ORIGINS=https://validator-console.example.com
   IPPAN_P2P_HOST=0.0.0.0
   IPPAN_DATA_DIR=/var/lib/ippan
   IPPAN_LOG_FORMAT=json
   ```

3. Run preflight validation: `ippan-node --config /etc/ippan/ippan.toml --check`.
4. Start via systemd (see `release/ippan.service` template) or `systemctl enable --now ippan`.

## 4. Key management

- Store validator Ed25519 keys under `/var/lib/ippan/keys/` with `600` permissions and the directory owned by `ippan`.
- Keep an encrypted offline backup (age, gpg, or HSM export) stored separately from snapshots.
- Rotate keys quarterly:
  1. Generate a new keypair with `ippan-wallet generate-key --output /tmp/new.key`.
  2. Update `VALIDATOR_ID` + signer secrets in the config.
  3. Restart the node and verify consensus participation metrics.

## 5. Operations runbook

| Task | Procedure |
| --- | --- |
| **Daily checks** | `journalctl -u ippan --since today`, `curl -s localhost:8080/health`, confirm peer count & consensus round advance. |
| **Snapshots** | Follow [Disaster Recovery Runbook](./disaster-recovery.md). Export hourly snapshots to `/var/backups/ippan/<timestamp>` and sync to object storage. |
| **Upgrades** | `ippan-node snapshot export --dir /var/backups/pre-upgrade` → stop service → deploy new binary/package → `systemctl start ippan` → verify `/health` + Prometheus. |
| **Metrics** | Expose Prometheus handle (`/metrics`) via a reverse proxy on localhost; scrape from the monitoring VLAN only. |
| **Logs** | Forward structured logs to Loki/Elastic using `journalbeat` or `vector` with JSON parsing. Retain at least 30 days. |

## 6. Security controls

- Set `RPC_ALLOWED_ORIGINS` to explicit domains; wildcards are ignored outside dev mode.
- Terminate TLS at a reverse proxy (nginx, Caddy) and forward to the loopback RPC.
- Enable the RPC security manager IP allow-list if the deployment is directly exposed.
- Monitor `/metrics` gauges (`node_health`, `consensus_round`, `mempool_size`) and alert on deviations.

## 7. Emergency actions

- **Node crash**: stop the service, restore the latest snapshot (`ippan-node snapshot import --force`), restart, and watch `/health`.
- **Validator key compromise**: stop the node, revoke validator credentials out-of-band (governance rotation), deploy a fresh keypair, and notify counterparties.
- **Network isolation**: close the P2P port at the firewall to prevent gossip while investigating, then re-open after remediation.

Keep this runbook in sync with your automation (Ansible, Terraform, etc.) and record every deviation in the change log for auditability.
