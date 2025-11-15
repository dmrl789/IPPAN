#!/usr/bin/env bash

set -euo pipefail

COMMAND="${1:-help}"
if [[ $# -gt 0 ]]; then
  shift
fi

RPC_URL="${RPC_URL:-http://127.0.0.1:8080}"
KEY_DIR="${KEY_DIR:-./demo-keys}"
SENDER_NAME="${SENDER_NAME:-demo-sender}"
RECEIVER_NAME="${RECEIVER_NAME:-demo-receiver}"
FUND_AMOUNT="${FUND_AMOUNT:-1000000000}"
PAY_AMOUNT="${PAY_AMOUNT:-250000000}"
FEE_LIMIT="${FEE_LIMIT:-2000}"
POLL_INTERVAL="${POLL_INTERVAL:-2}"
POLL_SECONDS="${POLL_SECONDS:-60}"
DEFAULT_MEMO="${DEMO_MEMO:-demo payment}"
SENDER_NONCE="${SENDER_NONCE:-0}"

LAST_TX_HASH=""

log() {
  echo "[$(date +%H:%M:%S)] $*"
}

usage() {
  cat <<'EOF'
Usage: scripts/demo_payment_flow.sh <command>

Commands:
  help        Show this message
  keys        Generate (or show) demo sender/receiver key pairs
  fund        Fund the sender via /dev/fund (requires node started with --dev)
  pay [memo]  Submit a demo payment using ippan-cli pay
  history     Query sender and receiver payment history
  run-all     keys + fund + pay + history in one go

Environment overrides:
  RPC_URL=http://127.0.0.1:8080
  KEY_DIR=./demo-keys
  FUND_AMOUNT=1000000000        # atomic units credited to sender
  PAY_AMOUNT=250000000          # atomic units transferred
  FEE_LIMIT=2000                # fee ceiling (atomic units)
  DEMO_MEMO="demo payment"

The node must be running with IPPAN_DEV_MODE=true (or --dev) so /dev/fund is enabled.
EOF
}

ensure_node_ready() {
  log "Checking node at ${RPC_URL}/health"
  local waited=0
  while ! curl -sf "${RPC_URL}/health" >/dev/null 2>&1; do
    if (( waited >= POLL_SECONDS )); then
      echo "Node at ${RPC_URL} is not responding" >&2
      exit 1
    fi
    sleep "${POLL_INTERVAL}"
    waited=$((waited + POLL_INTERVAL))
  done
  log "Node is responding."
}

ensure_keys() {
  mkdir -p "${KEY_DIR}"
  if [[ ! -f "${KEY_DIR}/${SENDER_NAME}_private.key" ]]; then
    log "Generating sender keypair (${SENDER_NAME})"
    cargo run -p keygen -- generate --output "${KEY_DIR}" --name "${SENDER_NAME}" >/dev/null
  fi
  if [[ ! -f "${KEY_DIR}/${RECEIVER_NAME}_private.key" ]]; then
    log "Generating receiver keypair (${RECEIVER_NAME})"
    cargo run -p keygen -- generate --output "${KEY_DIR}" --name "${RECEIVER_NAME}" >/dev/null
  fi
}

read_hex() {
  tr -d '\r\n' <"$1" | tr '[:upper:]' '[:lower:]'
}

load_identities() {
  ensure_keys
  local sender_pub receiver_pub
  sender_pub="$(read_hex "${KEY_DIR}/${SENDER_NAME}_public.key")"
  receiver_pub="$(read_hex "${KEY_DIR}/${RECEIVER_NAME}_public.key")"
  SENDER_HEX="${sender_pub}"
  RECEIVER_HEX="${receiver_pub}"
  SENDER_ADDR="i${sender_pub}"
  RECEIVER_ADDR="i${receiver_pub}"
  SENDER_PRIV="${KEY_DIR}/${SENDER_NAME}_private.key"
  RECEIVER_PRIV="${KEY_DIR}/${RECEIVER_NAME}_private.key"
}

fund_sender() {
  load_identities
  ensure_node_ready
  local payload
  payload=$(printf '{"address":"%s","amount":%s,"nonce":%s}' "${SENDER_ADDR}" "${FUND_AMOUNT}" "${SENDER_NONCE}")
  log "Funding sender via ${RPC_URL}/dev/fund"
  curl -sSf -H "Content-Type: application/json" -d "${payload}" "${RPC_URL}/dev/fund" | sed 's/^/[fund] /'
}

send_payment() {
  load_identities
  ensure_node_ready
  local memo="${1:-$DEFAULT_MEMO}"
  log "Submitting payment from ${SENDER_ADDR} -> ${RECEIVER_ADDR}"
  set +e
  local output
  output=$(cargo run -p cli -- --rpc-url "${RPC_URL}" pay \
    --from "${SENDER_ADDR}" \
    --to "${RECEIVER_ADDR}" \
    --amount "${PAY_AMOUNT}" \
    --fee "${FEE_LIMIT}" \
    --key-file "${SENDER_PRIV}" \
    --memo "${memo}" 2>&1)
  local status=$?
  set -e
  echo "${output}"
  if [[ ${status} -ne 0 ]]; then
    echo "Payment submission failed" >&2
    exit "${status}"
  fi
  local hash
  hash=$(echo "${output}" | awk '/Payment accepted:/ {print $3}' | tr -d '\r')
  if [[ -z "${hash}" ]]; then
    hash=$(echo "${output}" | grep -Eo '[0-9a-f]{64}' | tail -n1 || true)
  fi
  if [[ -z "${hash}" ]]; then
    echo "Could not detect transaction hash in CLI output" >&2
    exit 1
  fi
  LAST_TX_HASH="${hash}"
  printf "%s" "${hash}" >"${KEY_DIR}/.last_payment"
  log "Payment hash: ${hash}"
}

wait_for_tx() {
  local hash="${1:-$LAST_TX_HASH}"
  if [[ -z "${hash}" ]]; then
    echo "No transaction hash provided" >&2
    exit 1
  }
  log "Waiting for transaction ${hash} to appear in storage"
  local waited=0
  while ! curl -sf "${RPC_URL}/tx/${hash}" >/dev/null 2>&1; do
    if (( waited >= POLL_SECONDS )); then
      log "Transaction ${hash} not yet visible (continuing anyway)"
      return
    fi
    sleep "${POLL_INTERVAL}"
    waited=$((waited + POLL_INTERVAL))
  done
  log "Transaction ${hash} available through /tx/${hash}"
}

print_payments() {
  local label="$1"
  local hex_addr="$2"
  local url="${RPC_URL}/account/${hex_addr}/payments?limit=5"
  log "Querying ${label} payments (${url})"
  if command -v jq >/dev/null 2>&1; then
    curl -sSf "${url}" | jq
  else
    curl -sSf "${url}"
  fi
}

show_history() {
  load_identities
  ensure_node_ready
  local hash_file="${KEY_DIR}/.last_payment"
  if [[ -z "${LAST_TX_HASH}" && -f "${hash_file}" ]]; then
    LAST_TX_HASH="$(cat "${hash_file}")"
  fi
  if [[ -n "${LAST_TX_HASH}" ]]; then
    wait_for_tx "${LAST_TX_HASH}"
  fi
  print_payments "sender" "${SENDER_HEX}"
  print_payments "receiver" "${RECEIVER_HEX}"
}

case "${COMMAND}" in
  help)
    usage
    ;;
  keys)
    load_identities
    log "Sender address: ${SENDER_ADDR}"
    log "Receiver address: ${RECEIVER_ADDR}"
    ;;
  fund)
    fund_sender
    ;;
  pay)
    send_payment "${1:-$DEFAULT_MEMO}"
    ;;
  history)
    show_history
    ;;
  run-all)
    fund_sender
    send_payment "${1:-$DEFAULT_MEMO}"
    show_history
    ;;
  *)
    echo "Unknown command: ${COMMAND}" >&2
    usage
    exit 1
    ;;
esac
