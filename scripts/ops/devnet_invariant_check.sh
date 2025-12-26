#!/usr/bin/env bash
set -euo pipefail

NODES="${@:-api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk}"
RPC_PORT="${RPC_PORT:-8080}"
SLEEP_SECS="${SLEEP_SECS:-5}"

echo "Checking nodes: $NODES"

declare -A ROUNDS
declare -A VALS

for n in $NODES; do
  echo "→ $n"

  HEALTH=$(curl -sS --max-time 2 http://$n:$RPC_PORT/health)
  python3 -c 'import json,sys; h=json.load(sys.stdin); assert h.get("rpc_healthy") is True; assert h.get("consensus_healthy") is True' <<<"$HEALTH" \
    || { echo "FAIL health $n"; exit 1; }

  STATUS=$(curl -sS --max-time 2 http://$n:$RPC_PORT/status)
  python3 -c 'import json,sys; s=json.load(sys.stdin); assert s["peer_count"]>=3; assert s["validator_count"]==4; print(s["consensus"]["round"]); print(",".join(sorted(s["validator_ids_sample"])))' <<<"$STATUS"

  ROUNDS[$n]=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["consensus"]["round"])' <<<"$STATUS")
  VALS[$n]=$(python3 -c 'import json,sys; print(",".join(sorted(json.load(sys.stdin)["validator_ids_sample"])))' <<<"$STATUS")
done

sleep $SLEEP_SECS

for n in $NODES; do
  NEW=$(curl -sS http://$n:$RPC_PORT/status | python3 -c 'import json,sys; print(json.load(sys.stdin)["consensus"]["round"])')
  [[ "$NEW" -gt "${ROUNDS[$n]}" ]] || {
    echo "❌ round not advancing on $n"
    exit 1
  }
done

BASE=""
for v in "${VALS[@]}"; do
  if [[ -z "$BASE" ]]; then BASE="$v"; else
    [[ "$v" == "$BASE" ]] || { echo "❌ validator mismatch"; exit 1; }
  fi
done

echo "✅ DEVNET INVARIANTS OK"
