#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd)
UI_DIR="${REPO_ROOT}/apps/unified-ui"
DEFAULT_DIST_DIR="${UI_DIR}/dist"
TARGET_DIST_DIR="${UNIFIED_UI_DIST_DIR:-${DEFAULT_DIST_DIR}}"

log() {
  printf '[refresh-ui-assets] %s\n' "$*"
}

die() {
  printf '[refresh-ui-assets] ERROR: %s\n' "$*" >&2
  exit 1
}

command -v npm >/dev/null 2>&1 || die "npm is required but was not found in PATH"

[ -d "${UI_DIR}" ] || die "Unified UI directory not found at ${UI_DIR}"

log "Installing frontend dependencies (npm ci)"
(
  cd "${UI_DIR}"
  export NODE_ENV=production
  npm ci
  log "Building production assets (npm run build)"
  npm run build
)

if [ "${TARGET_DIST_DIR}" != "${DEFAULT_DIST_DIR}" ]; then
  log "Preparing custom dist directory at ${TARGET_DIST_DIR}"
  TARGET_PARENT=$(dirname -- "${TARGET_DIST_DIR}")
  mkdir -p "${TARGET_PARENT}"
  rm -rf "${TARGET_DIST_DIR}"
  cp -R "${DEFAULT_DIST_DIR}" "${TARGET_DIST_DIR}"
fi

log "Unified UI assets refreshed at ${TARGET_DIST_DIR}"
