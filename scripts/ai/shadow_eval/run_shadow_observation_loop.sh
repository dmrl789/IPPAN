#!/usr/bin/env bash
set -euo pipefail

# Periodically collect /status + /ai/status snapshots.
# Usage:
#   ./scripts/ai/shadow_eval/run_shadow_observation_loop.sh [ITERATIONS] [SLEEP_SECONDS]

ITERATIONS="${1:-288}"     # default ~288 samples
SLEEP_SECONDS="${2:-300}"  # default 300s (5 min) between samples

echo "Running shadow observation for ${ITERATIONS} iterations,"
echo "sleeping ${SLEEP_SECONDS}s between snapshots."

for (( i=1; i<=ITERATIONS; i++ )); do
  echo
  echo "=== Iteration ${i}/${ITERATIONS} ==="
  ./scripts/ai/shadow_eval/collect_shadow_status_once.sh || true
  if [ "${i}" -lt "${ITERATIONS}" ]; then
    sleep "${SLEEP_SECONDS}"
  fi
done

echo "OK: Shadow observation loop finished."
echo "Check logs/ai_shadow/ for all collected snapshots."


