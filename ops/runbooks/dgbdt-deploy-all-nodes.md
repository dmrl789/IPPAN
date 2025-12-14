# D-GBDT deploy to all devnet nodes (pinned model)

## What this does
- Uses `scripts/deploy-dgbdt-all-nodes.ps1` to SSH each node listed in `ops/nodes/devnet_nodes.txt`
- Optionally git-pulls a branch on each node
- Optionally restarts `ippan-node`
- Verifies:
  - `dlc.toml` contains a `model_hash = ...` line (hash-pinned model)
  - `/status` returns `consensus.metrics_available`
  - `/status` returns `peer_count`

## Preconditions
- You can SSH to all nodes as listed in `ops/nodes/devnet_nodes.txt`
- Nodes run a systemd service named `ippan-node`
- RPC is accessible locally on nodes at `http://127.0.0.1:8080/status`

## Run
From repo root (PowerShell):

```powershell
.\scripts\deploy-dgbdt-all-nodes.ps1
```

### Optional flags

* Skip git pull on nodes:

  ```powershell
  .\scripts\deploy-dgbdt-all-nodes.ps1 -SkipGitPull
  ```
* Skip restart:

  ```powershell
  .\scripts\deploy-dgbdt-all-nodes.ps1 -SkipRestart
  ```
* Custom SSH port:

  ```powershell
  .\scripts\deploy-dgbdt-all-nodes.ps1 -SshPort 22
  ```
* Deploy a specific branch:

  ```powershell
  .\scripts\deploy-dgbdt-all-nodes.ps1 -Branch master
  ```

## Troubleshooting (per node)

If the summary table shows `no` / `parse_error` / `error`, run:

```powershell
ssh -p 22 -o StrictHostKeyChecking=accept-new root@NODE "bash -lc 'systemctl status ippan-node -l'"
ssh -p 22 -o StrictHostKeyChecking=accept-new root@NODE "bash -lc 'journalctl -u ippan-node -n 200 --no-pager'"
ssh -p 22 -o StrictHostKeyChecking=accept-new root@NODE "bash -lc 'cat /etc/ippan/config/dlc.toml || cat /etc/ippan/dlc.toml || cat ~/IPPAN/config/dlc.toml'"
```

Then re-run the deploy script (optionally with `-SkipGitPull`).

## Expected output

A summary table with columns:

* `HashSeen` (yes/no)
* `MetricsAvailable` (yes/no)
* `PeerCount` (integer / unknown / parse_error)

