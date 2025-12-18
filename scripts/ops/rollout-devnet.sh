#!/usr/bin/env bash
set -euo pipefail

# Canary-first devnet rollout for ippan-node + HTTP-only health verification.
#
# Requirements (laptop / WSL):
# - bash, ssh, curl
# - jq OR python3 (JSON parsing)
#
# What it does:
# - upgrades canary first (node3), fail-fast if canary fails verification
# - upgrades remaining nodes
# - verifies: /status ok, build_sha consistent, peer_count=4, /time monotonic,
#   dataset_export.enabled=true and last_age_seconds <= 8h

RPC_PORT="${RPC_PORT:-8080}"
EXPECTED_PEERS="${EXPECTED_PEERS:-4}"
MAX_DATASET_AGE_SECONDS="${MAX_DATASET_AGE_SECONDS:-28800}" # 8h
TIME_SAMPLES="${TIME_SAMPLES:-10}"

CANARY_IP="5.223.51.238"
OTHER_IPS=("188.245.97.41" "135.181.145.174" "178.156.219.107")

need_cmd() { command -v "$1" >/dev/null 2>&1; }

VERIFY_ONLY=0
DRY_RUN=0
APPLY_EXPORTER_UNIT=1

usage() {
  cat <<'EOF'
Usage: scripts/ops/rollout-devnet.sh [--verify-only] [--dry-run] [--apply-exporter-unit=0|1]

Options:
  --verify-only            Skip SSH deploy; only run HTTP verification + build_sha parity checks
  --dry-run                Print the SSH commands that would run, do not execute them (exits 0)
  --apply-exporter-unit=0  Do not rewrite exporter systemd unit on nodes
  --apply-exporter-unit=1  Rewrite exporter systemd unit on nodes (default)
  -h, --help               Show this help

Env:
  RPC_PORT (default: 8080)
  EXPECTED_PEERS (default: 4)
  MAX_DATASET_AGE_SECONDS (default: 28800)
  TIME_SAMPLES (default: 10)
EOF
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --verify-only) VERIFY_ONLY=1; shift ;;
      --dry-run) DRY_RUN=1; shift ;;
      --apply-exporter-unit=0) APPLY_EXPORTER_UNIT=0; shift ;;
      --apply-exporter-unit=1) APPLY_EXPORTER_UNIT=1; shift ;;
      -h|--help) usage; exit 0 ;;
      *) echo "ERROR: unknown arg: $1" >&2; usage >&2; exit 2 ;;
    esac
  done
}

run_cmd() {
  # Usage: run_cmd <desc> <command...>
  local desc="$1"; shift
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] ${desc}: $*"
    return 0
  fi
  "$@"
}

json_get_path() {
  # Usage: json_get_path '<json>' 'status' OR 'dataset_export.enabled'
  local json="$1"
  local path="$2"

  if need_cmd jq; then
    # Convert dot-path to jq access
    jq -r ".$path" <<<"$json"
    return 0
  fi

  if need_cmd python3; then
    python3 - "$path" <<'PY'
import json,sys
path=sys.argv[1]
data=json.load(sys.stdin)
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

  echo "ERROR: need jq or python3 for JSON parsing" >&2
  exit 127
}

http_get() {
  local url="$1"
  curl -fsS --connect-timeout 3 --max-time 8 --retry 3 --retry-delay 1 --retry-all-errors "$url"
}

verify_time_monotonic() {
  local ip="$1"
  local prev=""
  local i t json
  for ((i=1; i<=TIME_SAMPLES; i++)); do
    json="$(http_get "http://${ip}:${RPC_PORT}/time")"
    t="$(json_get_path "$json" "time_us")"
    if [[ -z "$t" ]] || ! [[ "$t" =~ ^[0-9]+$ ]]; then
      echo "FAIL: node=${ip} time_us not integer (got: ${t:-<empty>})" >&2
      return 1
    fi
    if [[ -n "$prev" ]] && [[ "$t" -lt "$prev" ]]; then
      echo "FAIL: node=${ip} time not monotonic (prev=${prev} now=${t})" >&2
      return 1
    fi
    prev="$t"
    sleep 0.2
  done
  return 0
}

verify_node_http() {
  local ip="$1"
  local status_json status build_sha peer_count ds_enabled ds_age

  status_json="$(http_get "http://${ip}:${RPC_PORT}/status")"
  status="$(json_get_path "$status_json" "status")"
  build_sha="$(json_get_path "$status_json" "build_sha")"
  peer_count="$(json_get_path "$status_json" "peer_count")"
  ds_enabled="$(json_get_path "$status_json" "dataset_export.enabled")"
  ds_age="$(json_get_path "$status_json" "dataset_export.last_age_seconds")"

  if [[ "$status" != "ok" ]]; then
    echo "FAIL: node=${ip} /status.status != ok (got: ${status})" >&2
    echo "status_json=${status_json}" >&2
    return 1
  fi
  if [[ "$peer_count" != "$EXPECTED_PEERS" ]]; then
    echo "FAIL: node=${ip} /status.peer_count != ${EXPECTED_PEERS} (got: ${peer_count})" >&2
    echo "status_json=${status_json}" >&2
    return 1
  fi
  if [[ "$ds_enabled" != "true" ]]; then
    echo "FAIL: node=${ip} dataset_export.enabled != true (got: ${ds_enabled:-<empty>})" >&2
    echo "status_json=${status_json}" >&2
    return 1
  fi
  if [[ -z "$ds_age" ]] || ! [[ "$ds_age" =~ ^[0-9]+$ ]]; then
    echo "FAIL: node=${ip} dataset_export.last_age_seconds not integer (got: ${ds_age:-<empty>})" >&2
    echo "status_json=${status_json}" >&2
    return 1
  fi
  if (( ds_age > MAX_DATASET_AGE_SECONDS )); then
    echo "FAIL: node=${ip} dataset_export is stale: age_seconds=${ds_age} > ${MAX_DATASET_AGE_SECONDS}" >&2
    echo "status_json=${status_json}" >&2
    return 1
  fi

  verify_time_monotonic "$ip"

  echo "OK: node=${ip} build_sha=${build_sha} peer_count=${peer_count} dataset_age_seconds=${ds_age}"
  printf '%s\n' "$build_sha"
}

deploy_node() {
  local ip="$1"
  echo "=== DEPLOY ${ip} ==="

  if [[ "$APPLY_EXPORTER_UNIT" -eq 1 ]]; then
    # Harden exporter unit on node (template lives in repo; apply directly here).
    run_cmd "apply exporter unit ($ip)" ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "root@${ip}" "set -euo pipefail
cat >/etc/systemd/system/ippan-export-dataset.service <<'EOF'
[Unit]
Description=IPPAN Devnet Dataset Export (D-GBDT telemetry)
After=network-online.target ippan-node.service
Wants=network-online.target

[Service]
Type=oneshot
User=root
Nice=10
IOSchedulingClass=best-effort
IOSchedulingPriority=7
TimeoutStartSec=900
WorkingDirectory=/root/IPPAN
StandardOutput=journal
StandardError=journal
ExecStart=/usr/local/lib/ippan/export-dataset.sh
EOF
systemctl daemon-reload
"
  fi

  # Update and build as ippan-devnet (avoids git ownership ambiguity).
  run_cmd "update+build ($ip)" ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "root@${ip}" "set -euo pipefail
sudo -u ippan-devnet -H bash -lc 'set -euo pipefail
  git config --global --add safe.directory /opt/ippan || true
  cd /opt/ippan
  git fetch origin
  git checkout master
  git pull --rebase origin master
  cargo build -p ippan-node --release
'
" 

  # Stop → install (avoid \"text file busy\") → start
  run_cmd "install+restart ($ip)" ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "root@${ip}" "set -euo pipefail
TS=\$(date -u +%Y%m%dT%H%M%SZ)
cp -a /usr/local/bin/ippan-node /usr/local/bin/ippan-node.bak.\${TS} || true
systemctl stop ippan-node
sleep 1
install -m 0755 /opt/ippan/target/release/ippan-node /usr/local/bin/ippan-node
systemctl start ippan-node
systemctl status ippan-node --no-pager | head -n 8
"
}

main() {
  parse_args "$@"

  if ! need_cmd curl || ! need_cmd ssh; then
    echo "ERROR: missing curl and/or ssh" >&2
    exit 127
  fi
  if [[ "$VERIFY_ONLY" -eq 1 ]] && [[ "$DRY_RUN" -eq 1 ]]; then
    echo "ERROR: cannot combine --verify-only with --dry-run" >&2
    exit 2
  fi
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "NOTE: --dry-run set; no changes will be made and no HTTP checks will be run."
    echo "OK: dry-run complete"
    exit 0
  fi

  echo "=== CANARY FIRST: ${CANARY_IP} ==="
  if [[ "$VERIFY_ONLY" -ne 1 ]]; then
    deploy_node "${CANARY_IP}"
  fi
  canary_sha="$(verify_node_http "${CANARY_IP}")" || {
    echo "CANARY FAILED: aborting rollout." >&2
    exit 1
  }

  echo "=== REMAINING NODES ==="
  for ip in "${OTHER_IPS[@]}"; do
    if [[ "$VERIFY_ONLY" -ne 1 ]]; then
      deploy_node "$ip"
    fi
    node_sha="$(verify_node_http "$ip")"
    if [[ "$node_sha" != "$canary_sha" ]]; then
      echo "FAIL: build_sha drift (canary=${canary_sha} node=${ip} sha=${node_sha})" >&2
      exit 2
    fi
  done

  echo "=== FINAL VERIFY: all nodes build_sha consistent ==="
  for ip in "${CANARY_IP}" "${OTHER_IPS[@]}"; do
    node_sha="$(verify_node_http "$ip")"
    if [[ "$node_sha" != "$canary_sha" ]]; then
      echo "FAIL: build_sha drift on final verify (canary=${canary_sha} node=${ip} sha=${node_sha})" >&2
      exit 2
    fi
  done

  echo "OK: rollout complete (build_sha=${canary_sha})"
}

main "$@"


