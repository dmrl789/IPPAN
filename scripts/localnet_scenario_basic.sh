#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT_DIR}"

NODE1_RPC="${NODE1_RPC:-http://127.0.0.1:3111}"
NODE2_RPC="${NODE2_RPC:-http://127.0.0.1:3112}"
NODE3_RPC="${NODE3_RPC:-http://127.0.0.1:3113}"
KEY_DIR="${KEY_DIR:-${ROOT_DIR}/localnet/keys}"
ACCOUNT_A_NAME="${ACCOUNT_A_NAME:-localnet-a}"
ACCOUNT_B_NAME="${ACCOUNT_B_NAME:-localnet-b}"
FUND_AMOUNT="${FUND_AMOUNT:-1000000000}"
PAY_AMOUNT="${PAY_AMOUNT:-250000000}"
FEE_LIMIT="${FEE_LIMIT:-2000}"
HANDLE_VALUE="${HANDLE_VALUE:-@demo1.ipn}"
POLL_INTERVAL="${POLL_INTERVAL:-2}"
POLL_MAX_ATTEMPTS="${POLL_MAX_ATTEMPTS:-40}"
FILE_CONTENT="${FILE_CONTENT:-IPPAN localnet demo file v1}"
FILE_MIME="${FILE_MIME:-text/plain}"
FILE_TAGS_JSON="${FILE_TAGS_JSON:-[\"localnet\",\"demo\"]}"
CARGO_BIN="${CARGO_BIN:-cargo}"

log() {
  echo "[$(date +%H:%M:%S)] $*"
}

maybe_jq() {
  if command -v jq >/dev/null 2>&1; then
    echo "$1" | jq
  else
    echo "$1"
  fi
}

extract_json_field() {
  local json="$1"
  local field="$2"
  python - "$field" <<'PY' <<<"${json}"
import json
import sys
payload = json.load(sys.stdin)
field = sys.argv[1]
value = payload
for part in field.split('.'):
    if isinstance(value, dict) and part in value:
        value = value[part]
    else:
        raise SystemExit(f"missing field: {field}")
if isinstance(value, (dict, list)):
    import json as _json
    print(_json.dumps(value))
else:
    print(value)
PY
}

ensure_node_ready() {
  local rpc="$1"
  log "Checking ${rpc}/health"
  local attempts=0
  until curl -sf "${rpc}/health" >/dev/null 2>&1; do
    (( attempts++ ))
    if (( attempts > POLL_MAX_ATTEMPTS )); then
      echo "Node ${rpc} is not responding" >&2
      exit 1
    fi
    sleep "${POLL_INTERVAL}"
  done
}

ensure_keys() {
  mkdir -p "${KEY_DIR}"
  if [[ ! -f "${KEY_DIR}/${ACCOUNT_A_NAME}_private.key" ]]; then
    log "Generating keypair ${ACCOUNT_A_NAME}"
    "${CARGO_BIN}" run -p keygen -- generate --output "${KEY_DIR}" --name "${ACCOUNT_A_NAME}" >/dev/null
  fi
  if [[ ! -f "${KEY_DIR}/${ACCOUNT_B_NAME}_private.key" ]]; then
    log "Generating keypair ${ACCOUNT_B_NAME}"
    "${CARGO_BIN}" run -p keygen -- generate --output "${KEY_DIR}" --name "${ACCOUNT_B_NAME}" >/dev/null
  fi
}

read_hex() {
  tr -d '\r\n' <"$1" | tr '[:upper:]' '[:lower:]'
}

load_accounts() {
  ensure_keys
  ACCOUNT_A_PRIV_HEX="$(read_hex "${KEY_DIR}/${ACCOUNT_A_NAME}_private.key")"
  ACCOUNT_A_PUB_HEX="$(read_hex "${KEY_DIR}/${ACCOUNT_A_NAME}_public.key")"
  ACCOUNT_A_ADDR="i${ACCOUNT_A_PUB_HEX}"

  ACCOUNT_B_PRIV_HEX="$(read_hex "${KEY_DIR}/${ACCOUNT_B_NAME}_private.key")"
  ACCOUNT_B_PUB_HEX="$(read_hex "${KEY_DIR}/${ACCOUNT_B_NAME}_public.key")"
  ACCOUNT_B_ADDR="i${ACCOUNT_B_PUB_HEX}"
}

fund_account() {
  local rpc="$1"
  local address="$2"
  local amount="$3"
  local payload
  payload=$(cat <<JSON
{"address":"${address}","amount":${amount},"nonce":0}
JSON
)
  curl -sSf -H 'Content-Type: application/json' -d "${payload}" "${rpc}/dev/fund" >/dev/null
}

wait_for_handle() {
  local attempts=0
  while (( attempts < POLL_MAX_ATTEMPTS )); do
    if response=$(curl -sf "${NODE3_RPC}/handle/${HANDLE_VALUE}"); then
      log "Handle ${HANDLE_VALUE} is visible on node3"
      maybe_jq "${response}"
      return
    fi
    sleep "${POLL_INTERVAL}"
    (( attempts++ ))
  done
  echo "Handle ${HANDLE_VALUE} not visible on ${NODE3_RPC}" >&2
  exit 1
}

wait_for_payments() {
  local address_hex="$1"
  local attempts=0
  while (( attempts < POLL_MAX_ATTEMPTS )); do
    if response=$(curl -sf "${NODE3_RPC}/account/${address_hex}/payments?limit=5"); then
      if [[ "${response}" != "[]" ]]; then
        log "Payments for ${address_hex} visible on node3"
        maybe_jq "${response}"
        return
      fi
    fi
    sleep "${POLL_INTERVAL}"
    (( attempts++ ))
  done
  echo "Payments for ${address_hex} not visible on ${NODE3_RPC}" >&2
  exit 1
}

wait_for_file() {
  local file_id="$1"
  local attempts=0
  while (( attempts < POLL_MAX_ATTEMPTS )); do
    if response=$(curl -sf "${NODE3_RPC}/files/${file_id}"); then
      log "File ${file_id} replicated to node3"
      maybe_jq "${response}"
      return
    fi
    sleep "${POLL_INTERVAL}"
    (( attempts++ ))
  done
  echo "File ${file_id} not visible on ${NODE3_RPC}" >&2
  exit 1
}

log "Ensuring nodes are online"
ensure_node_ready "${NODE1_RPC}"
ensure_node_ready "${NODE2_RPC}"
ensure_node_ready "${NODE3_RPC}"

load_accounts
log "Account A: ${ACCOUNT_A_ADDR}"
log "Account B: ${ACCOUNT_B_ADDR}"

log "Funding demo accounts via /dev/fund on all nodes"
for rpc in "${NODE1_RPC}" "${NODE2_RPC}" "${NODE3_RPC}"; do
  fund_account "${rpc}" "${ACCOUNT_A_ADDR}" "${FUND_AMOUNT}"
  fund_account "${rpc}" "${ACCOUNT_B_ADDR}" "${FUND_AMOUNT}"
  log "Funded accounts on ${rpc}"
done

global_summary() {
  echo
  log "Scenario complete"
  log "Handle transaction: ${HANDLE_TX_HASH:-unknown}"
  log "Payment transaction: ${PAYMENT_TX_HASH:-unknown}"
  log "File ID: ${PUBLISHED_FILE_ID:-unknown}"
}

log "Registering ${HANDLE_VALUE} on node1"
handle_payload=$(cat <<JSON
{
  "handle": "${HANDLE_VALUE}",
  "owner": "${ACCOUNT_A_ADDR}",
  "metadata": {"purpose": "localnet-demo", "node": "A"},
  "fee": ${FEE_LIMIT},
  "signing_key": "${ACCOUNT_A_PRIV_HEX}"
}
JSON
)
handle_response=$(curl -sSf -H 'Content-Type: application/json' -d "${handle_payload}" "${NODE1_RPC}/handle/register")
maybe_jq "${handle_response}"
HANDLE_TX_HASH="$(extract_json_field "${handle_response}" "tx_hash")"
log "Handle tx hash: ${HANDLE_TX_HASH}"
wait_for_handle

log "Sending payment from Account A to Account B"
payment_payload=$(cat <<JSON
{
  "from": "${ACCOUNT_A_ADDR}",
  "to": "${ACCOUNT_B_ADDR}",
  "amount": ${PAY_AMOUNT},
  "fee": ${FEE_LIMIT},
  "memo": "localnet demo",
  "signing_key": "${ACCOUNT_A_PRIV_HEX}"
}
JSON
)
payment_response=$(curl -sSf -H 'Content-Type: application/json' -d "${payment_payload}" "${NODE1_RPC}/tx/payment")
maybe_jq "${payment_response}"
PAYMENT_TX_HASH="$(extract_json_field "${payment_response}" "tx_hash")"
log "Payment tx hash: ${PAYMENT_TX_HASH}"
wait_for_payments "${ACCOUNT_B_PUB_HEX}"

FILE_SIZE_BYTES="$(printf '%s' "${FILE_CONTENT}" | wc -c | tr -d '[:space:]')"
FILE_HASH=$(FILE_CONTENT_VALUE="${FILE_CONTENT}" python - <<'PY')
import hashlib
import os
print(hashlib.sha256(os.environ["FILE_CONTENT_VALUE"].encode()).hexdigest())
PY

log "Publishing file metadata on node2"
file_payload=$(cat <<JSON
{
  "owner": "${ACCOUNT_B_ADDR}",
  "content_hash": "${FILE_HASH}",
  "size_bytes": ${FILE_SIZE_BYTES},
  "mime_type": "${FILE_MIME}",
  "tags": ${FILE_TAGS_JSON}
}
JSON
)
file_response=$(curl -sSf -H 'Content-Type: application/json' -d "${file_payload}" "${NODE2_RPC}/files/publish")
maybe_jq "${file_response}"
PUBLISHED_FILE_ID="$(extract_json_field "${file_response}" "id")"
log "Published file id: ${PUBLISHED_FILE_ID}"
wait_for_file "${PUBLISHED_FILE_ID}"

global_summary
