#!/usr/bin/env bash
set -euo pipefail

# Deploy a locally built ippan-node binary to a remote devnet node and restart it.
# Verifies locally on the node via 127.0.0.1 (never trust public RPC during rollout).
#
# Usage:
#   ./scripts/ops/deploy_node_binary.sh target/release/ippan-node 188.245.97.41
#
# Optional env:
#   REMOTE_BIN=/usr/local/bin/ippan-node

BIN_LOCAL="${1:-target/release/ippan-node}"
NODE_IP="${2:-}"
REMOTE_BIN="${REMOTE_BIN:-/usr/local/bin/ippan-node}"
SSH_OPTS="${SSH_OPTS:--o BatchMode=yes -o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new}"
SCP_OPTS="${SCP_OPTS:--o BatchMode=yes -o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new}"

if [[ -z "$NODE_IP" ]]; then
  echo "Usage: $0 <local_bin_path> <node_ip>" >&2
  exit 1
fi

if [[ ! -f "$BIN_LOCAL" ]]; then
  echo "Missing local binary: $BIN_LOCAL" >&2
  exit 1
fi

echo "== Deploying to $NODE_IP =="
scp $SCP_OPTS "$BIN_LOCAL" "root@$NODE_IP:${REMOTE_BIN}.new"

ssh $SSH_OPTS "root@$NODE_IP" "bash -lc '
set -u
mv ${REMOTE_BIN}.new ${REMOTE_BIN}
chmod +x ${REMOTE_BIN}

if systemctl list-units --type=service | grep -q \"ippan-node\"; then
  systemctl restart ippan-node
  systemctl is-active --quiet ippan-node && echo \"systemd: ippan-node active\"
else
  pkill -f \"${REMOTE_BIN}\" || true
  nohup ${REMOTE_BIN} >/var/log/ippan-node.log 2>&1 &
  echo \"nohup: started\"
fi

sleep 8
echo \"--- local listen (ss/netstat, key ports) ---\"
if command -v ss >/dev/null 2>&1; then
  ss -lntp 2>/dev/null | grep -E \":(8080|18080|28080|38080|3000|3001)\\b\" || true
else
  netstat -lntp 2>/dev/null | grep -E \":(8080|18080|28080|38080|3000|3001)\\b\" || true
fi
echo \"--- local /status (try 8080 then 18080) ---\"
curl -sS --max-time 2 http://127.0.0.1:8080/status | head -c 1200 || true; echo
curl -sS --max-time 2 http://127.0.0.1:18080/status | head -c 1200 || true; echo
echo \"--- local /consensus/view (try 8080 then 18080) ---\"
curl -sS --max-time 2 http://127.0.0.1:8080/consensus/view | head -c 1200 || true; echo
curl -sS --max-time 2 http://127.0.0.1:18080/consensus/view | head -c 1200 || true; echo
echo \"--- systemd status tail ---\"
systemctl status ippan-node --no-pager -l | tail -n 60 || true
echo \"--- journal tail ---\"
journalctl -u ippan-node --no-pager -n 200 || true
'"

echo "== Done $NODE_IP =="


