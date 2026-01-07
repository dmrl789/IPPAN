#!/usr/bin/env bash
set -euo pipefail

# Usage: collect_operator_view.sh <srv-name> <srv-ip> <port-from> <port-to>
NAME="${1:?}"
IP="${2:?}"
P_FROM="${3:?}"
P_TO="${4:?}"

OUT="ops/ippantime/out/operator_${NAME}_$(date +%Y%m%d_%H%M%S).txt"
mkdir -p ops/ippantime/out

{
  echo "=== OPERATOR VIEW: $NAME ($IP) ==="
  date -Is
  echo
  echo "== Port reachability =="
  for p in $(seq "$P_FROM" "$P_TO"); do
    printf "%s: " "$p"
    curl -fsS --max-time 2 "http://${IP}:${p}/status" | grep -q ippan_time_us && echo OK || echo MISS
  done
  echo
  echo "== Sample RTT spot-check =="
  for p in $(seq "$P_FROM" "$P_TO"); do
    printf "%s: " "$p"
    curl -o /dev/null -s -w 'time_total=%{time_total}\n' --max-time 2 "http://${IP}:${p}/status" || true
  done
} > "$OUT"

echo "Wrote $OUT"

