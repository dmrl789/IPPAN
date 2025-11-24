# Disaster Recovery Runbook

IPPAN nodes can be rebuilt from deterministic snapshots. Follow this runbook whenever you need to back up state, restore a failed validator, or migrate hardware.

## Backup cadence

- **Validator nodes:** export a snapshot at least every hour during normal operations and immediately before scheduled maintenance. Keep the last 24 hours on local SSD plus copy dailies to durable storage (S3, GCS, rsync target).
- **Gateway / RPC nodes:** reuse validator snapshots if possible; otherwise back up once per day because gateways primarily cache data.
- Store snapshot directories under `/var/backups/ippan/<date>/` and replicate to an off-site bucket encrypted at rest.

## Export procedure

1. Stop the running node (Ctrl+C or systemd `stop`). The `snapshot` command refuses to run if the runtime lock file `data/.ippan.lock` is held, so stopping the node is mandatory.
2. Export the snapshot:

   ```bash
   ippan-node \
     --config /etc/ippan/devnet.toml \
     snapshot export \
     --dir /var/backups/ippan-$(date +%Y%m%dT%H%M%SZ) \
     --height $(cat /var/lib/ippan/db/height)
   ```

   - `--height` makes sure you capture the exact round you expect. The command fails if the on-disk height differs.
   - Use `--force` only when reusing an existing output directory; it will delete the directory first.
3. Copy the resulting directory to your backup target (e.g., `aws s3 cp --recursive`).

## Restore procedure

1. Provision a clean server and install the IPPAN binaries plus configuration.
2. Stop any running node service.
3. Copy the desired snapshot directory onto the node (e.g., `/var/backups/ippan-2025-11-24T10Z`).
4. Remove the existing database if it exists (or pass `--force`):

   ```bash
   ippan-node \
     --config /etc/ippan/devnet.toml \
     snapshot import \
     --dir /var/backups/ippan-2025-11-24T10Z \
     --force
   ```

   The importer wipes `data/db` when `--force` is provided and refuses to continue if the snapshot network ID differs from the node’s network profile.
5. Start the node normally (systemd service or direct command).

## Verification checklist

- `ippan-node status` reports the expected height/round from the manifest.
- Tip block hash (`manifest.tip_block_hash`) matches `GET /blocks/<height>` via RPC.
- Balances for critical accounts match pre-failure values.
- Node catches up with live peers within a few minutes (monitor `logs` or Prometheus metrics).
- Remove the snapshot after verification if it contains sensitive data that shouldn’t remain on the host.

## Failure modes & mitigations

| Issue | Symptoms | Mitigation |
| --- | --- | --- |
| **Lock file present** | `snapshot export`/`import` errors with “data directory is locked” | Ensure the node is stopped. If the previous process crashed, delete `data/.ippan.lock` manually and rerun. |
| **Height mismatch** | Export command returns `requested height ... does not match storage tip` | Wait for the node to finish syncing, or adjust `--height` to the current tip after confirming it is safe. |
| **Network mismatch** | Import aborts before writing data | Double-check `IPPAN_NETWORK_ID`/config. Never import a devnet snapshot into testnet or mainnet. |
| **Non-empty database on import** | Error “storage not empty” | Stop the node and re-run with `--force`, which removes the old `db` directory. |
| **Corrupted snapshot** | `validate_against_storage` fails or hashes differ | Re-export from another validator or the most recent healthy backup. |

Keep this runbook alongside your infrastructure documentation. Snapshots plus the built-in manifest validation allow clean migrations and recovery drills without hand-crafted scripts.
