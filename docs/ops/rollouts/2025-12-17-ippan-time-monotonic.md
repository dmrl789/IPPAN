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
---


