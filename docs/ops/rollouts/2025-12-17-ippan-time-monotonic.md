---
# Devnet Rollout — ippan-time monotonic fix

Date: 2025-12-17  
Master head: 34d415a0  
RPC port: 8080  
P2P port: 9000  

## Nodes
- Node 1: 188.245.97.41
- Node 2: 135.181.145.174
- Node 3 (canary): 5.223.51.238
- Node 4: 178.156.219.107

## Verification
- /status: "ok" on all nodes (127.0.0.1:8080)
- /peers: count=4 on all nodes
- /time: monotonic samples verified on all nodes
- Binary sha256 (/usr/local/bin/ippan-node): 65083b7c…0f8c3 (same on all nodes)

## Notes
- Any mismatch should be resolved by redeploying the canary artifact to the drifted node.

## When CI alerts (devnet-http-health.yml fails)

### Symptom
- CI reports one of:
  - `/status` not reachable or `.status != "ok"`
  - `/peers` length not equal to 4
  - `/time` missing or `.time_us` not an integer

### First response (operator laptop)
- Run:
  - `scripts/ops/check-devnet.ps1` (Windows PowerShell)
  - `scripts/ops/check-devnet.sh` (Linux/macOS/WSL)

### If a single node is down
- SSH the node:
  - `systemctl status ippan-node --no-pager`
  - `journalctl -u ippan-node -n 200 --no-pager`

### If RPC is unreachable but service is up
- Confirm ports from config + reality:
  - `/etc/ippan/config/node.toml` (`[rpc] port` vs `[p2p] port`)
  - `ss -lntp | grep -E ":(8080|9000)\\b"`

### If binary drift is detected
- Redeploy the canary artifact to the drifted node (recommended), or rebuild from a pinned git commit.
---


