#!/usr/bin/env bash
set -euo pipefail

# Staged batch ingest at 20k TPS offered (20s then 60s).
#
# Required env:
# - RPC (e.g. http://127.0.0.1:8080)  # run on api1 (no tunnel)
# - TO_ADDR (raw address; no handles)
# - NONCE_START (u64)
# - SENDERS_FILE (json) OR SENDER_KEY (hex private key file)
#
# Optional env:
# - BATCH_SIZE (default 2048)
# - CONCURRENCY (default 16)
# - MAX_INFLIGHT (default 2*concurrency)
# - DRAIN_SECONDS (default 10)
# - OUT_BASE (default /var/lib/ippan/out)
# - TXLOAD_BIN (default ippan-txload)
# - INVARIANT_HOSTS (optional)

RPC="${RPC:-}"
TO_ADDR="${TO_ADDR:-}"
NONCE_START="${NONCE_START:-}"
SENDERS_FILE="${SENDERS_FILE:-}"
SENDER_KEY="${SENDER_KEY:-}"

BATCH_SIZE="${BATCH_SIZE:-2048}"
CONCURRENCY="${CONCURRENCY:-16}"
MAX_INFLIGHT="${MAX_INFLIGHT:-$((CONCURRENCY * 2))}"
DRAIN_SECONDS="${DRAIN_SECONDS:-10}"
OUT_BASE="${OUT_BASE:-/var/lib/ippan/out}"
TXLOAD_BIN="${TXLOAD_BIN:-ippan-txload}"
INVARIANT_HOSTS="${INVARIANT_HOSTS:-}"

if [[ -z "${RPC}" ]]; then
  echo "missing RPC (e.g. RPC=http://127.0.0.1:8080)" >&2
  exit 2
fi
if [[ -z "${TO_ADDR}" ]]; then
  echo "missing TO_ADDR (raw address)" >&2
  exit 2
fi
if [[ -z "${NONCE_START}" ]]; then
  echo "missing NONCE_START (u64)" >&2
  exit 2
fi
if [[ -n "${SENDER_KEY}" && -n "${SENDERS_FILE}" ]]; then
  echo "provide only one of SENDER_KEY or SENDERS_FILE" >&2
  exit 2
fi
if [[ -z "${SENDER_KEY}" && -z "${SENDERS_FILE}" ]]; then
  echo "missing sender material: set SENDER_KEY=<path> or SENDERS_FILE=<path>" >&2
  exit 2
fi

need_cmd() { command -v "$1" >/dev/null 2>&1; }
if ! need_cmd curl; then echo "ERROR: curl not found" >&2; exit 127; fi
if ! need_cmd python3; then echo "ERROR: python3 not found" >&2; exit 127; fi
if ! need_cmd "${TXLOAD_BIN}"; then echo "ERROR: ${TXLOAD_BIN} not found in PATH" >&2; exit 127; fi

STAMP="$(date +%Y%m%d_%H%M%S)"
OUT_DIR="${OUT_BASE}/batch_${STAMP}"
mkdir -p "${OUT_DIR}"
STOP_FILE="${OUT_DIR}/RPC_UNHEALTHY_STOP.txt"

health_check() {
  curl -fsS --max-time 2 "${RPC}/health" >/dev/null 2>&1
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
  local SECONDS="$2"
  local NONCE_COUNT="$3"
  local LOG="${OUT_DIR}/stage_tps${TPS}_${SECONDS}s.log"

  echo "=== stage tps=${TPS} seconds=${SECONDS} log=${LOG} ==="

  if ! health_check; then
    echo "health check failed (1/2) before stage tps=${TPS}" >&2
    sleep 1
  fi
  if ! health_check; then
    echo "health check failed (2/2) before stage tps=${TPS}; stopping" >&2
    echo "RPC health failed twice before stage tps=${TPS} at $(date -Is)" > "${STOP_FILE}"
    exit 1
  fi

  set +e
  # shellcheck disable=SC2046
  "${TXLOAD_BIN}" batch \
    --rpc "${RPC}" \
    --to "${TO_ADDR}" \
    --tps "${TPS}" \
    --seconds "${SECONDS}" \
    --batch-size "${BATCH_SIZE}" \
    --concurrency "${CONCURRENCY}" \
    --max-inflight "${MAX_INFLIGHT}" \
    --drain-seconds "${DRAIN_SECONDS}" \
    $(txload_sender_flags) \
    --nonce-start "${NONCE_START}" \
    --nonce-count "${NONCE_COUNT}" \
    2>&1 | tee "${LOG}"
  EXIT_CODE=${PIPESTATUS[0]}
  set -e

  if [[ "${EXIT_CODE}" -ne 0 ]]; then
    echo "txload exited non-zero (${EXIT_CODE}); stopping" >&2
    exit 1
  fi

  SUMMARY="$(grep -E '^SUMMARY ' "${LOG}" | tail -n 1 || true)"
  if [[ -z "${SUMMARY}" ]]; then
    echo "missing SUMMARY line; stopping" >&2
    exit 1
  fi

  CLIENT_ERRORS="$(parse_kv "${SUMMARY}" "client_errors")"
  if [[ "${CLIENT_ERRORS}" != "0" ]]; then
    echo "client_errors=${CLIENT_ERRORS} > 0; stopping" >&2
    exit 1
  fi

  if ! health_check; then
    echo "health check failed (1/2) after stage tps=${TPS}" >&2
    sleep 1
  fi
  if ! health_check; then
    echo "health check failed (2/2) after stage tps=${TPS}; stopping" >&2
    echo "RPC health failed twice after stage tps=${TPS} at $(date -Is)" > "${STOP_FILE}"
    exit 1
  fi

  echo "${SUMMARY}"
  echo
}

echo "RPC=${RPC}"
echo "OUT_DIR=${OUT_DIR}"
echo "BATCH_SIZE=${BATCH_SIZE} CONCURRENCY=${CONCURRENCY} MAX_INFLIGHT=${MAX_INFLIGHT} DRAIN_SECONDS=${DRAIN_SECONDS}"
echo "TO_ADDR=${TO_ADDR}"
echo "NONCE_START=${NONCE_START}"
if [[ -n "${INVARIANT_HOSTS}" ]]; then echo "INVARIANT_HOSTS=${INVARIANT_HOSTS}"; fi
echo

if [[ -n "${INVARIANT_HOSTS}" ]]; then
  ./scripts/ops/devnet_invariant_check.sh ${INVARIANT_HOSTS}
fi

run_stage 20000 20 $((20000 * 20))
run_stage 20000 60 $((20000 * 60))

if [[ -n "${INVARIANT_HOSTS}" ]]; then
  ./scripts/ops/devnet_invariant_check.sh ${INVARIANT_HOSTS}
fi

echo "OK: ${OUT_DIR}"


