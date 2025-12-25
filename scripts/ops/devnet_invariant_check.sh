#!/usr/bin/env bash
set -euo pipefail

# Devnet invariant gate:
# - /health responds (ok=true)
# - /status.peer_count >= 3
# - /status.validator_count == 4
# - /status.validator_ids_sample identical across nodes
# - /status.consensus.round advances over time (two samples, 5s apart)
#
# Usage:
#   ./scripts/ops/devnet_invariant_check.sh
#   NODES="api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk" ./scripts/ops/devnet_invariant_check.sh
#   RPC_PORT=8080 SLEEP_SECS=5 ./scripts/ops/devnet_invariant_check.sh 1.2.3.4 2.3.4.5 3.4.5.6 4.5.6.7

RPC_PORT="${RPC_PORT:-8080}"
SLEEP_SECS="${SLEEP_SECS:-5}"
MIN_PEERS="${MIN_PEERS:-3}"
EXPECTED_VALIDATORS="${EXPECTED_VALIDATORS:-4}"
CURL_TIMEOUT_SECS="${CURL_TIMEOUT_SECS:-2}"

need_cmd() { command -v "$1" >/dev/null 2>&1; }

if ! need_cmd curl; then
  echo "ERROR: curl not found" >&2
  exit 127
fi

if ! need_cmd python3; then
  echo "ERROR: python3 not found (required; jq is intentionally not used)" >&2
  exit 127
fi

json_get() {
  # Usage: json_get '<json>' '<expr>'
  #
  # expr supports dotted paths (leading '.' optional), e.g:
  #   .peer_count
  #   .consensus.round
  # For arrays/dicts, outputs compact JSON with stable key ordering.
  local json="$1"
  local expr="$2"
  python3 -c '
import json,sys
expr=sys.argv[1].strip()
raw=sys.stdin.read()
try:
    data=json.loads(raw) if raw.strip() else None
except Exception:
    print("")
    raise SystemExit(0)
def get(obj, path):
    cur=obj
    for p in path:
        if isinstance(cur, dict):
            cur=cur.get(p)
        else:
            return None
    return cur
if expr.startswith("."):
    expr=expr[1:]
path=[p for p in expr.split(".") if p]
val=get(data, path) if data is not None else None
if isinstance(val, (dict,list)):
    print(json.dumps(val, sort_keys=True))
elif val is None:
    print("")
else:
    print(val)
' "$expr" <<<"$json"
}

is_jsonish() {
  local s="$1"
  [[ -n "$s" ]] && [[ "${s:0:1}" == "{" || "${s:0:1}" == "[" ]]
}

json_compact() {
  local json="$1"
  local expr="$2"
  json_get "$json" "$expr"
}

default_nodes=(api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk)

nodes=()
if [[ "${#@}" -ge 1 ]]; then
  nodes=("$@")
elif [[ -n "${NODES:-}" ]]; then
  # shellcheck disable=SC2206
  nodes=(${NODES})
else
  nodes=("${default_nodes[@]}")
fi

if [[ "${#nodes[@]}" -ne 4 ]]; then
  echo "ERROR: expected 4 nodes, got ${#nodes[@]}: ${nodes[*]}" >&2
  exit 2
fi

echo "Nodes: ${nodes[*]}"
echo "RPC_PORT=${RPC_PORT} MIN_PEERS=${MIN_PEERS} EXPECTED_VALIDATORS=${EXPECTED_VALIDATORS} SLEEP_SECS=${SLEEP_SECS}"

fail=0
declare -A round1=()
declare -A round2=()
declare -A vset=()

fetch_status() {
  local host="$1"
  curl -fsS -m "$CURL_TIMEOUT_SECS" -H "Accept: application/json" "http://${host}:${RPC_PORT}/status"
}

fetch_health() {
  local host="$1"
  curl -fsS -m "$CURL_TIMEOUT_SECS" -H "Accept: application/json" "http://${host}:${RPC_PORT}/health"
}

get_round() {
  # Prefer consensus.round; fall back to top-level round for compatibility.
  local status_json="$1"
  local r
  r="$(json_get "$status_json" '.consensus.round')"
  if [[ -n "$r" ]]; then
    echo "$r"
    return 0
  fi
  r="$(json_get "$status_json" '.round')"
  echo "$r"
}

check_once() {
  local host="$1"
  local status_json="$2"
  local health_json="$3"

  if ! is_jsonish "$health_json"; then
    echo "FAIL ${host}: /health did not return JSON (first bytes: ${health_json:0:80})" >&2
    return 1
  fi
  if ! is_jsonish "$status_json"; then
    echo "FAIL ${host}: /status did not return JSON (first bytes: ${status_json:0:80})" >&2
    return 1
  fi

  # Health semantics: accept either legacy {"ok":true} or modern {"rpc_healthy":true,...}.
  local ok rpc_healthy
  ok="$(json_get "$health_json" '.ok')"
  rpc_healthy="$(json_get "$health_json" '.rpc_healthy')"
  if [[ -n "$ok" ]]; then
    ok_norm="${ok,,}"
    if [[ "$ok_norm" != "true" && "$ok_norm" != "1" ]]; then
      echo "FAIL ${host}: /health ok != true (got: ${ok})" >&2
      return 1
    fi
  elif [[ -n "$rpc_healthy" ]]; then
    rpc_norm="${rpc_healthy,,}"
    if [[ "$rpc_norm" != "true" && "$rpc_norm" != "1" ]]; then
      echo "FAIL ${host}: /health rpc_healthy != true (got: ${rpc_healthy})" >&2
      return 1
    fi
  fi

  local peer_count validator_count round validators
  peer_count="$(json_get "$status_json" '.peer_count')"
  validator_count="$(json_get "$status_json" '.validator_count')"
  round="$(get_round "$status_json")"
  validators="$(json_compact "$status_json" '.validator_ids_sample')"

  echo "${host}: peer_count=${peer_count} validator_count=${validator_count} round=${round} validators=${validators}"

  # Record what we observed even if a later check fails (helps diagnose drift).
  vset["$host"]="$validators"
  round1["$host"]="$round"

  if [[ -z "$peer_count" ]] || [[ "$peer_count" -lt "$MIN_PEERS" ]]; then
    echo "FAIL ${host}: peer_count < ${MIN_PEERS} (got: ${peer_count})" >&2
    return 1
  fi
  if [[ -z "$validator_count" ]] || [[ "$validator_count" -ne "$EXPECTED_VALIDATORS" ]]; then
    echo "FAIL ${host}: validator_count != ${EXPECTED_VALIDATORS} (got: ${validator_count})" >&2
    return 1
  fi
  if [[ -z "$round" ]]; then
    echo "FAIL ${host}: missing round (expected consensus.round or round)" >&2
    return 1
  fi
  return 0
}

echo "=== sample #1 ==="
for host in "${nodes[@]}"; do
  status_json="$(fetch_status "$host")" || { echo "FAIL ${host}: /status unreachable" >&2; fail=1; continue; }
  health_json="$(fetch_health "$host")" || { echo "FAIL ${host}: /health unreachable" >&2; fail=1; continue; }
  if ! check_once "$host" "$status_json" "$health_json"; then
    fail=1
  fi
done

echo "Sleeping ${SLEEP_SECS}s..."
sleep "$SLEEP_SECS"

echo "=== sample #2 (round must increase) ==="
for host in "${nodes[@]}"; do
  status_json="$(fetch_status "$host")" || { echo "FAIL ${host}: /status unreachable" >&2; fail=1; continue; }
  r2="$(get_round "$status_json")"
  round2["$host"]="$r2"
  if [[ -z "$r2" ]]; then
    echo "FAIL ${host}: missing round on sample #2" >&2
    fail=1
    continue
  fi
  if [[ "${round1[$host]:-}" -ge "$r2" ]]; then
    echo "FAIL ${host}: round did not advance (${round1[$host]:-} -> ${r2})" >&2
    fail=1
  fi
done

echo "=== validator_ids_sample cross-node check ==="
baseline="${vset[${nodes[0]}]:-}"
if [[ -z "$baseline" ]]; then
  echo "FAIL: baseline validator_ids_sample missing from ${nodes[0]}" >&2
  fail=1
else
  for host in "${nodes[@]}"; do
    if [[ "${vset[$host]:-}" != "$baseline" ]]; then
      echo "FAIL ${host}: validator_ids_sample drift" >&2
      echo "  baseline(${nodes[0]}): ${baseline}" >&2
      echo "  ${host}: ${vset[$host]:-}" >&2
      fail=1
    fi
  done
fi

echo "=== SUMMARY ==="
for host in "${nodes[@]}"; do
  echo "${host}: round ${round1[$host]:-?} -> ${round2[$host]:-?}"
done

if [[ "$fail" -ne 0 ]]; then
  echo "DEVNET INVARIANT: FAIL" >&2
  exit 1
fi

echo "DEVNET INVARIANT: PASS"

