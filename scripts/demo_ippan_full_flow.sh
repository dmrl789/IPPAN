#!/usr/bin/env bash

set -euo pipefail

cat <<'INTRO'
=== IPPAN End-to-End Dev Demo ===
This script assumes an IPPAN node is already running locally (e.g.
`cargo run --bin ippan-node -- --dev`) with dev mode + stub DHTs enabled.
It will generate demo keys, register @demo.ipn, send a payment, publish a
file descriptor, and print AI/DHT status snapshots.
INTRO

RPC_URL="${RPC_URL:-http://127.0.0.1:8080}"
KEY_DIR="${KEY_DIR:-./demo-keys}"
SENDER_NAME="${SENDER_NAME:-demo-sender}"
RECIPIENT_NAME="${RECIPIENT_NAME:-demo-recipient}"
FUND_AMOUNT="${FUND_AMOUNT:-1000000000}"
PAY_AMOUNT="${PAY_AMOUNT:-250000000}"
FEE_LIMIT="${FEE_LIMIT:-2000}"
HANDLE_VALUE="${DEMO_HANDLE:-@demo.ipn}"
FILE_TEXT="${FILE_TEXT:-IPPAN demo artifact}"
FILE_PATH="${FILE_PATH:-./demo-file.txt}"
FILE_TAGS_JSON=${FILE_TAGS_JSON:-'["demo","ippan"]'}
POLL_SECONDS="${POLL_SECONDS:-60}"
POLL_INTERVAL="${POLL_INTERVAL:-2}"

log() {
  echo "[$(date +%H:%M:%S)] $*"
}

pretty_json() {
  if command -v jq >/dev/null 2>&1; then
    jq .
  else
    cat
  fi
}

read_hex_file() {
  tr -d '\r\n' <"$1" | tr '[:upper:]' '[:lower:]'
}

ensure_node() {
  log "Checking node health at ${RPC_URL}/health"
  local waited=0
  while ! curl -sf "${RPC_URL}/health" >/dev/null 2>&1; do
    if (( waited >= POLL_SECONDS )); then
      echo "Node at ${RPC_URL} is not responding" >&2
      exit 1
    fi
    sleep "${POLL_INTERVAL}"
    waited=$((waited + POLL_INTERVAL))
  done
  log "Node responded"
}

ensure_keys() {
  mkdir -p "${KEY_DIR}"
  if [[ ! -f "${KEY_DIR}/${SENDER_NAME}_private.key" ]]; then
    log "Generating sender key pair (${SENDER_NAME})"
    cargo run -p keygen -- generate --output "${KEY_DIR}" --name "${SENDER_NAME}" >/dev/null
  fi
  if [[ ! -f "${KEY_DIR}/${RECIPIENT_NAME}_private.key" ]]; then
    log "Generating recipient key pair (${RECIPIENT_NAME})"
    cargo run -p keygen -- generate --output "${KEY_DIR}" --name "${RECIPIENT_NAME}" >/dev/null
  fi
}

load_identities() {
  ensure_keys
  SENDER_HEX=$(read_hex_file "${KEY_DIR}/${SENDER_NAME}_public.key")
  RECEIVER_HEX=$(read_hex_file "${KEY_DIR}/${RECIPIENT_NAME}_public.key")
  SENDER_ADDR="i${SENDER_HEX}"
  RECEIVER_ADDR="i${RECEIVER_HEX}"
  SENDER_KEY=$(read_hex_file "${KEY_DIR}/${SENDER_NAME}_private.key")
}

fund_sender() {
  log "Funding ${SENDER_ADDR} via /dev/fund"
  curl -sSf -H "Content-Type: application/json" \
    -d "{\"address\":\"${SENDER_ADDR}\",\"amount\":${FUND_AMOUNT},\"nonce\":0}" \
    "${RPC_URL}/dev/fund" | pretty_json
}

register_handle() {
  log "Registering handle ${HANDLE_VALUE}"
  curl -sSf -X POST "${RPC_URL}/handle/register" \
    -H "Content-Type: application/json" \
    -d @- <<JSON | pretty_json
{
  "handle": "${HANDLE_VALUE}",
  "owner": "${SENDER_ADDR}",
  "metadata": {"purpose": "e2e demo"},
  "fee": "${FEE_LIMIT}",
  "signing_key": "${SENDER_KEY}"
}
JSON
  sleep 2
  local encoded_handle
  encoded_handle=$(python - "$HANDLE_VALUE" <<'PY'
import sys, urllib.parse
print(urllib.parse.quote(sys.argv[1], safe=''))
PY
  )
  log "Lookup for ${HANDLE_VALUE}"
  curl -sSf "${RPC_URL}/handle/${encoded_handle}" | pretty_json
}

send_payment() {
  log "Sending payment ${PAY_AMOUNT} from ${SENDER_ADDR} -> ${RECEIVER_ADDR}"
  local response
  response=$(curl -sSf -X POST "${RPC_URL}/tx/payment" \
    -H "Content-Type: application/json" \
    -d @- <<JSON)
{
  "from": "${SENDER_ADDR}",
  "to": "${RECEIVER_ADDR}",
  "amount": "${PAY_AMOUNT}",
  "fee": "${FEE_LIMIT}",
  "memo": "demo payment",
  "signing_key": "${SENDER_KEY}"
}
JSON
  echo "${response}" | pretty_json
  TX_HASH=$(echo "${response}" | (jq -r '.tx_hash' 2>/dev/null || python -c 'import json,sys; print(json.load(sys.stdin)["tx_hash"])'))
  log "tx_hash=${TX_HASH}"
}

print_history() {
  for addr in "${SENDER_HEX}" "${RECEIVER_HEX}"; do
    log "Payments for ${addr}"
    curl -sSf "${RPC_URL}/account/${addr}/payments?limit=5" | pretty_json
  done
}

prepare_file() {
  echo "${FILE_TEXT}" > "${FILE_PATH}"
  if command -v sha256sum >/dev/null 2>&1; then
    CONTENT_HASH=$(sha256sum "${FILE_PATH}" | cut -d ' ' -f1)
  else
    CONTENT_HASH=$(shasum -a 256 "${FILE_PATH}" | awk '{print $1}')
  fi
  if stat -c%s / >/dev/null 2>&1; then
    FILE_SIZE=$(stat -c%s "${FILE_PATH}")
  else
    FILE_SIZE=$(stat -f%z "${FILE_PATH}")
  fi
}

publish_file() {
  prepare_file
  log "Publishing file descriptor (${FILE_PATH})"
  local response
  response=$(curl -sSf -X POST "${RPC_URL}/files/publish" \
    -H "Content-Type: application/json" \
    -d @- <<JSON)
{
  "owner": "${SENDER_ADDR}",
  "content_hash": "${CONTENT_HASH}",
  "size_bytes": ${FILE_SIZE},
  "mime_type": "text/plain",
  "tags": ${FILE_TAGS_JSON}
}
JSON
  echo "${response}" | pretty_json
  FILE_ID=$(echo "${response}" | (jq -r '.id' 2>/dev/null || python -c 'import json,sys; print(json.load(sys.stdin)["id"])'))
  log "File ID: ${FILE_ID}"
  curl -sSf "${RPC_URL}/files/${FILE_ID}" | pretty_json
}

show_ai_status() {
  log "AI status"
  curl -sSf "${RPC_URL}/ai/status" | pretty_json
  log "DHT mode env (local shell): file=${IPPAN_FILE_DHT_MODE:-stub}, handle=${IPPAN_HANDLE_DHT_MODE:-stub}"
}

main() {
  ensure_node
  load_identities
  fund_sender
  register_handle
  send_payment
  print_history
  publish_file
  show_ai_status
  log "Demo complete"
}

main "$@"
