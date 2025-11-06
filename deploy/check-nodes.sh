#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   HOST="188.245.97.41" UI_URL="http://188.245.97.41:3001" API_BASE="http://188.245.97.41:8080" ./deploy/check-nodes.sh
# or pass via env; defaults try common ports.

HOST="${HOST:-}"

if [[ -z "$HOST" ]]; then
  echo "HOST env is required" >&2
  exit 2
fi

default_api_base="http://$HOST:8080"
API_BASE="${API_BASE:-$default_api_base}"
UI_URL="${UI_URL:-http://$HOST:3001}"
LB_HEALTH="${LB_HEALTH:-http://$HOST:3000/lb-health}"
HTTP_HEALTH="${HTTP_HEALTH:-$API_BASE/health}"
HTTP_STATUS="${HTTP_STATUS:-$API_BASE/status}"
HTTP_PEERS="${HTTP_PEERS:-$API_BASE/peers}"
STATUS_FALLBACK="${STATUS_FALLBACK:-$API_BASE/version}"
PEERS_FALLBACK="${PEERS_FALLBACK:-$API_BASE/p2p/peers}"
P2P_PORTS="${P2P_PORTS:-4001,7000,7080,8080,8081,3000}"
SYSTEMD_SVC="${SYSTEMD_SVC:-ippan-node}"
DOCKER_COMPOSE_DIR="${DOCKER_COMPOSE_DIR:-/opt/ippan}"

# Helper
json_escape() {
  JSON_VALUE="$1" python3 - <<'PY'
import json
import os

print(json.dumps(os.environ.get("JSON_VALUE", "")))
PY
}

check_cmd() {
  local cmd="$1"
  if eval "$cmd" >/dev/null 2>&1; then echo "ok"; else echo "fail"; fi
}

echo "== Checking host $HOST =="

# 1) Basic reachability (from the host itself)
IP_ADDR="$(hostname -I 2>/dev/null | awk '{print $1}')"
DATE_NOW="$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || true)"

# 2) Service presence (systemd or docker)
SYSTEMD_PRESENT="$(check_cmd "systemctl status $SYSTEMD_SVC")"
DOCKER_PRESENT="$(check_cmd "docker ps")"

if [[ "$SYSTEMD_PRESENT" == "ok" ]]; then
  SYSTEMD_ACTIVE="$(systemctl is-active "$SYSTEMD_SVC" 2>/dev/null || true)"
else
  SYSTEMD_ACTIVE="n/a"
fi

if [[ "$DOCKER_PRESENT" == "ok" ]]; then
  # If using docker compose
  if [[ -d "$DOCKER_COMPOSE_DIR" ]]; then
    pushd "$DOCKER_COMPOSE_DIR" >/dev/null || true
    DOCKER_PS="$(docker compose ps --status running 2>/dev/null || true)"
    popd >/dev/null || true
  else
    DOCKER_PS="$(docker ps --format '{{.Names}} {{.Ports}}' 2>/dev/null || true)"
  fi
else
  DOCKER_PS="docker-not-installed"
fi

# 3) Ports listening
declare -a OPEN_PORTS=()
IFS=',' read -r -a PORTS <<< "$P2P_PORTS"
for p in "${PORTS[@]}"; do
  if ss -ltn "( sport = :$p )" 2>/dev/null | grep -q ":$p"; then
    OPEN_PORTS+=("$p")
  fi
done

open_ports_json="[]"
if ((${#OPEN_PORTS[@]})); then
  open_ports_json="[${OPEN_PORTS[0]}"
  for port in "${OPEN_PORTS[@]:1}"; do
    open_ports_json+=",$port"
  done
  open_ports_json+="]"
fi

# 4) HTTP/LB/Health checks
http_code() {
  curl -sS -o /dev/null -w "%{http_code}" "$1" || echo "000"
}

select_endpoint() {
  local primary="$1"
  local fallback="${2:-}"
  local primary_code
  primary_code="$(http_code "$primary")"

  if [[ "$primary_code" == "200" ]] || [[ -z "$fallback" ]] || [[ "$fallback" == "$primary" ]]; then
    printf '%s\n%s\nprimary\n' "$primary" "$primary_code"
    return
  fi

  local fallback_code
  fallback_code="$(http_code "$fallback")"

  if [[ "$fallback_code" == "200" ]]; then
    printf '%s\n%s\nfallback\n' "$fallback" "$fallback_code"
  else
    printf '%s\n%s\nprimary\n' "$primary" "$primary_code"
  fi
}

hc_api="$(http_code "$HTTP_HEALTH")"

readarray -t status_sel < <(select_endpoint "$HTTP_STATUS" "$STATUS_FALLBACK")
status_url="${status_sel[0]:-$HTTP_STATUS}"
hc_status="${status_sel[1]:-000}"
status_source="${status_sel[2]:-primary}"

readarray -t peers_sel < <(select_endpoint "$HTTP_PEERS" "$PEERS_FALLBACK")
peers_url="${peers_sel[0]:-$HTTP_PEERS}"
hc_peers="${peers_sel[1]:-000}"
peers_source="${peers_sel[2]:-primary}"

hc_lb="$(http_code "$LB_HEALTH")"

# 5) Fetch details (don’t fail pipeline if endpoints absent)
get_json() {
  curl -sS --max-time 5 "$1" || echo "{}"
}
status_json="$(get_json "$status_url")"
peers_json="$(get_json "$peers_url")"

extract_version() {
  STATUS_JSON="$1" python3 - <<'PY'
import json
import os

raw = os.environ.get("STATUS_JSON", "{}")
try:
    data = json.loads(raw)
except Exception:
    print("unknown")
else:
    print(data.get("version") or data.get("build") or "unknown")
PY
}

extract_peer_count() {
  PEERS_JSON="$1" python3 - <<'PY'
import json
import os

raw = os.environ.get("PEERS_JSON", "{}")
try:
    data = json.loads(raw)
except Exception:
    print(0)
else:
    peers = data.get("peers")
    try:
        print(len(peers))
    except TypeError:
        print(0)
PY
}

# 6) Derive simple metrics
version="$(extract_version "$status_json")"
peer_count="$(extract_peer_count "$peers_json")"

if ! [[ "$peer_count" =~ ^[0-9]+$ ]]; then
  peer_count=0
fi

# 7) Optional UI check
if [[ -n "$UI_URL" ]]; then
  ui_code="$(http_code "$UI_URL")"
else
  ui_code="(skipped)"
fi

# 8) Output summary JSON
summary="$(cat <<EOF
{
  "timestamp": $(json_escape "$DATE_NOW"),
  "host": $(json_escape "$HOST"),
  "ip": $(json_escape "${IP_ADDR:-unknown}"),
  "systemd_present": $(json_escape "$SYSTEMD_PRESENT"),
  "systemd_active": $(json_escape "$SYSTEMD_ACTIVE"),
  "docker_present": $(json_escape "$DOCKER_PRESENT"),
  "docker_ps": $(json_escape "$DOCKER_PS"),
  "open_ports": $open_ports_json,
  "endpoints": {
    "health_url": $(json_escape "$HTTP_HEALTH"),
    "health_code": $(json_escape "$hc_api"),
    "status_url": $(json_escape "$status_url"),
    "status_source": $(json_escape "$status_source"),
    "status_code": $(json_escape "$hc_status"),
    "peers_url": $(json_escape "$peers_url"),
    "peers_source": $(json_escape "$peers_source"),
    "peers_code": $(json_escape "$hc_peers"),
    "lb_url": $(json_escape "$LB_HEALTH"),
    "lb_code": $(json_escape "$hc_lb"),
    "ui_url": $(json_escape "$UI_URL"),
    "ui_code": $(json_escape "$ui_code")
  },
  "version": $(json_escape "$version"),
  "peer_count": $peer_count,
  "status_sample": $status_json,
  "peers_sample": $peers_json
}
EOF
)"
echo "$summary"

# 9) Fail if critical checks fail
fail=0
[[ "$hc_api" == "200" ]] || fail=1
[[ "$hc_status" == "200" ]] || fail=1
[[ "$hc_peers" == "200" ]] || fail=1
[[ "$peer_count" =~ ^[1-9][0-9]*$ ]] || fail=1

exit $fail
