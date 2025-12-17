#!/usr/bin/env bash
set -euo pipefail

# Devnet verifier: checks /status, /peers count, /time, and ippan-node sha256 across all devnet nodes.
#
# Requirements:
# - curl
# - ssh (keys configured for root@<ip>)
# - jq OR python3 (used for JSON parsing)

RPC_PORT="${RPC_PORT:-8080}"
EXPECTED_PEERS="${EXPECTED_PEERS:-4}"
TIME_SAMPLES="${TIME_SAMPLES:-10}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1
}

json_get() {
  # Usage: json_get '<json>' '<expr>'
  # expr supports: .status .version .peer_count .node_id .time_us length
  local json="$1"
  local expr="$2"

  if need_cmd jq; then
    if [[ "$expr" == "length" ]]; then
      jq -r 'length' <<<"$json"
    else
      jq -r "$expr" <<<"$json"
    fi
    return 0
  fi

  if need_cmd python3; then
    python3 - "$expr" <<'PY'
import json,sys
expr=sys.argv[1]
data=json.load(sys.stdin)
if expr=="length":
    print(len(data))
else:
    key=expr.lstrip(".")
    val=data.get(key, "")
    print("" if val is None else val)
PY
    return 0
  fi

  echo "ERROR: need jq or python3 for JSON parsing" >&2
  exit 127
}

if ! need_cmd curl; then
  echo "ERROR: curl not found" >&2
  exit 127
fi
if ! need_cmd ssh; then
  echo "ERROR: ssh not found" >&2
  exit 127
fi

NODES=(
  "188.245.97.41"
  "135.181.145.174"
  "5.223.51.238"
  "178.156.219.107"
)

fail=0
hashes=()

for ip in "${NODES[@]}"; do
  echo "=== ${ip} ==="

  status_json="$(curl -fsS "http://${ip}:${RPC_PORT}/status")" || { echo "FAIL: /status"; fail=1; continue; }
  status="$(json_get "$status_json" '.status')"
  version="$(json_get "$status_json" '.version')"
  peer_count="$(json_get "$status_json" '.peer_count')"
  node_id="$(json_get "$status_json" '.node_id')"

  peers_json="$(curl -fsS "http://${ip}:${RPC_PORT}/peers")" || { echo "FAIL: /peers"; fail=1; continue; }
  peers_len="$(json_get "$peers_json" 'length')"

  time_json="$(curl -fsS "http://${ip}:${RPC_PORT}/time")" || { echo "FAIL: /time"; fail=1; continue; }
  time_us="$(json_get "$time_json" '.time_us')"

  # Monotonic time samples
  prev=""
  monotonic_ok=1
  for ((i=1; i<=TIME_SAMPLES; i++)); do
    t="$(curl -fsS "http://${ip}:${RPC_PORT}/time" | { if need_cmd jq; then jq -r '.time_us'; else python3 -c 'import json,sys; print(json.load(sys.stdin).get(\"time_us\", \"\"))'; fi; })"
    if [[ -n "$prev" ]] && [[ "$t" -lt "$prev" ]]; then monotonic_ok=0; fi
    prev="$t"
    sleep 0.2
  done

  sha="$(ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "root@${ip}" "sha256sum /usr/local/bin/ippan-node 2>/dev/null | awk '{print \$1}'" | head -n1 || true)"

  echo "status=${status} version=${version} node_id=${node_id} peer_count=${peer_count} peers_len=${peers_len} time_us=${time_us} sha256=${sha}"

  if [[ "$status" != "ok" ]]; then
    echo "FAIL: status != ok"
    fail=1
  fi
  if [[ "$peers_len" -ne "$EXPECTED_PEERS" ]]; then
    echo "FAIL: peers_len != ${EXPECTED_PEERS}"
    fail=1
  fi
  if [[ "$monotonic_ok" -ne 1 ]]; then
    echo "FAIL: time not monotonic"
    fail=1
  fi
  if [[ -z "$sha" ]]; then
    echo "FAIL: missing sha256"
    fail=1
  else
    hashes+=("$sha")
  fi
done

unique_hashes="$(printf '%s\n' "${hashes[@]:-}" | sort -u | wc -l | tr -d ' ')"
echo "=== SUMMARY ==="
echo "unique_hashes=${unique_hashes}"

if [[ "${#hashes[@]}" -gt 0 ]] && [[ "$unique_hashes" -ne 1 ]]; then
  echo "FAIL: sha256 mismatch across nodes"
  exit 2
fi

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo "OK: all nodes healthy and binary hashes match"


