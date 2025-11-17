#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
CONFIG_DIR="${ROOT_DIR}/localnet"
CARGO_BIN="${CARGO_BIN:-cargo}"
LOG_LEVEL="${RUST_LOG:-info}"

NODE1_DROP_OUTBOUND_PROB="${NODE1_DROP_OUTBOUND_PROB:-0}"
NODE1_DROP_INBOUND_PROB="${NODE1_DROP_INBOUND_PROB:-0}"
NODE1_LAT_MIN="${NODE1_LAT_MIN:-0}"
NODE1_LAT_MAX="${NODE1_LAT_MAX:-0}"

NODE2_DROP_OUTBOUND_PROB="${NODE2_DROP_OUTBOUND_PROB:-150}"
NODE2_DROP_INBOUND_PROB="${NODE2_DROP_INBOUND_PROB:-150}"
NODE2_LAT_MIN="${NODE2_LAT_MIN:-25}"
NODE2_LAT_MAX="${NODE2_LAT_MAX:-150}"

NODE3_DROP_OUTBOUND_PROB="${NODE3_DROP_OUTBOUND_PROB:-400}"
NODE3_DROP_INBOUND_PROB="${NODE3_DROP_INBOUND_PROB:-500}"
NODE3_LAT_MIN="${NODE3_LAT_MIN:-60}"
NODE3_LAT_MAX="${NODE3_LAT_MAX:-350}"

log() {
  echo "[localnet-chaos] $*" >&2
}

start_node() {
  local name="$1"
  shift || true
  local config="${CONFIG_DIR}/${name}.toml"
  local log_file="${CONFIG_DIR}/${name}.chaos.log"
  local pid_file="${CONFIG_DIR}/${name}.pid"

  if [[ ! -f "${config}" ]]; then
    log "Missing config ${config}"
    exit 1
  fi

  if [[ -f "${pid_file}" ]]; then
    local existing_pid
    existing_pid="$(cat "${pid_file}")"
    if [[ -n "${existing_pid}" && -e "/proc/${existing_pid}" ]]; then
      log "${name} already running with PID ${existing_pid}; skipping"
      return
    fi
  fi

  local -a env_vars=("RUST_LOG=${LOG_LEVEL}")
  for entry in "$@"; do
    env_vars+=("${entry}")
  done

  log "Starting ${name} with chaos env: ${env_vars[*]}"
  (
    cd "${ROOT_DIR}"
    env "${env_vars[@]}" "${CARGO_BIN}" run -p ippan-node -- --config "${config}"
  ) >"${log_file}" 2>&1 &
  echo $! >"${pid_file}"
  log "${name} PID $(cat "${pid_file}") (logs -> ${log_file})"
}

mkdir -p "${CONFIG_DIR}"

NODE1_ENV=(
  "IPPAN_CHAOS_DROP_OUTBOUND_PROB=${NODE1_DROP_OUTBOUND_PROB}"
  "IPPAN_CHAOS_DROP_INBOUND_PROB=${NODE1_DROP_INBOUND_PROB}"
  "IPPAN_CHAOS_EXTRA_LATENCY_MS_MIN=${NODE1_LAT_MIN}"
  "IPPAN_CHAOS_EXTRA_LATENCY_MS_MAX=${NODE1_LAT_MAX}"
)
NODE2_ENV=(
  "IPPAN_CHAOS_DROP_OUTBOUND_PROB=${NODE2_DROP_OUTBOUND_PROB}"
  "IPPAN_CHAOS_DROP_INBOUND_PROB=${NODE2_DROP_INBOUND_PROB}"
  "IPPAN_CHAOS_EXTRA_LATENCY_MS_MIN=${NODE2_LAT_MIN}"
  "IPPAN_CHAOS_EXTRA_LATENCY_MS_MAX=${NODE2_LAT_MAX}"
)
NODE3_ENV=(
  "IPPAN_CHAOS_DROP_OUTBOUND_PROB=${NODE3_DROP_OUTBOUND_PROB}"
  "IPPAN_CHAOS_DROP_INBOUND_PROB=${NODE3_DROP_INBOUND_PROB}"
  "IPPAN_CHAOS_EXTRA_LATENCY_MS_MIN=${NODE3_LAT_MIN}"
  "IPPAN_CHAOS_EXTRA_LATENCY_MS_MAX=${NODE3_LAT_MAX}"
)

start_node node1 "${NODE1_ENV[@]}"
start_node node2 "${NODE2_ENV[@]}"
start_node node3 "${NODE3_ENV[@]}"

log "All nodes started with chaos profiles. Use scripts/localnet_churn_scenario.sh or scripts/localnet_chaos_scenario.sh next."
