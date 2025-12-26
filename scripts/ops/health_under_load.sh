#!/usr/bin/env bash
set -euo pipefail

# Simple /health probe loop to catch timeouts/errors during load tests.
#
# Usage:
#   RPC="http://127.0.0.1:18080" ./scripts/ops/health_under_load.sh

RPC="${RPC:-}"
if [[ -z "$RPC" ]]; then
  echo "ERROR: RPC env var required (e.g. http://127.0.0.1:18080)" >&2
  exit 2
fi

DURATION_S="${DURATION_S:-30}"
INTERVAL_S="${INTERVAL_S:-1}"

ok=0
fail=0
max_ms=0
sum_ms=0
count=0

end=$(( $(date +%s) + DURATION_S ))
while [[ $(date +%s) -lt $end ]]; do
  # time_total in seconds; convert to ms.
  t="$(curl -sS --connect-timeout 2 --max-time 4 -o /dev/null -w '%{time_total}' "$RPC/health" || true)"
  if [[ -z "$t" ]]; then
    fail=$((fail+1))
  else
    ms="$(python3 - <<PY
import sys
print(int(float(sys.argv[1])*1000))
PY
"$t")"
    ok=$((ok+1))
    count=$((count+1))
    sum_ms=$((sum_ms+ms))
    if [[ $ms -gt $max_ms ]]; then max_ms=$ms; fi
  fi
  sleep "$INTERVAL_S"
done

avg_ms=0
if [[ $count -gt 0 ]]; then avg_ms=$((sum_ms/count)); fi

echo "health_ok: $ok"
echo "health_fail: $fail"
echo "health_avg_ms: $avg_ms"
echo "health_max_ms: $max_ms"
echo "SUMMARY health_ok=$ok health_fail=$fail health_avg_ms=$avg_ms health_max_ms=$max_ms"


