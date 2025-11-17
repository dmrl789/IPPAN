#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT_DIR}"

NODE1_RPC="${NODE1_RPC:-http://127.0.0.1:3111}"
NODE2_RPC="${NODE2_RPC:-http://127.0.0.1:3112}"
NODE3_RPC="${NODE3_RPC:-http://127.0.0.1:3113}"
NODE2_PID_FILE="${ROOT_DIR}/localnet/node2.pid"
KEY_DIR="${KEY_DIR:-${ROOT_DIR}/localnet/keys}"
ACCOUNT_A_NAME="${ACCOUNT_A_NAME:-churn-a}"
ACCOUNT_B_NAME="${ACCOUNT_B_NAME:-churn-b}"
FUND_AMOUNT="${FUND_AMOUNT:-1500000000}"
PAY_AMOUNT="${PAY_AMOUNT:-400000000}"
FEE_LIMIT="${FEE_LIMIT:-3500}"
HANDLE_VALUE="${HANDLE_VALUE:-@churn-$(date +%s).ipn}"
FILE_CONTENT="${FILE_CONTENT:-IPPAN churn file payload v1}"
FILE_MIME="${FILE_MIME:-text/plain}"
FILE_TAGS_JSON="${FILE_TAGS_JSON:-[\"churn\",\"resilience\"]}"
POLL_INTERVAL="${POLL_INTERVAL:-3}"
POLL_MAX_ATTEMPTS="${POLL_MAX_ATTEMPTS:-40}"
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

read_hex() {
  tr -d '\r\n' <"$1" | tr '[:upper:]' '[:lower:]'
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

load_accounts() {
  ensure_keys
  ACCOUNT_A_PRIV_HEX="$(read_hex "${KEY_DIR}/${ACCOUNT_A_NAME}_private.key")"
  ACCOUNT_A_PUB_HEX="$(read_hex "${KEY_DIR}/${ACCOUNT_A_NAME}_public.key")"
  ACCOUNT_A_ADDR="i${ACCOUNT_A_PUB_HEX}"

  ACCOUNT_B_PRIV_HEX="$(read_hex "${KEY_DIR}/${ACCOUNT_B_NAME}_private.key")"
  ACCOUNT_B_PUB_HEX="$(read_hex "${KEY_DIR}/${ACCOUNT_B_NAME}_public.key")"
  ACCOUNT_B_ADDR="i${ACCOUNT_B_PUB_HEX}"
}

ensure_node_ready() {
  local rpc="$1"
  local attempts=0
  until curl -sf "${rpc}/health" >/dev/null 2>&1; do
    (( attempts++ ))
    if (( attempts > POLL_MAX_ATTEMPTS )); then
      echo "Node ${rpc} did not respond" >&2
      exit 1
    fi
    sleep "${POLL_INTERVAL}"
  done
}

wait_for_node_down() {
  local rpc="$1"
  local attempts=0
  while curl -sf "${rpc}/health" >/dev/null 2>&1; do
    (( attempts++ ))
    if (( attempts > POLL_MAX_ATTEMPTS )); then
      echo "Node ${rpc} still online" >&2
      exit 1
    fi
    sleep "${POLL_INTERVAL}"
  done
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

send_payment() {
  local rpc="$1"
  local from_addr="$2"
  local to_addr="$3"
  local priv="$4"
  local memo="$5"
  local payload
  payload=$(cat <<JSON
{
  "from": "${from_addr}",
  "to": "${to_addr}",
  "amount": ${PAY_AMOUNT},
  "fee": ${FEE_LIMIT},
  "memo": "${memo}",
  "signing_key": "${priv}"
}
JSON
)
  curl -sSf -H 'Content-Type: application/json' -d "${payload}" "${rpc}/tx/payment"
}

register_handle() {
  local rpc="$1"
  local owner="$2"
  local priv="$3"
  local payload
  payload=$(cat <<JSON
{
  "handle": "${HANDLE_VALUE}",
  "owner": "${owner}",
  "metadata": {"purpose": "churn", "rpc": "${rpc}"},
  "fee": ${FEE_LIMIT},
  "signing_key": "${priv}"
}
JSON
)
  curl -sSf -H 'Content-Type: application/json' -d "${payload}" "${rpc}/handle/register"
}

publish_file() {
  local rpc="$1"
  local owner="$2"
  local payload
  local size_bytes
  size_bytes="$(printf '%s' "${FILE_CONTENT}" | wc -c | tr -d '[:space:]')"
  local content_hash
  content_hash=$(FILE_CONTENT_VALUE="${FILE_CONTENT}" python - <<'PY')
import hashlib
import os
print(hashlib.sha256(os.environ["FILE_CONTENT_VALUE"].encode()).hexdigest())
PY
  payload=$(cat <<JSON
{
  "owner": "${owner}",
  "content_hash": "${content_hash}",
  "size_bytes": ${size_bytes},
  "mime_type": "${FILE_MIME}",
  "tags": ${FILE_TAGS_JSON}
}
JSON
)
  curl -sSf -H 'Content-Type: application/json' -d "${payload}" "${rpc}/files/publish"
}

poll_until_visible() {
  local description="$1"
  local cmd="$2"
  local attempts=0
  while (( attempts < POLL_MAX_ATTEMPTS )); do
    if output=$(eval "${cmd}" 2>/dev/null); then
      if [[ -n "${output}" && "${output}" != "[]" ]]; then
        log "${description} visible"
        maybe_jq "${output}"
        return 0
      fi
    fi
    (( attempts++ ))
    sleep "${POLL_INTERVAL}"
  done
  echo "${description} not visible after ${POLL_MAX_ATTEMPTS} attempts" >&2
  return 1
}

log "Ensuring nodes are online"
ensure_node_ready "${NODE1_RPC}"
ensure_node_ready "${NODE2_RPC}"
ensure_node_ready "${NODE3_RPC}"

load_accounts
log "Account A: ${ACCOUNT_A_ADDR}"
log "Account B: ${ACCOUNT_B_ADDR}"

log "Funding accounts on all nodes"
for rpc in "${NODE1_RPC}" "${NODE2_RPC}" "${NODE3_RPC}"; do
  fund_account "${rpc}" "${ACCOUNT_A_ADDR}" "${FUND_AMOUNT}"
  fund_account "${rpc}" "${ACCOUNT_B_ADDR}" "${FUND_AMOUNT}"
  log "Funded via ${rpc}"
done

log "Stopping node2 (best-effort)."
if [[ -f "${NODE2_PID_FILE}" ]]; then
  NODE2_PID="$(cat "${NODE2_PID_FILE}")"
  if [[ -n "${NODE2_PID}" && -e "/proc/${NODE2_PID}" ]]; then
    log "Sending SIGTERM to node2 PID ${NODE2_PID}"
    kill "${NODE2_PID}" || true
  fi
fi
log "If node2 is still running elsewhere, stop it now then press enter to continue."
read -r -p "Press enter once node2 is offline..."
wait_for_node_down "${NODE2_RPC}"
log "Node2 confirmed offline"

log "Submitting transactions while node2 is down"
payment_a=$(send_payment "${NODE1_RPC}" "${ACCOUNT_A_ADDR}" "${ACCOUNT_B_ADDR}" "${ACCOUNT_A_PRIV_HEX}" "node1->node3")
maybe_jq "${payment_a}"
payment_b=$(send_payment "${NODE3_RPC}" "${ACCOUNT_B_ADDR}" "${ACCOUNT_A_ADDR}" "${ACCOUNT_B_PRIV_HEX}" "node3->node1")
maybe_jq "${payment_b}"
handle_response=$(register_handle "${NODE3_RPC}" "${ACCOUNT_B_ADDR}" "${ACCOUNT_B_PRIV_HEX}")
maybe_jq "${handle_response}"
file_response=$(publish_file "${NODE3_RPC}" "${ACCOUNT_A_ADDR}")
maybe_jq "${file_response}"
FILE_ID="$(python - <<'PY' <<<"${file_response}"
import json,sys
print(json.load(sys.stdin)["id"])
PY
)"

log "Restart node2 using scripts/localnet_chaos_start.sh (or your process manager) with the same data dir."
read -r -p "Press enter once node2 has been restarted..."
ensure_node_ready "${NODE2_RPC}"
log "Node2 is back online, verifying state convergence"

poll_until_visible "Payments for ${ACCOUNT_A_PUB_HEX} on node2" \
  "curl -sf '${NODE2_RPC}/account/${ACCOUNT_A_PUB_HEX}/payments?limit=5'"
poll_until_visible "Payments for ${ACCOUNT_B_PUB_HEX} on node2" \
  "curl -sf '${NODE2_RPC}/account/${ACCOUNT_B_PUB_HEX}/payments?limit=5'"
poll_until_visible "Handle ${HANDLE_VALUE} on node2" \
  "curl -sf '${NODE2_RPC}/handle/${HANDLE_VALUE}'"
poll_until_visible "File ${FILE_ID} on node2" \
  "curl -sf '${NODE2_RPC}/files/${FILE_ID}'"

log "Churn scenario complete — node2 caught up after restart"
