#!/usr/bin/env bash
set -euo pipefail

# Staged batch ingest: 200k → 300k → 400k → 500k offered TPS.
#
# Runs on api1 (no tunnel). Required env:
# - RPC (e.g. http://127.0.0.1:8080)
# - TO_ADDR (raw address; no handles)
# - NONCE_START (u64)
# - SENDERS_FILE (json) OR SENDER_KEY (hex private key file)
#
# Optional env:
# - BATCH_SIZE (default 1024 - smaller batches for high TPS)
# - CONCURRENCY (default 256 - high concurrency for 500k)
# - MAX_INFLIGHT (default 8192)
# - MAX_QUEUE (default 200000)
# - DRAIN_SECONDS (default 10)
# - STAGE_SECONDS (default 20 per stage)
# - OUT_BASE (default /var/lib/ippan/out)
# - TXLOAD_BIN (default ippan-txload)
#
# Stop conditions (script exits with error if any occur):
# - client_errors > 0
# - invalid > 0
# - dropped_queue_full > 0 (client bottleneck - fix txload knobs)

RPC="${RPC:-}"
TO_ADDR="${TO_ADDR:-}"
NONCE_START="${NONCE_START:-}"
SENDERS_FILE="${SENDERS_FILE:-}"
SENDER_KEY="${SENDER_KEY:-}"

# High-TPS tuned defaults (differ from ramp_batch_200k.sh)
BATCH_SIZE="${BATCH_SIZE:-1024}"           # Smaller batches = less decode burst latency
CONCURRENCY="${CONCURRENCY:-256}"          # High concurrency for 500k offered
MAX_INFLIGHT="${MAX_INFLIGHT:-8192}"       # Large client-side buffer
MAX_QUEUE="${MAX_QUEUE:-200000}"           # Very large queue for high TPS
DRAIN_SECONDS="${DRAIN_SECONDS:-10}"
STAGE_SECONDS="${STAGE_SECONDS:-20}"       # 20s per stage
OUT_BASE="${OUT_BASE:-/var/lib/ippan/out}"
TXLOAD_BIN="${TXLOAD_BIN:-ippan-txload}"

if [[ -z "${RPC}" ]]; then echo "missing RPC" >&2; exit 2; fi
if [[ -z "${TO_ADDR}" ]]; then echo "missing TO_ADDR" >&2; exit 2; fi
if [[ -z "${NONCE_START}" ]]; then echo "missing NONCE_START" >&2; exit 2; fi
if [[ -n "${SENDER_KEY}" && -n "${SENDERS_FILE}" ]]; then echo "provide only one of SENDER_KEY or SENDERS_FILE" >&2; exit 2; fi
if [[ -z "${SENDER_KEY}" && -z "${SENDERS_FILE}" ]]; then echo "missing sender material: set SENDER_KEY or SENDERS_FILE" >&2; exit 2; fi

need_cmd() { command -v "$1" >/dev/null 2>&1; }
if ! need_cmd curl; then echo "ERROR: curl not found" >&2; exit 127; fi
if ! need_cmd python3; then echo "ERROR: python3 not found" >&2; exit 127; fi
if ! need_cmd "${TXLOAD_BIN}"; then echo "ERROR: ${TXLOAD_BIN} not found in PATH" >&2; exit 127; fi

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="${OUT_BASE}/batch_${STAMP}"
mkdir -p "${OUT_DIR}"
STOP_FILE="${OUT_DIR}/RPC_UNHEALTHY_STOP.txt"

health_check() {
  curl -fsS --max-time 2 "${RPC}/health" >/dev/null 2>&1
}

status_check() {
  curl -fsS --max-time 5 "${RPC}/status" 2>/dev/null | head -c 500 || true
}

txload_sender_flags() {
  if [[ -n "${SENDER_KEY}" ]]; then
    printf -- "--from-key %q" "${SENDER_KEY}"
  else
    printf -- "--senders-file %q" "${SENDERS_FILE}"
  fi
}

parse_kv() {
  # usage: parse_kv "<line>" "<key>"
  python3 - <<'PY' "$1" "$2"
import sys
line = sys.argv[1]
key = sys.argv[2]
out = "0"
for part in line.strip().split():
    if part.startswith(key + "="):
        out = part.split("=", 1)[1]
print(out)
PY
}

run_stage() {
  local TPS="$1"
  local SECS="$2"
  local LABEL="${3:-${TPS}}"
  local NONCE_COUNT=$((TPS * SECS))
  local LOG="${OUT_DIR}/run_${LABEL}_${SECS}s.log"
  local SUMMARY_FILE="${OUT_DIR}/summary_${LABEL}.txt"

  echo "=== stage offered=${TPS} tps for ${SECS}s (label=${LABEL}) ===" | tee "${LOG}"
  echo "OUT_DIR=${OUT_DIR}" | tee -a "${LOG}"
  echo "BATCH_SIZE=${BATCH_SIZE} CONCURRENCY=${CONCURRENCY} MAX_INFLIGHT=${MAX_INFLIGHT} MAX_QUEUE=${MAX_QUEUE}" | tee -a "${LOG}"

  # Pre-flight health check (2 attempts)
  if ! health_check; then
    echo "health check failed (1/2) before stage ${LABEL}" >&2 | tee -a "${LOG}"
    sleep 1
  fi
  if ! health_check; then
    echo "health check failed (2/2) before stage ${LABEL}; stopping" >&2 | tee -a "${LOG}"
    echo "RPC health failed twice before stage ${LABEL} at $(date -Is)" > "${STOP_FILE}"
    exit 1
  fi
  echo "pre-flight /health OK" | tee -a "${LOG}"
  status_check | tee -a "${LOG}"
  echo | tee -a "${LOG}"

  set +e
  # shellcheck disable=SC2046
  "${TXLOAD_BIN}" batch \
    --rpc "${RPC}" \
    --to "${TO_ADDR}" \
    --tps "${TPS}" \
    --seconds "${SECS}" \
    --batch-size "${BATCH_SIZE}" \
    --concurrency "${CONCURRENCY}" \
    --max-inflight "${MAX_INFLIGHT}" \
    --drain-seconds "${DRAIN_SECONDS}" \
    $(txload_sender_flags) \
    --nonce-start "${NONCE_START}" \
    --nonce-count "${NONCE_COUNT}" \
    2>&1 | tee -a "${LOG}"
  EXIT_CODE=${PIPESTATUS[0]}
  set -e

  if [[ "${EXIT_CODE}" -ne 0 ]]; then
    echo "txload exited non-zero (${EXIT_CODE}); stopping" >&2 | tee -a "${LOG}"
    exit 1
  fi

  # Extract SUMMARY line
  SUMMARY="$(grep -E '^SUMMARY ' "${LOG}" | tail -n 1 || true)"
  if [[ -z "${SUMMARY}" ]]; then
    echo "missing SUMMARY line; stopping" >&2 | tee -a "${LOG}"
    exit 1
  fi

  # Write summary to dedicated file
  echo "${SUMMARY}" > "${SUMMARY_FILE}"
  echo "Summary written to: ${SUMMARY_FILE}" | tee -a "${LOG}"
  echo "${SUMMARY}" | tee -a "${LOG}"

  # Stop conditions
  CLIENT_ERRORS="$(parse_kv "${SUMMARY}" "client_errors")"
  if [[ "${CLIENT_ERRORS}" != "0" ]]; then
    echo "STOP: client_errors=${CLIENT_ERRORS} > 0" >&2 | tee -a "${LOG}"
    exit 2
  fi

  INVALID="$(parse_kv "${SUMMARY}" "invalid")"
  if [[ "${INVALID}" != "0" ]]; then
    echo "STOP: invalid=${INVALID} > 0" >&2 | tee -a "${LOG}"
    exit 3
  fi

  DROPPED="$(parse_kv "${SUMMARY}" "dropped_queue_full")"
  if [[ "${DROPPED}" != "0" ]]; then
    echo "STOP: dropped_queue_full=${DROPPED} > 0 (client bottleneck - increase MAX_INFLIGHT/MAX_QUEUE or senders)" >&2 | tee -a "${LOG}"
    exit 4
  fi

  # Post-flight health check (2 attempts)
  if ! health_check; then
    echo "health check failed (1/2) after stage ${LABEL}" >&2 | tee -a "${LOG}"
    sleep 1
  fi
  if ! health_check; then
    echo "health check failed (2/2) after stage ${LABEL}; stopping" >&2 | tee -a "${LOG}"
    echo "RPC health failed twice after stage ${LABEL} at $(date -Is)" > "${STOP_FILE}"
    exit 1
  fi
  echo "post-flight /health OK" | tee -a "${LOG}"
  status_check | tee -a "${LOG}"
  echo | tee -a "${LOG}"

  # Update nonce start for next stage
  NONCE_START=$((NONCE_START + NONCE_COUNT))
}

echo "=============================================="
echo "Batch Ingest Ramp: 200k → 300k → 400k → 500k"
echo "=============================================="
echo "RPC=${RPC}"
echo "OUT_DIR=${OUT_DIR}"
echo "BATCH_SIZE=${BATCH_SIZE} CONCURRENCY=${CONCURRENCY}"
echo "MAX_INFLIGHT=${MAX_INFLIGHT} MAX_QUEUE=${MAX_QUEUE}"
echo "DRAIN_SECONDS=${DRAIN_SECONDS} STAGE_SECONDS=${STAGE_SECONDS}"
echo "TO_ADDR=${TO_ADDR} NONCE_START=${NONCE_START}"
echo "=============================================="
echo

# Stages: 200k, 300k, 400k, 500k offered TPS
run_stage 200000 "${STAGE_SECONDS}" "200k"
run_stage 300000 "${STAGE_SECONDS}" "300k"
run_stage 400000 "${STAGE_SECONDS}" "400k"
run_stage 500000 "${STAGE_SECONDS}" "500k"

echo "=============================================="
echo "Ramp completed OK: ${OUT_DIR}"
echo "=============================================="
echo "Summaries:"
for f in "${OUT_DIR}"/summary_*.txt; do
  echo "--- $(basename "$f") ---"
  cat "$f"
done

