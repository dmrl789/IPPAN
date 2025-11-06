#!/usr/bin/env bash
#
# Launch the IPPAN full-stack docker-compose environment, wait for the
# multi-node network to become healthy, validate node discovery, HashTimer
# synchronisation, and API/explorer responsiveness, and capture artifacts for
# deterministic replay.

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
REPO_ROOT=$(cd "${SCRIPT_DIR}/../.." && pwd -P)
cd "$REPO_ROOT"

COMPOSE_FILE=${COMPOSE_FILE:-deploy/docker-compose.full-stack.yml}
PROJECT_NAME=${PROJECT_NAME:-ippan-fullstack-ci}
ARTIFACTS_DIR=${ARTIFACTS_DIR:-artifacts/full-stack}
WAIT_TIMEOUT=${WAIT_TIMEOUT:-240}
MIN_PEER_COUNT=${MIN_PEER_COUNT:-2}
HASH_TIMER_MAX_SKEW_US=${HASH_TIMER_MAX_SKEW_US:-500000}

LOG_DIR="$ARTIFACTS_DIR/logs"
METRICS_DIR="$ARTIFACTS_DIR/metrics"
REPORT_DIR="$ARTIFACTS_DIR/reports"

mkdir -p "$LOG_DIR" "$METRICS_DIR" "$REPORT_DIR"

if [[ -n "${NODE_ENDPOINTS_OVERRIDE:-}" ]]; then
  IFS=',' read -r -a NODE_ENDPOINTS <<< "${NODE_ENDPOINTS_OVERRIDE// /,}"
else
  NODE_ENDPOINTS=("http://127.0.0.1:8080" "http://127.0.0.1:8081" "http://127.0.0.1:8082")
fi

if [[ -n "${NODE_NAMES_OVERRIDE:-}" ]]; then
  IFS=',' read -r -a NODE_NAMES <<< "${NODE_NAMES_OVERRIDE// /,}"
else
  NODE_NAMES=("node-bootstrap" "node-2" "node-3")
fi

GATEWAY_ENDPOINT=${GATEWAY_ENDPOINT_OVERRIDE:-"http://127.0.0.1:7080"}

if [[ ${#NODE_ENDPOINTS[@]} -ne ${#NODE_NAMES[@]} ]]; then
  echo "error: NODE_ENDPOINTS and NODE_NAMES length mismatch" >&2
  exit 1
fi

REPORT_FILE="$REPORT_DIR/full-stack-validation.json"

compose() {
  docker compose -p "$PROJECT_NAME" -f "$COMPOSE_FILE" "$@"
}

collect_logs() {
  local services
  services=$(compose ps --services 2>/dev/null) || return
  compose logs --no-color >"$LOG_DIR/full-stack.log" 2>/dev/null || true
  while IFS= read -r svc; do
    [[ -z "$svc" ]] && continue
    compose logs --no-color "$svc" >"$LOG_DIR/${svc}.log" 2>/dev/null || true
  done <<<"$services"
}

collect_metrics() {
  local idx=0
  for base in "${NODE_ENDPOINTS[@]}"; do
    local name="${NODE_NAMES[$idx]}"
    local safe_name=${name//[^a-zA-Z0-9_-]/-}
    curl --fail -sS --max-time 10 "$base/metrics" >"$METRICS_DIR/${safe_name}.prom" 2>/dev/null || true
    curl --fail -sS --max-time 10 "$base/health" >"$REPORT_DIR/${safe_name}-health.json" 2>/dev/null || true
    curl --fail -sS --max-time 10 "$base/peers" >"$REPORT_DIR/${safe_name}-peers.json" 2>/dev/null || true
    curl --fail -sS --max-time 10 "$base/time" >"$REPORT_DIR/${safe_name}-time.json" 2>/dev/null || true
    ((idx++))
  done

  curl --fail -sS --max-time 10 "$GATEWAY_ENDPOINT/api/health" >"$REPORT_DIR/gateway-api-health.json" 2>/dev/null || true
  curl --fail -sS --max-time 10 "$GATEWAY_ENDPOINT/api/peers" >"$REPORT_DIR/gateway-api-peers.json" 2>/dev/null || true
  curl --fail -sS --max-time 10 "$GATEWAY_ENDPOINT/explorer" >"$REPORT_DIR/gateway-explorer.json" 2>/dev/null || true
  curl --fail -sS --max-time 10 "$GATEWAY_ENDPOINT/explorer/api/peers" >"$REPORT_DIR/gateway-explorer-peers.json" 2>/dev/null || true
}

cleanup() {
  local exit_code=$?
  set +e
  compose ps >"$REPORT_DIR/docker-ps.txt" 2>/dev/null || true
  compose ps --services >"$REPORT_DIR/docker-services.txt" 2>/dev/null || true
  collect_metrics
  collect_logs
  compose down -v --remove-orphans >/dev/null 2>&1 || true
  exit $exit_code
}

trap cleanup EXIT

echo "🏗️  Building IPPAN full-stack images (project: $PROJECT_NAME)"
compose down -v --remove-orphans >/dev/null 2>&1 || true
DOCKER_BUILDKIT=${DOCKER_BUILDKIT:-1} compose build --pull

echo "🚀 Starting multi-node network"
compose up -d

export VALIDATION_WAIT_TIMEOUT="$WAIT_TIMEOUT"
export VALIDATION_MIN_PEERS="$MIN_PEER_COUNT"
export VALIDATION_HASH_TIMER_MAX_SKEW_US="$HASH_TIMER_MAX_SKEW_US"
export VALIDATION_REPORT_FILE="$REPORT_FILE"
export VALIDATION_GATEWAY_BASE="$GATEWAY_ENDPOINT"
export VALIDATION_NODE_BASES="${NODE_ENDPOINTS[*]}"
export VALIDATION_NODE_NAMES="${NODE_NAMES[*]}"

python3 <<'PY'
import json
import os
import sys
import time
import urllib.error
import urllib.request

node_bases = [base.strip() for base in os.environ["VALIDATION_NODE_BASES"].split() if base.strip()]
node_names = [name.strip() for name in os.environ["VALIDATION_NODE_NAMES"].split() if name.strip()]

if len(node_bases) != len(node_names):
    print("Node base/name mismatch", file=sys.stderr)
    sys.exit(2)

gateway_base = os.environ["VALIDATION_GATEWAY_BASE"].rstrip('/')
wait_timeout = int(os.environ.get("VALIDATION_WAIT_TIMEOUT", "240"))
min_peers = int(os.environ.get("VALIDATION_MIN_PEERS", "2"))
max_skew = int(os.environ.get("VALIDATION_HASH_TIMER_MAX_SKEW_US", "500000"))
report_file = os.environ["VALIDATION_REPORT_FILE"]

nodes = [{"name": name, "base": base.rstrip('/')} for name, base in zip(node_names, node_bases)]

def fetch_json(url, timeout=5):
    req = urllib.request.Request(url)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            data = resp.read()
            code = resp.getcode()
    except urllib.error.HTTPError as exc:
        body = exc.read().decode(errors="replace") if hasattr(exc, "read") else ""
        return exc.code, None, f"HTTP {exc.code}: {exc.reason}", body
    except Exception as exc:  # pylint: disable=broad-except
        return None, None, str(exc), ""

    try:
        payload = json.loads(data.decode() or "null")
    except json.JSONDecodeError as exc:
        return code, None, f"JSON decode error: {exc}", data.decode(errors="replace")
    return code, payload, None, None

def wait_for_health(node):
    deadline = time.time() + wait_timeout
    last_error = None
    while time.time() < deadline:
        status, payload, error, _ = fetch_json(f"{node['base']}/health")
        node['health'] = {
            'status_code': status,
            'payload': payload,
            'error': error,
        }
        if status == 200 and isinstance(payload, dict) and str(payload.get('status', '')).lower() == 'healthy':
            return True
        last_error = error or f"HTTP {status}"
        time.sleep(3)
    node['health_timeout'] = True
    node['health_last_error'] = last_error
    return False

def update_peer_info(node):
    status_h, payload_h, error_h, _ = fetch_json(f"{node['base']}/health")
    status_p, payload_p, error_p, _ = fetch_json(f"{node['base']}/peers")
    node['health'] = {
        'status_code': status_h,
        'payload': payload_h,
        'error': error_h,
    }
    node['peers'] = {
        'status_code': status_p,
        'payload': payload_p,
        'error': error_p,
    }

    reported = 0
    if isinstance(payload_h, dict):
        reported = int(payload_h.get('peer_count') or 0)
    listed = 0
    if isinstance(payload_p, dict):
        peers = payload_p.get('peers')
        if isinstance(peers, list):
            listed = len(peers)
    node['peer_count_reported'] = reported
    node['peer_count_listed'] = listed
    node['observed_peer_count'] = max(reported, listed)


all_health_ok = True
for node in nodes:
    ok = wait_for_health(node)
    all_health_ok = all_health_ok and ok

discovery_deadline = time.time() + wait_timeout
while time.time() < discovery_deadline:
    for node in nodes:
        update_peer_info(node)
    if all(
        node['health'].get('status_code') == 200
        and isinstance(node['health'].get('payload'), dict)
        and str(node['health']['payload'].get('status', '')).lower() == 'healthy'
        and node['peers'].get('status_code') == 200
        and node.get('observed_peer_count', 0) >= min_peers
        for node in nodes
    ):
        break
    time.sleep(3)
else:
    for node in nodes:
        update_peer_info(node)

hash_timer_samples = []
for node in nodes:
    status, payload, error, _ = fetch_json(f"{node['base']}/time")
    node['time'] = {
        'status_code': status,
        'payload': payload,
        'error': error,
    }
    if status == 200 and isinstance(payload, dict):
        value = payload.get('time_us')
        if value is None:
            value = payload.get('timestamp')
        if isinstance(value, (int, float)):
            node['time']['value_us'] = int(value)
            hash_timer_samples.append(int(value))
        else:
            node['time']['error'] = node['time'].get('error') or 'missing time_us'

hash_timer_skew = None
if hash_timer_samples:
    hash_timer_skew = max(hash_timer_samples) - min(hash_timer_samples)

gateway = {}
gateway_paths = {
    'api_health': '/api/health',
    'api_peers': '/api/peers',
    'explorer': '/explorer',
    'explorer_peers': '/explorer/api/peers',
}

for key, path in gateway_paths.items():
    status, payload, error, raw = fetch_json(f"{gateway_base}{path}")
    entry = {'status_code': status, 'payload': payload, 'error': error}
    if payload is None and raw:
        entry['raw'] = raw[:2048]
    gateway[key] = entry

summary = {
    'node_health_ok': all(
        node['health'].get('status_code') == 200
        and isinstance(node['health'].get('payload'), dict)
        and str(node['health']['payload'].get('status', '')).lower() == 'healthy'
        for node in nodes
    ),
    'node_discovery_ok': all(node.get('observed_peer_count', 0) >= min_peers for node in nodes),
    'hash_timer_ok': hash_timer_skew is not None and hash_timer_skew <= max_skew,
    'gateway_ok': gateway['api_health'].get('status_code') == 200
    and isinstance(gateway['api_health'].get('payload'), dict)
    and str(gateway['api_health']['payload'].get('status', '')).lower() == 'healthy'
    and gateway['api_peers'].get('status_code') == 200,
    'explorer_ok': gateway['explorer'].get('status_code') == 200,
}

results = {
    'timestamp': time.time(),
    'settings': {
        'min_peer_count': min_peers,
        'hash_timer_max_skew_us': max_skew,
        'wait_timeout_seconds': wait_timeout,
    },
    'summary': summary,
    'hash_timer': {
        'samples_us': hash_timer_samples,
        'skew_us': hash_timer_skew,
        'threshold_us': max_skew,
    },
    'nodes': nodes,
    'gateway': gateway,
}

os.makedirs(os.path.dirname(report_file), exist_ok=True)
with open(report_file, 'w', encoding='utf-8') as fp:
    json.dump(results, fp, indent=2, sort_keys=True)

print("Validation summary:")
for key, value in summary.items():
    print(f"  - {key}: {'✅' if value else '❌'}")
if hash_timer_skew is not None:
    print(f"  - hash_timer_skew_us: {hash_timer_skew} (threshold {max_skew})")

overall_ok = all(summary.values())
sys.exit(0 if overall_ok else 1)
PY
