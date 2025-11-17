#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
CONFIG_DIR="${ROOT_DIR}/localnet"
CARGO_BIN="${CARGO_BIN:-cargo}"
LOG_LEVEL="${RUST_LOG:-info}"

start_node() {
  local name="$1"
  local config="${CONFIG_DIR}/${name}.toml"
  local log_file="${CONFIG_DIR}/${name}.log"
  local pid_file="${CONFIG_DIR}/${name}.pid"

  if [[ ! -f "${config}" ]]; then
    echo "[localnet] Missing config ${config}" >&2
    exit 1
  fi

  if [[ -f "${pid_file}" ]]; then
    local pid
    pid="$(cat "${pid_file}")"
    if [[ -n "${pid}" && -e "/proc/${pid}" ]]; then
      echo "[localnet] ${name} already running with PID ${pid}" >&2
      return
    fi
  fi

  echo "[localnet] Starting ${name} using ${config}" >&2
  (
    cd "${ROOT_DIR}"
    RUST_LOG="${LOG_LEVEL}" "${CARGO_BIN}" run -p ippan-node -- --config "${config}"
  ) >"${log_file}" 2>&1 &
  echo $! >"${pid_file}"
  echo "[localnet] ${name} PID $(cat "${pid_file}") (logs -> ${log_file})" >&2
}

mkdir -p "${CONFIG_DIR}"

start_node node1
start_node node2
start_node node3

echo "[localnet] All nodes started. Use scripts/localnet_stop.sh to stop them." >&2
