#!/usr/bin/env bash
set -euo pipefail

# Devnet triage helper: collects the key evidence needed to debug failures.
#
# Requirements:
# - ssh, curl
# - jq OR python3 (JSON parsing)

RPC_PORT="${RPC_PORT:-8080}"

need_cmd() { command -v "$1" >/dev/null 2>&1; }

http_get() {
  local url="$1"
  curl -fsS --connect-timeout 3 --max-time 8 --retry 2 --retry-delay 1 --retry-all-errors "$url" || true
}

json_get_path() {
  local json="$1"
  local path="$2"

  if need_cmd jq; then
    jq -r ".$path" <<<"$json" 2>/dev/null || echo ""
    return 0
  fi

  if need_cmd python3; then
    python3 - "$path" <<'PY'
import json,sys
path=sys.argv[1]
try:
    data=json.load(sys.stdin)
except Exception:
    print("")
    raise SystemExit(0)
cur=data
for k in path.split("."):
    if isinstance(cur, dict) and k in cur:
        cur=cur[k]
    else:
        cur=None
        break
if cur is None:
    print("")
elif isinstance(cur, bool):
    print("true" if cur else "false")
else:
    print(cur)
PY
    return 0
  fi

  echo ""
}

NODES=(
  "5.223.51.238"
  "188.245.97.41"
  "135.181.145.174"
  "178.156.219.107"
)

if ! need_cmd ssh || ! need_cmd curl; then
  echo "ERROR: need ssh and curl" >&2
  exit 127
fi

for ip in "${NODES[@]}"; do
  echo
  echo "================================================================================"
  echo "NODE ${ip}"
  echo "================================================================================"

  echo "--- systemctl status ippan-node ---"
  ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "root@${ip}" \
    "systemctl status ippan-node --no-pager || true"

  echo "--- journalctl -u ippan-node (last 120) ---"
  ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "root@${ip}" \
    "journalctl -u ippan-node -n 120 --no-pager || true"

  echo "--- /status summary ---"
  status_json="$(http_get "http://${ip}:${RPC_PORT}/status")"
  if [[ -n "$status_json" ]]; then
    status="$(json_get_path "$status_json" "status")"
    build_sha="$(json_get_path "$status_json" "build_sha")"
    peer_count="$(json_get_path "$status_json" "peer_count")"
    ds_enabled="$(json_get_path "$status_json" "dataset_export.enabled")"
    ds_age="$(json_get_path "$status_json" "dataset_export.last_age_seconds")"
    ds_last="$(json_get_path "$status_json" "dataset_export.last_ts_utc")"
    echo "status=${status} build_sha=${build_sha} peer_count=${peer_count} dataset_enabled=${ds_enabled} dataset_age_seconds=${ds_age} dataset_last_ts_utc=${ds_last}"
  else
    echo "FAIL: could not fetch /status"
  fi

  echo "--- exporter timer/service status ---"
  ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "root@${ip}" \
    "systemctl status ippan-export-dataset.timer --no-pager || true; echo; systemctl status ippan-export-dataset.service --no-pager || true"

  echo "--- exporter service logs (last 80) ---"
  ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "root@${ip}" \
    "journalctl -u ippan-export-dataset.service -n 80 --no-pager || true"

  echo "--- newest dataset file ---"
  ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "root@${ip}" \
    "set -euo pipefail
f=\$(ls -1t /var/lib/ippan/ai_datasets/devnet_dataset_*.csv.gz 2>/dev/null | head -n 1 || true)
echo \"newest=\${f:-<none>}\"
if [ -n \"\${f}\" ]; then
  stat -c '%y %s %n' \"\${f}\" || true
fi
"
done


