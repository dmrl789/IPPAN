---
title: Devnet rollback + drift SOP
---

## Scope
This runbook is for the devnet fleet (4 nodes) and assumes:
- Binary path: `/usr/local/bin/ippan-node`
- Canonical build repo: `/opt/ippan`
- RPC: `http://127.0.0.1:8080`
- Dataset exports: `ippan-export-dataset.timer` → `/var/lib/ippan/ai_datasets/devnet_dataset_*.csv.gz`

## Rollback `ippan-node` (per node)
1) Stop:
   sudo systemctl stop ippan-node

2) Restore a backup:
   ls -1t /usr/local/bin/ippan-node.bak.* | head -n 5
   sudo cp -a /usr/local/bin/ippan-node.bak.<TIMESTAMP> /usr/local/bin/ippan-node
   sudo chmod +x /usr/local/bin/ippan-node

3) Start + verify:
   sudo systemctl start ippan-node
   sudo systemctl status ippan-node --no-pager

   curl -fsS http://127.0.0.1:8080/status | jq '{status,build_sha,peer_count,dataset_export}'
   curl -fsS http://127.0.0.1:8080/peers | jq 'length'
   curl -fsS http://127.0.0.1:8080/time  | jq -r '.time_us'

## Drift response (HTTP-only)
Symptoms:
- CI devnet HTTP health reports multiple `build_sha` values across nodes.

Response:
1) Pick canary node and decide target `build_sha` (usually canary’s).
2) Rebuild + redeploy that target commit to the other nodes (canary-first if changing again).
3) Confirm:
   - `/status.build_sha` matches on all nodes
   - `/peers` length is 4

## Dataset freshness failure
Symptoms:
- `/status.dataset_export.enabled=false` or `last_age_seconds` missing
- or CI reports `dataset_export` stale (age > 8h)

Checklist (per node):
1) Confirm marker + timer enabled:
   sudo ls -l /etc/ippan/markers/dataset_export_enabled
   sudo systemctl status ippan-export-dataset.timer --no-pager

2) Run one export:
   sudo systemctl start ippan-export-dataset.service
   sudo journalctl -u ippan-export-dataset.service -n 120 --no-pager
   ls -1t /var/lib/ippan/ai_datasets/devnet_dataset_*.csv.gz | head -n 3

3) Re-check status:
   curl -fsS http://127.0.0.1:8080/status | jq '.dataset_export'


