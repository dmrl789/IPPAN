#!/usr/bin/env bash
set -euo pipefail

# Devnet invariant gate:
# - validator_count == expected
# - peer_count == expected
# - consensus round advances
# - validator ids set is consistent across nodes (best-effort; uses validator_ids_sample when present)
#
# Usage:
#   ./scripts/ops/devnet_invariant_check.sh api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk
#
# Env:
#   EXPECTED_VALIDATORS=4
#   EXPECTED_PEERS=4

EXPECTED_VALIDATORS="${EXPECTED_VALIDATORS:-4}"
EXPECTED_PEERS="${EXPECTED_PEERS:-4}"

if [[ "$#" -lt 1 ]]; then
  echo "Usage: $0 <host1> [host2...]" >&2
  exit 2
fi

need_cmd() { command -v "$1" >/dev/null 2>&1; }
if ! need_cmd curl; then echo "ERROR: curl not found" >&2; exit 127; fi
if ! need_cmd python3; then echo "ERROR: python3 not found" >&2; exit 127; fi

fetch_status() {
  local host="$1"
  # Retry a bit and use a longer max-time; /status can be briefly slow under load.
  curl -fsS --connect-timeout 2 --max-time 6 --retry 2 --retry-delay 1 --retry-all-errors \
    "http://${host}:8080/status"
}

json_ok() {
  python3 -c 'import json,sys; json.load(sys.stdin);' >/dev/null 2>&1
}

extract_fields() {
  # Prints: validator_count peer_count round vids_join
  python3 -c 'import json,sys; s=json.load(sys.stdin); vc=s.get("validator_count"); pc=s.get("peer_count"); r=(s.get("consensus") or {}).get("round"); vids=s.get("validator_ids_sample"); vids_join=""; 
if isinstance(vids,list): vids_join=",".join(sorted(str(x) for x in vids));
print("" if vc is None else vc); print("" if pc is None else pc); print("" if r is None else r); print(vids_join)'
}

fail=0
rounds=()
validators_seen=()

echo "Checking nodes: $*"

for h in "$@"; do
  echo "→ $h"
  s1="$(fetch_status "$h")" || { echo "FAIL: $h /status"; fail=1; continue; }
  if ! json_ok <<<"$s1"; then
    echo "FAIL: $h /status returned invalid JSON" >&2
    fail=1
    continue
  fi
  mapfile -t fields1 < <(extract_fields <<<"$s1")
  vc="${fields1[0]:-}"
  pc="${fields1[1]:-}"
  r1="${fields1[2]:-}"
  vids="${fields1[3]:-}"

  # Round advance check.
  sleep 2
  s2="$(fetch_status "$h")" || { echo "FAIL: $h /status (second)"; fail=1; continue; }
  if ! json_ok <<<"$s2"; then
    echo "FAIL: $h /status (second) returned invalid JSON" >&2
    fail=1
    continue
  fi
  mapfile -t fields2 < <(extract_fields <<<"$s2")
  r2="${fields2[2]:-}"

  echo "$r2"
  echo "${vids}" | tr -d '\n' || true
  echo ""

  if [[ "${vc:-}" != "${EXPECTED_VALIDATORS}" ]]; then
    echo "FAIL: $h validator_count=$vc expected=$EXPECTED_VALIDATORS" >&2
    fail=1
  fi
  if [[ "${pc:-}" != "${EXPECTED_PEERS}" ]]; then
    echo "FAIL: $h peer_count=$pc expected=$EXPECTED_PEERS" >&2
    fail=1
  fi
  if [[ -n "${r1:-}" && -n "${r2:-}" ]]; then
    if [[ "$r2" -le "$r1" ]]; then
      echo "FAIL: $h round not advancing ($r1 -> $r2)" >&2
      fail=1
    fi
  else
    echo "FAIL: $h missing consensus.round in /status" >&2
    fail=1
  fi

  rounds+=("$r2")
  validators_seen+=("$vids")
done

if [[ "$fail" -ne 0 ]]; then
  echo "❌ DEVNET INVARIANTS FAIL"
  exit 1
fi

# Best-effort: validator_ids_sample should match across nodes when present.
uniq_vids="$(printf '%s\n' "${validators_seen[@]}" | sed '/^$/d' | sort -u | wc -l | tr -d ' ')"
if [[ "${uniq_vids:-0}" -gt 1 ]]; then
  echo "FAIL: validator_ids_sample mismatch across nodes (drift)" >&2
  printf '%s\n' "${validators_seen[@]}" | sed 's/^/  /' >&2
  exit 2
fi

echo "✅ DEVNET INVARIANTS OK"


